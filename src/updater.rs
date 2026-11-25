//! 应用更新：检查新版本、下载，并在退出时把应用替换成新版本。
//!
//! 更新从哪来、允不允许装，全由打包时写进资源目录的 `nsh/assets/nsh-updater.json`
//! 决定（也就是 `nsh.json` 的 `updater`）：代码里不出现任何地址，清单的签名、应用 id、
//! 版本只能变新、只允许 HTTPS、压缩包的 size 与 SHA-256、新包的 id 与可执行文件，
//! 全部由 `nsh-updater` 校验。
//!
//! 这一层只做三件事：找到那份配置、在自己的线程里用系统 TLS 栈取字节、把结果交给引擎
//! 线程。替换分两步：下载完成后 [`Installation::stage`]（解包并校验新应用，摆在应用
//! 旁边），退出应用时 [`Staged::finish`] 把它换上——与 `installOnQuit` 一样，不打断正在
//! 跑的提醒，也不会替用户重启。

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use nsh_updater::install::{Installation, Staged};
use nsh_updater::{Config, Download};

use crate::APP_ID;

/// 清单与签名的大小上限：一份 JSON、一段 base64
const MANIFEST_LIMIT: u64 = 2 * 1024 * 1024;
/// 压缩包的大小上限：清单给了 size 时以它为准，这里只是兜底
const ARCHIVE_LIMIT: u64 = 512 * 1024 * 1024;
/// 建连接（DNS / TCP / TLS / 代理握手）的最长时间
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// 等响应头的最长时间
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
/// 本地 DNS 解析的最长时间
const RESOLVE_TIMEOUT: Duration = Duration::from_secs(10);
/// 两次收到数据之间的最长空档：12 MB 的安装包在慢代理上要下几分钟，
/// 不能用「整次请求 60 秒」这种总时限把还在动的下载掐掉；只要数据还在流，
/// 慢也不算失败，彻底不动 30 秒才判超时。
const READ_TIMEOUT: Duration = Duration::from_secs(30);
/// 进度上报的最小跨档（百分比）：避免几十 MB 的下载把消息队列刷满
const PROGRESS_STEP: u64 = 2;

/// 资源目录相对应用根目录的两处可能位置：Windows / Linux 在 `<应用>/nsh/assets`，
/// macOS 在 `<应用>.app/Contents/Resources/nsh/assets`
const CONFIG_PATHS: [&str; 2] = [
    "nsh/assets/nsh-updater.json",
    "Contents/Resources/nsh/assets/nsh-updater.json",
];

/// 可以安装的新版本（清单里挑出来、给这台机器的那一条）。
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub version: String,
    /// 更新说明：清单里的纯文本，可能为空
    pub notes: String,
    /// 手动更新时打开的地址（压缩包本身，或商店 / 下载页）
    pub url: String,
    pub size: Option<u64>,
    /// 压缩包的 SHA-256：清单里没有它说明 `url` 是下载页，不是能自己装的包
    pub sha256: String,
    /// 能不能直接替换：装在只读目录（Program Files、系统包）时为 false，
    /// 这时只给用户一个下载页，不假装能自己装
    pub can_install: bool,
}

impl Found {
    /// 更新说明里第一句有用的话，压成一行做提示（完整说明留给发布页）
    pub fn summary(&self) -> String {
        let line = self
            .notes
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with('#'))
            .unwrap_or_default()
            .trim_start_matches(['-', '*', ' '])
            .replace("**", "");
        match line.char_indices().nth(88) {
            Some((cut, _)) => format!("{}…", &line[..cut]),
            None => line,
        }
    }

    /// 压缩包大小，用于「约 18.5 MB」
    pub fn size_label(&self) -> Option<String> {
        self.size.map(bytes_label)
    }
}

/// 字节数写成「18.5 MB」这样的文字
pub fn bytes_label(bytes: u64) -> String {
    let megabytes = bytes as f64 / (1024.0 * 1024.0);
    if megabytes >= 1.0 {
        format!("{megabytes:.1} MB")
    } else {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    }
}

/// 界面上看到的更新状态。同一时间只会有一个检查 / 下载在跑。
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    /// 没有打包配置（`cargo run` / `nsh run` / 测试）：没有可替换的安装，不检查
    Unavailable,
    /// 还没查过
    Idle,
    Checking,
    /// 已经是最新
    UpToDate,
    /// 有新版本，等用户点下载
    Available(Found),
    Downloading {
        version: String,
        received: u64,
        total: Option<u64>,
    },
    /// 已下载并校验，退出应用时替换
    Ready {
        version: String,
    },
    /// 用户要求现在替换：正在换文件，随后重启
    Installing,
    Failed(String),
}

impl Status {
    /// 正在跑网络 / 解包 / 换文件：这期间不再发起新的动作
    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            Status::Checking | Status::Downloading { .. } | Status::Installing
        )
    }

    /// 是否有值得在主界面角标提示的更新状态（发现新版本，或已下载待重启）。
    pub fn has_notice(&self) -> bool {
        matches!(self, Status::Available(_) | Status::Ready { .. })
    }

    /// 下载进度（0.0–1.0）；不知道总大小时给 0（界面只显示已下载的字节数）
    pub fn progress(&self) -> f32 {
        match self {
            Status::Downloading {
                received, total, ..
            } => match total {
                Some(total) if *total > 0 => (*received as f32 / *total as f32).clamp(0.0, 1.0),
                _ => 0.0,
            },
            _ => 0.0,
        }
    }
}

/// 后台线程交给引擎线程的消息：只有纯数据（信号只在引擎线程写）。
pub enum Event {
    /// 检查结果：`None` 表示已是最新
    Checked(Result<Option<Found>, String>),
    Progress {
        received: u64,
        total: Option<u64>,
    },
    /// 已经解包校验、摆在应用旁边的新版本（退出时替换）
    Staged(Result<(Staged, String), String>),
}

/// 界面这一侧接上的管道：只接一次，返回排空用的一端。
static SENDER: OnceLock<Sender<Event>> = OnceLock::new();

/// 接上管道。应用启动时调一次，接收端由引擎线程按帧排空。
pub fn connect() -> Receiver<Event> {
    let (sender, receiver) = channel();
    let _ = SENDER.set(sender);
    receiver
}

fn send(event: Event) {
    if let Some(sender) = SENDER.get() {
        let _ = sender.send(event);
    }
}

/// 打包时写下的那份配置：字节给 `Installation` 逐字节比对，解析出的配置给检查用。
///
/// 找不到它说明这次不是从安装包运行的（开发运行、测试），也就没有可替换的应用。
pub fn bundled() -> Option<(Vec<u8>, Config)> {
    let bytes = bundled_bytes()?;
    let config = Config::parse(&bytes).ok()?;
    Some((bytes, config))
}

/// 资源的原始字节（`nsh package` 写的，用来确认「这个进程就是这个安装」）。
pub fn bundled_bytes() -> Option<Vec<u8>> {
    // 本地演练 / 测试用：直接指定那份配置，不依赖打包布局
    if let Ok(path) = std::env::var("REMINDER_UPDATER_CONFIG") {
        return std::fs::read(path).ok();
    }
    let executable = std::env::current_exe().ok()?;
    find_config(executable.parent()?)
}

/// 从 `start` 往上找四层：macOS 的 `Contents/MacOS` 要往上三层才到 `.app`。
fn find_config(start: &Path) -> Option<Vec<u8>> {
    let mut directory = start;
    for _ in 0..4 {
        for relative in CONFIG_PATHS {
            if let Ok(bytes) = std::fs::read(directory.join(relative)) {
                return Some(bytes);
            }
        }
        directory = directory.parent()?;
    }
    None
}

/// 开始检查新版本（后台线程；结果以 [`Event::Checked`] 回来）。
pub fn start_check() {
    std::thread::spawn(|| send(Event::Checked(check())));
}

/// 开始下载并校验新版本（后台线程；结果以 [`Event::Staged`] 回来）。
pub fn start_download(found: Found) {
    std::thread::spawn(move || send(Event::Staged(download_and_stage(&found))));
}

/// 退出时把已经摆好的新版本换上去。
///
/// 失败只记日志：此刻界面已经没了，应用保持原样退出，下次启动还是旧版本。
pub fn install_at_quit(staged: Staged) -> Result<(), String> {
    staged.finish().map(|_| ()).map_err(|error| error.to_string())
}

/// 现在就把已经摆好的新版本换上去，调用方随后重启应用。
///
/// 先把程序路径记下来：换完之后这个进程的可执行文件已经不是它了，
/// 重启要找的是用户认识的那个路径（与 `nsh/updater` 的 `installAndRestart` 同一步）。
pub fn install_now(staged: Staged) -> Result<(), String> {
    nsh_core::restart::remember_program();
    staged.finish().map(|_| ()).map_err(|error| error.to_string())
}

/// 清掉上次替换没能删掉的旧文件。
///
/// Windows 上正在运行的 exe 在替换时删不掉，留到下次启动；所以应用一启动就静默跑一次，
/// 不取决于这次有没有联网、用户有没有点检查。
pub fn clean_leftovers() {
    let Some((bytes, _)) = bundled() else {
        return;
    };
    if let Some(installation) = Installation::current(APP_ID, &bytes) {
        installation.remove_leftovers();
    }
}

fn check() -> Result<Option<Found>, String> {
    let (bytes, config) = bundled().ok_or("没有打包的更新配置，开发运行不检查更新")?;
    // 这个进程是不是就来自这个安装：决定了能不能自己替换，还是只能给一张下载页
    let can_install =
        Installation::current(APP_ID, &bytes).is_some_and(|installation| installation.can_install());
    check_config(&config, can_install)
}

/// 用一份配置查新版本：清单 → 签名（配了 publicKey 才有）→ 给这台机器的那个包。
fn check_config(config: &Config, can_install: bool) -> Result<Option<Found>, String> {
    let manifest = get(&config.manifest_url(), MANIFEST_LIMIT)?;
    let signature = match config.signature_url() {
        Some(url) => Some(get(&url, MANIFEST_LIMIT)?),
        None => None,
    };
    let available = config
        .check(&manifest, signature.as_deref(), &nsh_updater::platform())
        .map_err(|error| error.to_string())?;
    Ok(available.and_then(|available| {
        // 没有 SHA-256 的是一张下载页（移动端商店），不是这个应用能自己装的包
        let sha256 = available.sha256?;
        Some(Found {
            version: available.version,
            notes: available.notes.unwrap_or_default(),
            url: available.url,
            size: available.size,
            sha256,
            can_install,
        })
    }))
}

fn download_and_stage(found: &Found) -> Result<(Staged, String), String> {
    let (bytes, _) = bundled().ok_or("没有打包的更新配置，无法自动更新")?;
    let installation =
        Installation::current(APP_ID, &bytes).ok_or("当前不是从安装包运行的，请到下载页手动更新")?;
    if !installation.can_install() {
        return Err("应用目录不可写，请到下载页手动更新".to_owned());
    }

    let archive = archive_path();
    if let Err(error) = download_to(found, &archive) {
        let _ = std::fs::remove_file(&archive);
        return Err(error);
    }
    // 解包并校验新应用：失败现在就能报给用户，而不是等退出时静默失败
    let staged = match installation.stage(&archive) {
        Ok(staged) => staged,
        Err(error) => {
            let _ = std::fs::remove_file(&archive);
            return Err(format!("更新包校验失败：{error}"));
        }
    };
    // 已经解到应用旁边，压缩包不再需要
    let _ = std::fs::remove_file(&archive);
    Ok((staged, found.version.clone()))
}

/// 下载到临时文件，边下边按清单的 size 与 SHA-256 校验。
fn download_to(found: &Found, path: &Path) -> Result<(), String> {
    let mut response = request(&found.url)?;
    let mut download = Download::new(&found.sha256, found.size);
    let mut file = std::fs::File::create(path).map_err(|error| error.to_string())?;
    let limit = found.size.unwrap_or(ARCHIVE_LIMIT).min(ARCHIVE_LIMIT) + 1;
    let mut reader = response.body_mut().with_config().limit(limit).reader();

    let mut buffer = vec![0_u8; 64 * 1024];
    let mut reported: Option<u64> = None;
    loop {
        let read = reader.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        let received = download
            .update(&buffer[..read])
            .map_err(|error| error.to_string())?;
        file.write_all(&buffer[..read])
            .map_err(|error| error.to_string())?;

        // 按百分比跨档上报，界面那一侧只看最后一个值
        let percent = found
            .size
            .filter(|total| *total > 0)
            .map(|total| (received * 100 / total).min(100));
        let report = match (reported, percent) {
            (None, _) => true,
            (Some(last), Some(percent)) => percent >= last + PROGRESS_STEP || percent == 100,
            (Some(_), None) => false,
        };
        if report {
            reported = Some(percent.unwrap_or_default());
            send(Event::Progress {
                received,
                total: found.size,
            });
        }
    }
    drop(file);
    download.finish().map_err(|error| error.to_string())
}

/// 压缩包落在系统临时目录：`stage` 会把内容解到应用旁边，这里只是过路。
fn archive_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "reminder-{}-{}.update",
        env!("CARGO_PKG_VERSION"),
        std::process::id()
    ))
}

fn get(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let mut response = request(url)?;
    response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(|error| error.to_string())
}

/// 一次 GET：系统 TLS 栈（平台证书校验）+ 用户环境里的代理，重定向由 ureq 跟随。
fn request(url: &str) -> Result<ureq::http::Response<ureq::Body>, String> {
    let response = agent()
        .get(url)
        .header("User-Agent", concat!("reminder/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|error| format!("连接失败：{error}"))?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(format!("服务器返回 HTTP {status}"));
    }
    Ok(response)
}

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        let mut builder = ureq::Agent::config_builder()
            // 不给整个请求设总时限：它会把「正在慢慢下载」当成超时。
            // 连接、响应头、单次读各有预算，卡死 30 秒才失败。
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_resolve(Some(RESOLVE_TIMEOUT))
            .timeout_recv_response(Some(RESPONSE_TIMEOUT))
            .timeout_per_read(Some(READ_TIMEOUT))
            // 状态码自己判断，好给用户「服务器返回 HTTP 404」这样的说明
            .http_status_as_error(false)
            .tls_config(
                ureq::tls::TlsConfig::builder()
                    .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                    .build(),
            );
        // 用户环境里的代理（HTTP_PROXY / HTTPS_PROXY / ALL_PROXY）
        if let Some(proxy) = ureq::Proxy::try_from_env() {
            builder = builder.proxy(Some(proxy));
        }
        builder.build().into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(notes: &str) -> Found {
        Found {
            version: "0.3.0".to_owned(),
            notes: notes.to_owned(),
            url: "https://example.com/reminder.zip".to_owned(),
            size: Some(19_400_000),
            sha256: "0".repeat(64),
            can_install: true,
        }
    }

    #[test]
    fn summary_skips_headings_and_list_markers() {
        let notes = "# 更新内容\n\n- 支持退出时自动更新\n- 修复托盘图标";
        assert_eq!(found(notes).summary(), "支持退出时自动更新");
        assert_eq!(found("").summary(), "");
    }

    #[test]
    fn summary_keeps_a_single_short_line() {
        let summary = found(&"a".repeat(200)).summary();
        assert!(summary.ends_with('…'));
        assert_eq!(summary.chars().count(), 89);
    }

    #[test]
    fn size_is_human_readable() {
        assert_eq!(found("").size_label().as_deref(), Some("18.5 MB"));
        let mut small = found("");
        small.size = Some(700 * 1024);
        assert_eq!(small.size_label().as_deref(), Some("700 KB"));
        let mut unknown = found("");
        unknown.size = None;
        assert_eq!(unknown.size_label(), None);
    }

    #[test]
    fn progress_is_a_clamped_fraction() {
        let half = Status::Downloading {
            version: "0.3.0".to_owned(),
            received: 50,
            total: Some(100),
        };
        assert!((half.progress() - 0.5).abs() < f32::EPSILON);
        let unknown = Status::Downloading {
            version: "0.3.0".to_owned(),
            received: 50,
            total: None,
        };
        assert_eq!(unknown.progress(), 0.0);
    }

    #[test]
    fn only_network_states_are_busy() {
        assert!(Status::Checking.is_busy());
        assert!(Status::Installing.is_busy());
        assert!(
            Status::Downloading {
                version: "0.3.0".to_owned(),
                received: 1,
                total: None,
            }
            .is_busy()
        );
        for idle in [
            Status::Unavailable,
            Status::Idle,
            Status::UpToDate,
            Status::Available(found("")),
            Status::Ready {
                version: "0.3.0".to_owned(),
            },
            Status::Failed("x".to_owned()),
        ] {
            assert!(!idle.is_busy(), "{idle:?} 不该挡住新的操作");
        }
    }

    #[test]
    fn only_available_and_ready_raise_a_notice() {
        assert!(Status::Available(found("")).has_notice());
        assert!(
            Status::Ready {
                version: "0.3.0".to_owned(),
            }
            .has_notice()
        );
        for quiet in [
            Status::Unavailable,
            Status::Idle,
            Status::Checking,
            Status::UpToDate,
            Status::Downloading {
                version: "0.3.0".to_owned(),
                received: 1,
                total: Some(2),
            },
            Status::Installing,
            Status::Failed("x".to_owned()),
        ] {
            assert!(!quiet.has_notice(), "{quiet:?} 不该在顶栏亮角标");
        }
    }

    /// 打包布局的两条路径都能从可执行文件往上找到配置；空目录就是开发运行
    #[test]
    fn config_is_found_in_either_packaged_layout() {
        let root =
            std::env::temp_dir().join(format!("reminder-updater-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);

        // Windows / Linux：<应用>/reminder(.exe) + <应用>/nsh/assets/
        let portable = root.join("portable");
        std::fs::create_dir_all(portable.join("nsh/assets")).unwrap();
        std::fs::write(
            portable.join("nsh/assets/nsh-updater.json"),
            br#"{"id":"cc.baiyapeng.reminder"}"#,
        )
        .unwrap();
        assert!(
            find_config(&portable).is_some(),
            "找不到 Windows / Linux 布局里的配置"
        );

        // macOS：<应用>.app/Contents/MacOS/reminder + Contents/Resources/nsh/assets/
        let bundle = root.join("Reminder.app");
        std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        std::fs::create_dir_all(bundle.join("Contents/Resources/nsh/assets")).unwrap();
        std::fs::write(
            bundle.join("Contents/Resources/nsh/assets/nsh-updater.json"),
            br#"{"id":"cc.baiyapeng.reminder"}"#,
        )
        .unwrap();
        assert!(
            find_config(&bundle.join("Contents/MacOS")).is_some(),
            "找不到 macOS .app 布局里的配置"
        );

        // 开发运行：可执行文件旁边没有那份配置
        let development = root.join("target/debug");
        std::fs::create_dir_all(&development).unwrap();
        assert!(find_config(&development).is_none());

        let _ = std::fs::remove_dir_all(&root);
    }

    /// 清单缺 SHA-256 时不能当成可安装的包（那是一张下载页）
    #[test]
    fn a_manifest_without_sha256_is_not_installable() {
        let manifest = serde_json::json!({
            "id": "cc.baiyapeng.reminder",
            "version": "0.3.0",
            "platforms": { "windows-x86_64": { "url": "https://example.com/a.zip" } }
        })
        .to_string();
        // 非穷尽结构体：按文档用 Default 建再赋值
        let mut config = Config::default();
        config.id = "cc.baiyapeng.reminder".to_owned();
        config.version = "0.2.0".to_owned();
        config.github = Some("b-yp/reminder".to_owned());

        let available = config
            .check(manifest.as_bytes(), None, "windows-x86_64")
            .unwrap()
            .unwrap();
        assert!(available.sha256.is_none());
        assert!(available.size.is_none());
    }

    /// 清单是别的应用、版本比当前旧：都不算更新
    #[test]
    fn another_app_or_an_older_version_is_not_an_update() {
        let mut config = Config::default();
        config.id = "cc.baiyapeng.reminder".to_owned();
        config.version = "0.2.0".to_owned();
        config.github = Some("b-yp/reminder".to_owned());

        let other = br#"{"id":"cc.baiyapeng.other","version":"9.9.9","platforms":{}}"#;
        assert!(
            config
                .check(other, None, "windows-x86_64")
                .unwrap()
                .is_none()
        );

        let older = br#"{"id":"cc.baiyapeng.reminder","version":"0.1.9","platforms":{"windows-x86_64":{"url":"https://example.com/a.zip","sha256":"00"}}}"#;
        assert!(
            config
                .check(older, None, "windows-x86_64")
                .unwrap()
                .is_none()
        );
    }

    /// 打包出来的配置文件名，与 `nsh-updater` 认的那份一致
    #[test]
    fn the_config_asset_name_is_the_package_one() {
        assert_eq!(CONFIG_PATHS[0], format!("nsh/assets/{}", nsh_updater::CONFIG_ASSET));
        assert_eq!(
            CONFIG_PATHS[1],
            format!("Contents/Resources/nsh/assets/{}", nsh_updater::CONFIG_ASSET)
        );
    }
}

