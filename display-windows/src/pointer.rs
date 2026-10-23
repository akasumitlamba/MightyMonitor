//! Explicit pointer movement on a validated display within the virtual desktop.
use super::Display;
use std::io;

#[derive(Clone, Copy)]
struct DesktopRect {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}
fn desktop_bounds(screens: &[Display]) -> io::Result<DesktopRect> {
    let left = screens.iter().map(|screen| screen.left).min();
    let top = screens.iter().map(|screen| screen.top).min();
    let right = screens
        .iter()
        .map(|screen| i64::from(screen.left) + i64::from(screen.width))
        .max();
    let bottom = screens
        .iter()
        .map(|screen| i64::from(screen.top) + i64::from(screen.height))
        .max();
    match (left, top, right, bottom) {
        (Some(left), Some(top), Some(right), Some(bottom)) => Ok(DesktopRect {
            left,
            top,
            width: i32::try_from(right - i64::from(left))
                .map_err(|_| io::Error::other("Desktop width is out of range"))?,
            height: i32::try_from(bottom - i64::from(top))
                .map_err(|_| io::Error::other("Desktop height is out of range"))?,
        }),
        _ => Err(io::Error::other("No active desktop screens")),
    }
}
fn coordinates(display: &Display, desktop: DesktopRect, x: f32, y: f32) -> io::Result<(i32, i32)> {
    if !x.is_finite()
        || !y.is_finite()
        || !(0.0..=1.0).contains(&x)
        || !(0.0..=1.0).contains(&y)
        || desktop.width < 2
        || desktop.height < 2
        || display.width < 2
        || display.height < 2
        || i64::from(display.left) < i64::from(desktop.left)
        || i64::from(display.top) < i64::from(desktop.top)
        || i64::from(display.left) + i64::from(display.width)
            > i64::from(desktop.left) + i64::from(desktop.width)
        || i64::from(display.top) + i64::from(display.height)
            > i64::from(desktop.top) + i64::from(desktop.height)
    {
        return Err(io::Error::other(
            "Selected screen is outside the current desktop or pointer position is invalid",
        ));
    }
    let px = f64::from(display.left) + f64::from(x) * f64::from(display.width - 1);
    let py = f64::from(display.top) + f64::from(y) * f64::from(display.height - 1);
    Ok((
        ((px - f64::from(desktop.left)) * 65535.0 / f64::from(desktop.width - 1)).round() as i32,
        ((py - f64::from(desktop.top)) * 65535.0 / f64::from(desktop.height - 1)).round() as i32,
    ))
}

/// Requires explicit local and viewer control consent at the calling layer.
/// This does not change monitor configuration or bypass Windows input restrictions.
#[cfg(target_os = "windows")]
pub fn move_pointer(display: &Display, x: f32, y: f32) -> io::Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE,
        MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT,
    };
    // Use one physical-coordinate enumeration for identity and desktop bounds;
    // do not combine window/DPI-scaled metrics with a physical capture rectangle.
    let screens = super::displays()?;
    super::validate_snapshot(display, &screens)?;
    let desktop = desktop_bounds(&screens)?;
    let (dx, dy) = coordinates(display, desktop, x, y)?;
    let event = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                ..Default::default()
            },
        },
    };
    // SAFETY: INPUT is fully initialized with its matching mouse union member;
    // the slice lives for this synchronous call and cbsize is its exact ABI size.
    if unsafe { SendInput(&[event], std::mem::size_of::<INPUT>() as i32) } != 1 {
        return Err(io::Error::other("Windows did not accept pointer input. Check desktop access and application privileges."));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn display(left: i32, top: i32, width: u32, height: u32) -> Display {
        Display {
            device: "test".into(),
            handle: 1,
            left,
            top,
            width,
            height,
            primary: false,
        }
    }
    fn inverse(value: i32, origin: i32, size: i32) -> i32 {
        (f64::from(origin) + f64::from(value) * f64::from(size - 1) / 65535.0).round() as i32
    }
    #[test]
    fn targets_left_above_and_offset_monitors_without_primary_screen_scaling() {
        let screens = [
            display(-1920, 0, 1920, 1080),
            display(0, -1200, 1920, 1200),
            display(1920, 0, 1920, 2160),
            display(0, 0, 1920, 1080),
        ];
        let desktop = desktop_bounds(&screens).unwrap();
        assert_eq!(
            (desktop.left, desktop.top, desktop.width, desktop.height),
            (-1920, -1200, 5760, 3360)
        );
        for screen in screens {
            for (x, y) in [(0.0, 0.0), (0.5, 0.5), (1.0, 1.0)] {
                let (dx, dy) = coordinates(&screen, desktop, x, y).unwrap();
                let px = screen.left + (f64::from(x) * f64::from(screen.width - 1)).round() as i32;
                let py = screen.top + (f64::from(y) * f64::from(screen.height - 1)).round() as i32;
                assert!((inverse(dx, desktop.left, desktop.width) - px).abs() <= 1);
                assert!((inverse(dy, desktop.top, desktop.height) - py).abs() <= 1);
                assert!((0..=65535).contains(&dx) && (0..=65535).contains(&dy));
            }
        }
    }
    #[test]
    fn rejects_invalid_points_and_a_monitor_outside_the_updated_desktop() {
        assert!(desktop_bounds(&[]).is_err());
        let desktop = DesktopRect {
            left: 0,
            top: 0,
            width: 1920,
            height: 1080,
        };
        let screen = display(0, 0, 1920, 1080);
        for (x, y) in [
            (f32::NAN, 0.0),
            (0.0, f32::INFINITY),
            (-0.01, 0.5),
            (0.5, 1.01),
        ] {
            assert!(coordinates(&screen, desktop, x, y).is_err());
        }
        assert!(coordinates(&display(-1920, 0, 1920, 1080), desktop, 0.5, 0.5).is_err());
        assert!(coordinates(
            &screen,
            DesktopRect {
                width: 0,
                ..desktop
            },
            0.5,
            0.5
        )
        .is_err());
        assert_eq!(coordinates(&screen, desktop, 0.0, 0.0).unwrap(), (0, 0));
        assert_eq!(
            coordinates(&screen, desktop, 1.0, 1.0).unwrap(),
            (65535, 65535)
        );
    }
}
