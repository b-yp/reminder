//! Runs the Rust application directly on the machine's native backend.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
use nsh_platform_linux as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn main() -> impl std::process::Termination {
    env_logger::init();

    platform::run_app(
        platform::AppSource::Rust(platform::Root::of::<nsh_app_reminder::App>()),
        platform::AppOptions {
            title: "reminder".to_owned(),
            appearance: platform::Appearance::Dark,
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            app_id: nsh_app_reminder::APP_ID.to_owned(),
            size: (nsh_app_reminder::WINDOW_WIDTH as i32, nsh_app_reminder::WINDOW_HEIGHT as i32),
            #[cfg(target_os = "windows")]
            icon: Some(std::path::PathBuf::from(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/app.ico"
            ))),
            ..Default::default()
        },
    )
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!(
    "Run this binary on macOS, Windows or Linux; use the TSX entry with nsh run for iOS and Android."
);
