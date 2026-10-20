//! Windows shortcut target: configure our private codec runtime before loading it.
#![forbid(unsafe_code)]
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
#[cfg(target_os = "windows")]
mod launch_args;
#[cfg(target_os = "windows")]
fn launch() -> Result<i32, Box<dyn std::error::Error>> {
    use std::{env, path::PathBuf, process::Command};
    let root = PathBuf::from(
        env::var_os("LOCALAPPDATA").ok_or("Windows local application folder unavailable")?,
    )
    .join("MightyMonitor")
    .join("gstreamer-1.28.8");
    let bin = root.join("bin");
    if !bin.join("gstreamer-1.0-0.dll").is_file() {
        return Err("The video runtime is missing. Run the MightyMonitor installer again.".into());
    }
    let mut paths = vec![bin];
    paths.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));
    let executable = env::current_exe()?.with_file_name("mm-viewer.exe");
    let mut command = Command::new(executable);
    command
        .env("PATH", env::join_paths(paths)?)
        .env(
            "GST_PLUGIN_PATH_1_0",
            root.join("lib").join("gstreamer-1.0"),
        )
        .env(
            "GST_PLUGIN_SYSTEM_PATH_1_0",
            root.join("lib").join("gstreamer-1.0"),
        )
        .env(
            "GST_PLUGIN_SCANNER_1_0",
            root.join("libexec")
                .join("gstreamer-1.0")
                .join("gst-plugin-scanner.exe"),
        );
    let args: Vec<_> = env::args_os().skip(1).collect();
    let forwarded = match launch_args::parse(args)? {
        launch_args::Launch::App => Vec::new(),
        launch_args::Launch::CheckRuntime => vec![std::ffi::OsString::from("--check-runtime")],
        launch_args::Launch::Invitation(link) => vec![
            std::ffi::OsString::from("--invite"),
            std::ffi::OsString::from(link),
        ],
    };
    command.args(forwarded);
    Ok(command.status()?.code().unwrap_or(1))
}
#[cfg(target_os = "windows")]
fn main() {
    match launch() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            if std::env::args().nth(1).as_deref() == Some("--check-runtime") {
                eprintln!("{error}");
                std::process::exit(1);
            }
            // The script is constant; diagnostic text is passed as environment data.
            let _ = std::process::Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", "Add-Type -AssemblyName PresentationFramework; [System.Windows.MessageBox]::Show($env:MM_STARTUP_ERROR, 'MightyMonitor') | Out-Null"])
                .env("MM_STARTUP_ERROR", error.to_string()).status();
            std::process::exit(1);
        }
    }
}
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("This launcher is for the Windows installer.");
    std::process::exit(1);
}
