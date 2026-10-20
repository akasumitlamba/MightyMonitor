//! Read-only Windows display snapshots. No display configuration or driver install.
//! Native pointers stay inside the adapter; callers receive owned snapshot data.
#![deny(unsafe_op_in_unsafe_fn)]
use std::io;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Display {
    device: String,
    handle: u64,
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
}
impl Display {
    pub fn device(&self) -> &str {
        &self.device
    }
    /// Session-local HMONITOR value for the capture source, never persisted.
    pub fn handle(&self) -> u64 {
        self.handle
    }
    pub fn label(&self) -> String {
        format!(
            "{} · {} × {}{}",
            self.device.trim_start_matches(r"\\.\"),
            self.width,
            self.height,
            if self.primary { " · Main" } else { "" }
        )
    }
    /// Reject unplugging, handle reuse and layout/resolution changes. Never fall back.
    pub fn validate(&self) -> io::Result<()> {
        validate_snapshot(self, &displays()?)
    }
}
fn validate_snapshot(selected: &Display, current: &[Display]) -> io::Result<()> {
    if current.iter().any(|display| display == selected) {
        Ok(())
    } else {
        Err(io::Error::other(
            "Selected screen changed or disconnected. Stop sharing and select it again.",
        ))
    }
}
pub fn select(device: &str) -> io::Result<Display> {
    displays()?
        .into_iter()
        .find(|display| display.device == device)
        .ok_or_else(|| {
            io::Error::other("Selected screen is unavailable. Refresh screens and select it again.")
        })
}

#[cfg(not(target_os = "windows"))]
pub fn displays() -> io::Result<Vec<Display>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Windows screen selection requires Windows",
    ))
}

#[cfg(target_os = "windows")]
pub fn displays() -> io::Result<Vec<Display>> {
    use windows::{
        core::BOOL,
        Win32::{
            Foundation::{LPARAM, RECT},
            Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW},
        },
    };
    #[derive(Default)]
    struct State {
        items: Vec<Display>,
        failed: bool,
    }
    unsafe extern "system" fn visit(handle: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
        // SAFETY: EnumDisplayMonitors calls synchronously on this thread. data
        // points to the exclusively borrowed State alive for the entire call.
        let state = unsafe { &mut *(data.0 as *mut State) };
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        // SAFETY: MONITORINFOEXW is repr(C), starts with MONITORINFO, and cbSize
        // advertises its full initialized allocation. The handle comes from Windows.
        if !unsafe { GetMonitorInfoW(handle, std::ptr::addr_of_mut!(info).cast()) }.as_bool() {
            state.failed = true;
            return BOOL(0);
        }
        let rect = info.monitorInfo.rcMonitor;
        let width = i64::from(rect.right) - i64::from(rect.left);
        let height = i64::from(rect.bottom) - i64::from(rect.top);
        let end = info
            .szDevice
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(info.szDevice.len());
        let Ok(device) = String::from_utf16(&info.szDevice[..end]) else {
            state.failed = true;
            return BOOL(0);
        };
        if width <= 0
            || height <= 0
            || width > u32::MAX as i64
            || height > u32::MAX as i64
            || device.is_empty()
            || handle.is_invalid()
            || state.items.len() >= 64
        {
            state.failed = true;
            return BOOL(0);
        }
        state.items.push(Display {
            device,
            handle: handle.0 as usize as u64,
            left: rect.left,
            top: rect.top,
            width: width as u32,
            height: height as u32,
            primary: info.monitorInfo.dwFlags & 1 != 0,
        });
        BOOL(1)
    }
    let mut state = State::default();
    // SAFETY: no HDC/clip is supplied. State remains alive and is accessed only
    // by the synchronous callback. No pointer or native ownership escapes.
    let success = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(visit),
            LPARAM(&mut state as *mut State as isize),
        )
    };
    if !success.as_bool() || state.failed || state.items.is_empty() {
        return Err(io::Error::other(
            "Could not read Windows screens. Refresh and try again.",
        ));
    }
    state.items.sort_by(|a, b| a.device.cmp(&b.device));
    Ok(state.items)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_disappearance_handle_reuse_and_layout_changes() {
        let selected = Display {
            device: r"\\.\DISPLAY2".into(),
            handle: 12,
            left: -1920,
            top: 0,
            width: 1920,
            height: 1080,
            primary: false,
        };
        assert!(validate_snapshot(&selected, std::slice::from_ref(&selected)).is_ok());
        assert!(validate_snapshot(&selected, &[]).is_err());
        for changed in [
            Display {
                handle: 13,
                ..selected.clone()
            },
            Display {
                device: r"\\.\DISPLAY3".into(),
                ..selected.clone()
            },
            Display {
                left: 0,
                ..selected.clone()
            },
            Display {
                width: 1280,
                ..selected.clone()
            },
            Display {
                primary: true,
                ..selected.clone()
            },
        ] {
            assert!(validate_snapshot(&selected, &[changed]).is_err());
        }
        assert_eq!(selected.label(), "DISPLAY2 · 1920 × 1080");
    }
    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "requires an interactive Windows desktop"]
    fn enumerated_screens_can_be_selected_and_validated() {
        for display in displays().unwrap() {
            assert_eq!(select(display.device()).unwrap(), display);
            display.validate().unwrap();
        }
    }
}
