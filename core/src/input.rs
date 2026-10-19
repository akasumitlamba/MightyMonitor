//! Serializable remote input events.

use serde::{Deserialize, Serialize};

/// Remote input. Mouse and touch coordinates are normalized to `0..=1`.
/// Hosts must call validate before dispatching. Keys use USB HID keyboard usages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputEvent {
    MouseMove {
        x: f32,
        y: f32,
    },
    MouseButton {
        button: MouseButton,
        pressed: bool,
    },
    Scroll {
        dx: f32,
        dy: f32,
    },
    Key {
        code: u32,
        pressed: bool,
    },
    Touch {
        id: u32,
        phase: TouchPhase,
        x: f32,
        y: f32,
    },
    /// Release every remotely held key/button, for focus loss or control revocation.
    ReleaseAll,
    Text(String),
}
impl InputEvent {
    pub fn validate(&self) -> bool {
        let coordinate = |x: f32| x.is_finite() && (0.0..=1.0).contains(&x);
        match self {
            Self::MouseMove { x, y } => coordinate(*x) && coordinate(*y),
            Self::Touch { id, x, y, .. } => *id < 16 && coordinate(*x) && coordinate(*y),
            Self::Scroll { dx, dy } => {
                dx.is_finite() && dy.is_finite() && dx.abs() <= 120.0 && dy.abs() <= 120.0
            }
            Self::Key { code, .. } => (4..=0xe7).contains(code),
            Self::Text(text) => !text.is_empty() && text.len() <= 4096 && !text.contains('\0'),
            Self::MouseButton { .. } | Self::ReleaseAll => true,
        }
    }
}

/// A mouse button.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

/// Phase of a touch contact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TouchPhase {
    Start,
    Move,
    End,
    Cancel,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Message;

    #[test]
    fn rejects_unbounded_or_nonfinite_input() {
        assert!(!InputEvent::MouseMove {
            x: f32::NAN,
            y: 0.0
        }
        .validate());
        assert!(!InputEvent::Scroll {
            dx: 0.0,
            dy: f32::INFINITY
        }
        .validate());
        assert!(!InputEvent::Key {
            code: u32::MAX,
            pressed: true
        }
        .validate());
        assert!(!InputEvent::Text("x".repeat(4097)).validate());
        assert!(InputEvent::MouseMove { x: 0.0, y: 1.0 }.validate());
        assert!(InputEvent::ReleaseAll.validate());
    }
    #[test]
    fn input_event_roundtrip() {
        let mut events = vec![
            InputEvent::MouseMove { x: 0.25, y: 0.75 },
            InputEvent::Scroll { dx: -1.5, dy: 2.0 },
            InputEvent::Key {
                code: 42,
                pressed: true,
            },
        ];
        for button in [
            MouseButton::Left,
            MouseButton::Right,
            MouseButton::Middle,
            MouseButton::Back,
            MouseButton::Forward,
        ] {
            events.push(InputEvent::MouseButton {
                button,
                pressed: false,
            });
        }
        for phase in [
            TouchPhase::Start,
            TouchPhase::Move,
            TouchPhase::End,
            TouchPhase::Cancel,
        ] {
            events.push(InputEvent::Touch {
                id: 7,
                phase,
                x: 0.0,
                y: 1.0,
            });
        }
        for event in events {
            let message = Message::Input(event);
            assert_eq!(
                Message::from_bytes(&message.to_bytes().unwrap()).unwrap(),
                message
            );
        }
    }
}
