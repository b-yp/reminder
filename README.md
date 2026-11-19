<div align="center">
  <img src="assets/logo.png" width="96" alt="reminder 提醒" />

  <h1>reminder 提醒</h1>

  <p>一张常驻桌面的小卡片，提醒你喝水、起身、护眼，按时吃饭和睡觉。</p>

  <p>
    <img alt="platform" src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-18181b?style=flat-square">
    <img alt="rust" src="https://img.shields.io/badge/Rust-2024-b45309?style=flat-square&logo=rust&logoColor=ffffff">
    <img alt="nsh" src="https://img.shields.io/badge/NSH-0.6.0-92400e?style=flat-square">
  </p>
</div>

<img src="docs/screenshots/main.png" width="100%" alt="reminder 主界面：左为浅色模式，右为深色模式" />

<p align="center"><sub>主界面 · 左：浅色模式　右：深色模式</sub></p>

它不试图管理你的全部任务，只关心几件需要反复发生的小事：喝水、起身、护眼、按时吃饭和睡觉。到点会主动提醒，打卡后重新计时；收起成胶囊、收进托盘，计时也不会停。

## 功能

| 能力 | 说明 |
| --- | --- |
| **两种提醒节奏** | 每隔 N 分钟循环一次，或每天固定时刻提醒；自定义最长支持 24 小时 |
| **到点真的会叫你** | 提示音 + 原生通知 + 把悬浮窗唤到最前，不容易错过 |
| **打卡与今日统计** | 一键打卡、延后 5 分钟、环形进度；按天记录次数，底栏汇总今日打卡 |
| **常用预设** | 喝水、起身活动、护眼远眺、番茄专注、带薪拉屎、站立拉伸、早餐、午餐、下班打卡、睡觉 |
| **迷你胶囊** | 收起为 196×44 的小胶囊，优先显示最紧急的提醒，可直接打卡或展开 |
| **桌面集成** | 置顶悬浮、随处拖动、位置记忆、托盘常驻、全局热键、开机自启 |
| **外观** | 浅色 / 深色 / 跟随系统，20%–100% 窗口不透明度，金黄品牌强调色 |
| **本地优先** | 单个 JSON 文件，原子写入，离线时间补偿，全程不联网 |

## 快速开始

需要 [Rust](https://rustup.rs/) 与 `nsh` 0.6.0（安装方式见 [NSH 文档](https://nsh.dev/docs/installation)）。macOS 需要 Xcode Command Line Tools，Windows 需要 MSVC 生成工具，Linux 需要 GTK 4 开发库。

```sh
git clone https://github.com/b-yp/reminder.git
cd reminder
nsh run
```

`nsh run` 会用本机的原生桌面后端启动应用：macOS 上是 AppKit，Windows 上是 Win32，Linux 上是 GTK 4。
也可以直接 `cargo run`，同样会构建本机的原生后端。

打包：

```sh
nsh package --target macos    # 或 windows / linux
```

产物在 `.nsh/out/`。

## 使用

### 添加提醒

点顶栏的 **+**，进入「新建提醒」抽屉：

<img src="docs/screenshots/add.png" width="100%" alt="新建提醒：常用推荐，左浅色右深色" />

<p align="center"><sub>常用推荐 · 左：浅色模式　右：深色模式</sub></p>

常用推荐里点一下即可加入；已经在列表中的模板会显示为「已加入」，不会重复添加。

切到「自定义新提醒」可以自行配置名称、循环周期或每天定点，以及 16 个图标主题：

<img src="docs/screenshots/custom.png" width="100%" alt="自定义新提醒：左浅色右深色" />

<p align="center"><sub>自定义新提醒 · 左：浅色模式　右：深色模式</sub></p>

### 打卡、延后与撤销

- 右侧 **✓** 打卡：记录次数、重置间隔，或把当天的定点提醒标记为完成。
- 点击行内的周期徽章（如 `30m`）可以快速轮换间隔时长；定点提醒按墙上时间走，不参与轮换。
- 行尾 **⋯** 菜单里有「延后 5 分钟」和「删除提醒」；删除后 5 秒内可以在底部 Toast 里撤销。
- 到期的行会变成淡红警示色，倒计时变成「超时 mm:ss」，迷你胶囊也会优先显示它。

### 迷你胶囊

顶栏右上角的收起按钮会把整张卡片折成 196×44 的胶囊，贴在屏幕边缘也不挡事：

<img src="docs/screenshots/mini.png" width="100%" alt="迷你胶囊：左浅色右深色" />

<p align="center"><sub>迷你胶囊 · 左：浅色模式　右：深色模式</sub></p>

胶囊里显示当前最紧急的提醒，以及它在这批提醒中的位置（如 `3/5`）；点 **✓** 直接打卡，点展开按钮回到完整卡片。

### 偏好设置

点顶栏的齿轮打开设置：

<img src="docs/screenshots/settings.png" width="100%" alt="偏好设置：左浅色右深色" />

<p align="center"><sub>偏好设置 · 左：浅色模式　右：深色模式</sub></p>

- **外观主题**：浅色、深色，或跟随系统。
- **窗口不透明度**：20%–100%，实时生效并记忆。
- **全局热键**：默认 macOS 为 `⌥⌘R`，Windows / Linux 为 `Ctrl+Alt+R`；可在「换一组」里轮换。
- **开机自启**：登录系统后自动运行，登记进各平台自己的启动项。

> 截图中的全局热键为 `⇧⌘Space`，默认值是 `⌥⌘R`，在设置里可以随时换一组。

### 托盘与关闭

关闭按钮不是退出，而是收进系统托盘；计时、提示音和托盘图标继续工作。左键点击托盘图标唤回窗口，右键菜单里可以显示 / 隐藏窗口，或真正退出。

## 数据与隐私

所有数据都在本机，没有账号、没有云同步、没有网络请求。

| 平台 | 状态文件 |
| --- | --- |
| macOS | `~/Library/Application Support/cc.baiyapeng.reminder/state.json` |
| Windows | `%APPDATA%\cc.baiyapeng.reminder\state.json` |
| Linux | `$XDG_CONFIG_HOME/cc.baiyapeng.reminder/state.json`（默认 `~/.config/…`） |

设置环境变量 `REMINDER_DATA_PATH` 可以把它指到任意位置。文件里保存提醒列表、主题、不透明度、热键、窗口位置和上次保存时间；写入采用临时文件 + 原子替换，避免意外关机写坏数据。启动时若发现距离上次保存已经过了一段时间，会把离线期间流逝的时间补算到未暂停的间隔提醒上（最多 7 天）。

## 项目结构

| 路径 | 职责 |
| --- | --- |
| `src/app.rs`、`src/app/` | 根组件、响应式状态上下文、托盘 / 关闭 / 主题 / 热键 / 心跳等应用级副作用 |
| `src/views/` | 完整卡片、迷你胶囊、顶栏与底栏、提醒列表、新建抽屉、设置抽屉、撤销 Toast |
| `src/ui/` | 应用自己的控件外观层与设计令牌，界面统一从这里组合 |
| `src/model.rs` | 提醒模型、间隔 / 定点调度、打卡与超时计算 |
| `src/storage.rs` | JSON 持久化、原子写入、离线时间补偿 |
| `src/window.rs` | 平台窗口层：置顶悬浮、透明度、托盘、关闭拦截 |
| `src/hotkey.rs`、`src/autostart.rs`、`src/alarm.rs` | 全局热键、开机自启、到期提示音与通知 |
| `src/theme_mode.rs`、`src/icons.rs`、`src/presets.rs` | 主题令牌与对比度测试、矢量图标、常用预设 |

界面遵循 NSH 的编码与设计规范：主题色统一来自 `nsh_base::theme()`，品牌金黄作为强调色，`theme_mode` 里有 WCAG AA 的对比度回归测试；交互控件基于 NSH 的语义组件构建，保留键盘操作与读屏标签。

## 开发与测试

```sh
nsh check                    # cargo check + TypeScript 测试的类型检查
nsh test                     # cargo test + App.test.tsx（无头）
nsh test nsh.json --trace    # 看根组件挂载了什么、每次提交的耗时
cargo test                   # 只跑 Rust 侧的无头 UI / 逻辑测试
```

`App.test.tsx` 用 TypeScript 驱动真实 Rust 根组件，覆盖了主界面、设置抽屉、新建抽屉和表单复位等流程。

README 里的截图不是设计稿：它们由 `nsh test --target macos` 驱动真实窗口，在 macOS 上分别以浅色和深色渲染后截取，再拼成一张对照图。截图后统一补了背景和投影，并裁掉了窗口自带的圆角阴影边缘。为了在一张图里完整展示抽屉内容，设置和自定义表单的截图把窗口临时拉高了；日常运行时窗口默认是 320×350，内容超出时在卡片内滚动。

## 平台说明

- macOS、Windows、Linux 上提供完整的悬浮窗、托盘、全局热键与开机自启。
- Web 和移动端目标可以运行界面，但没有系统托盘、全局热键和登录启动项；设置里会显示「不可用」。
- 提醒音使用系统默认提示音；通知走各平台原生通知中心，首次使用时可能需要授予通知权限。
- 默认窗口尺寸为 320×350（卡片）或 196×44（胶囊）；抽屉内容超出时在卡片内滚动。

<div align="center"><sub>愿它帮你记得喝水、起身，也记得按时休息。</sub></div>
