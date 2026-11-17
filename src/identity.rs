//! 应用身份：反向域名标识的唯一来源。
//!
//! 这一个字符串同时决定：打包后的 macOS / iOS bundle id 与 Android applicationId、
//! 通知归属与系统权限主体、开机自启登记项、用户数据目录，以及卸载时要清掉的残留。
//! 所以代码里不再各写一份，`nsh.json` 的 `"id"` 也必须与它一致（见下方测试）。
//!
//! 只使用字母、数字和点：Android 的 applicationId 不接受连字符，
//! Apple 的 bundle id 不接受下划线。

/// 应用标识，按自有域名反向书写（`baiyapeng.cc` → `cc.baiyapeng.reminder`）
pub const APP_ID: &str = "cc.baiyapeng.reminder";

#[cfg(test)]
mod tests {
    use super::APP_ID;

    fn nsh_json() -> serde_json::Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("nsh.json");
        let text = std::fs::read_to_string(&path).expect("read nsh.json");
        serde_json::from_str(&text).expect("parse nsh.json")
    }

    /// 商店、更新和系统登记都认 `nsh.json` 的 id，改动必须两边同时改
    #[test]
    fn nsh_json_id_matches_app_id() {
        assert_eq!(nsh_json()["id"].as_str(), Some(APP_ID));
    }

    /// 发布元数据一份：`nsh.json` 的 version 就是 crate 版本
    #[test]
    fn nsh_json_version_matches_crate_version() {
        assert_eq!(
            nsh_json()["version"].as_str(),
            Some(env!("CARGO_PKG_VERSION"))
        );
    }

    /// Android applicationId 与 Apple bundle id 的交集：至少两段，每段字母开头、只含字母数字
    #[test]
    fn app_id_is_a_valid_bundle_identifier() {
        let segments: Vec<&str> = APP_ID.split('.').collect();
        assert!(segments.len() >= 2, "标识至少要有一个点：{APP_ID}");
        for segment in segments {
            assert!(!segment.is_empty(), "标识里不能有空段：{APP_ID}");
            assert!(
                segment.starts_with(|c: char| c.is_ascii_alphabetic()),
                "每段必须以字母开头：{segment}"
            );
            assert!(
                segment.chars().all(|c| c.is_ascii_alphanumeric()),
                "只允许字母和数字：{segment}"
            );
        }
    }
}
