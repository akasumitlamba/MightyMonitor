//! Strict startup arguments, shared with the std-only Windows runtime launcher.
//! Parsing never connects, starts sharing or grants remote control.
use std::ffi::OsString;
pub enum Launch {
    App,
    CheckRuntime,
    Invitation(String),
}
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Launch, &'static str> {
    let values = args
        .into_iter()
        .map(|arg| arg.into_string().map_err(|_| "Invalid startup argument"))
        .collect::<Result<Vec<_>, _>>()?;
    match values.as_slice() {
        [] => Ok(Launch::App),
        [flag] if flag == "--check-runtime" => Ok(Launch::CheckRuntime),
        [link] if invitation(link) => Ok(Launch::Invitation(link.clone())),
        [flag, link] if flag == "--invite" && invitation(link) => {
            Ok(Launch::Invitation(link.clone()))
        }
        _ => Err("Pass one private connection link, or open MightyMonitor without arguments"),
    }
}
fn invitation(text: &str) -> bool {
    text.len() <= 512
        && text.starts_with("mightymonitor://")
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b":/._-[]%".contains(&byte))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Result<Launch, &'static str> {
        parse(values.iter().map(OsString::from))
    }
    #[test]
    fn startup_link_does_not_enable_any_other_action() {
        let link = "mightymonitor://192.0.2.1:5000/fingerprint/token";
        assert!(matches!(args(&[link]), Ok(Launch::Invitation(_))));
        assert!(matches!(
            args(&["--invite", link]),
            Ok(Launch::Invitation(_))
        ));
        for extra in ["--allow-control", "--secure", "--check-runtime"] {
            assert!(args(&[link, extra]).is_err());
        }
        assert!(args(&["https://example.com"]).is_err());
        assert!(args(&["mightymonitor://bad\nlink"]).is_err());
        assert!(args(&["--invite"]).is_err());
        assert!(args(&[&format!("mightymonitor://{}", "x".repeat(513))]).is_err());
    }
}
