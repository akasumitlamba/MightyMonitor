#![cfg(target_os = "linux")]
//! Run only on an isolated X server: verifies actual XTest key/button delivery.
use mightymonitor_core::{InputEvent, MouseButton};
use mightymonitor_host::input::{desktop::DesktopInput, InputLease};
use std::time::{Duration, Instant};
use x11rb::{
    connection::Connection,
    protocol::{xproto::*, Event},
    COPY_FROM_PARENT, CURRENT_TIME,
};
#[test]
#[ignore = "Requires an isolated Xvfb and MM_ALLOW_INPUT_SMOKE=1"]
fn x11_input_smoke() {
    assert_eq!(
        std::env::var("MM_ALLOW_INPUT_SMOKE").as_deref(),
        Ok("1"),
        "Use an isolated Xvfb and explicitly enable this input test"
    );
    let (connection, index) = x11rb::connect(None).unwrap();
    let screen = &connection.setup().roots[index];
    let window = connection.generate_id().unwrap();
    connection
        .create_window(
            COPY_FROM_PARENT as u8,
            window,
            screen.root,
            20,
            20,
            320,
            200,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new().event_mask(
                EventMask::KEY_PRESS
                    | EventMask::KEY_RELEASE
                    | EventMask::BUTTON_PRESS
                    | EventMask::BUTTON_RELEASE,
            ),
        )
        .unwrap()
        .check()
        .unwrap();
    connection.map_window(window).unwrap().check().unwrap();
    connection
        .set_input_focus(InputFocus::PARENT, window, CURRENT_TIME)
        .unwrap()
        .check()
        .unwrap();
    connection.flush().unwrap();
    let mut lease = InputLease(Box::new(
        DesktopInput::new(
            screen.width_in_pixels.into(),
            screen.height_in_pixels.into(),
        )
        .unwrap(),
    ));
    lease
        .0
        .apply(&InputEvent::MouseMove {
            x: 60.0 / (screen.width_in_pixels - 1) as f32,
            y: 80.0 / (screen.height_in_pixels - 1) as f32,
        })
        .unwrap();
    lease
        .0
        .apply(&InputEvent::MouseButton {
            button: MouseButton::Left,
            pressed: true,
        })
        .unwrap();
    lease
        .0
        .apply(&InputEvent::Key {
            code: 4,
            pressed: true,
        })
        .unwrap();
    drop(lease);
    let mut seen = [false; 4];
    let deadline = Instant::now() + Duration::from_secs(3);
    while !seen.iter().all(|v| *v) && Instant::now() < deadline {
        match connection.poll_for_event().unwrap() {
            Some(Event::KeyPress(_)) => seen[0] = true,
            Some(Event::KeyRelease(_)) => seen[1] = true,
            Some(Event::ButtonPress(_)) => seen[2] = true,
            Some(Event::ButtonRelease(_)) => seen[3] = true,
            _ => std::thread::sleep(Duration::from_millis(5)),
        }
    }
    assert_eq!(
        seen, [true; 4],
        "A real key/button press or its lease-drop release was missing"
    );
    let pointer = connection
        .query_pointer(screen.root)
        .unwrap()
        .reply()
        .unwrap();
    assert_eq!((pointer.root_x, pointer.root_y), (60, 80));
}
