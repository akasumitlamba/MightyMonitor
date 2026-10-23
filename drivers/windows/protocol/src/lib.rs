//! Original, bounded per-connection protocol. No global monitor-list mutation.
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
use std::io;

pub const PIPE: &str = r"\\.\pipe\MightyMonitor.Display.v1";
pub const MAX_FRAME: usize = 192;
pub const MAX_CLIENTS: usize = 4;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mode {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
}
impl Mode {
    pub fn validate(self) -> io::Result<()> {
        if !(640..=3840).contains(&self.width)
            || !(480..=2160).contains(&self.height)
            || self.width & 1 != 0
            || self.height & 1 != 0
            || !(1..=60).contains(&self.refresh_hz)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unsupported display mode",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Create { mode: Mode },
    Check { lease: u64 },
    Remove { lease: u64 },
    Ping {},
}
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    Created {
        lease: u64,
        adapter_luid: i64,
        target_id: u32,
    },
    Ok {},
    Rejected {},
}
/// A two-byte little-endian size precedes each strictly bounded JSON payload.
/// Parsers reject unknown fields/commands and trailing payloads; no old Notify,
/// RemoveAll, foreign lease enumeration or arbitrary monitor ID is accepted.
pub fn encode<T: Serialize>(value: &T) -> io::Result<Vec<u8>> {
    let payload = serde_json::to_vec(value).map_err(io::Error::other)?;
    if payload.is_empty() || payload.len() > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid driver frame size",
        ));
    }
    let mut frame = Vec::with_capacity(payload.len() + 2);
    frame.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}
pub fn decode_request(payload: &[u8]) -> io::Result<Request> {
    if payload.is_empty() || payload.len() > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid driver frame size",
        ));
    }
    let request = serde_json::from_slice(payload).map_err(io::Error::other)?;
    if let Request::Create { mode } = request {
        mode.validate()?;
    }
    Ok(request)
}
pub fn decode_response(payload: &[u8]) -> io::Result<Response> {
    if payload.is_empty() || payload.len() > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid driver frame size",
        ));
    }
    serde_json::from_slice(payload).map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_global_mutations_unknown_fields_and_trailing_requests() {
        for data in [
            r#"{"op":"notify","monitors":[]}"#,
            r#"{"op":"remove_all"}"#,
            r#"{"op":"ping","monitor_id":1}"#,
            r#"{"op":"ping"}{"op":"ping"}"#,
            r#"{"op":"create","mode":{"width":1920,"height":1080,"refresh_hz":60,"extra":1}}"#,
        ] {
            assert!(decode_request(data.as_bytes()).is_err(), "accepted: {data}");
        }
        assert!(decode_request(&[b' '; MAX_FRAME + 1]).is_err());
        assert!(decode_response(br#"{"result":"ok","foreign":1}"#).is_err());
    }
    #[test]
    fn round_trips_owned_commands_and_full_os_identity() {
        let request = Request::Create {
            mode: Mode {
                width: 1920,
                height: 1080,
                refresh_hz: 60,
            },
        };
        let frame = encode(&request).unwrap();
        assert_eq!(
            u16::from_le_bytes([frame[0], frame[1]]) as usize,
            frame.len() - 2
        );
        assert_eq!(decode_request(&frame[2..]).unwrap(), request);
        let response = Response::Created {
            lease: u64::MAX,
            adapter_luid: i64::MIN,
            target_id: u32::MAX,
        };
        let frame = encode(&response).unwrap();
        assert_eq!(
            serde_json::from_slice::<Response>(&frame[2..]).unwrap(),
            response
        );
    }
    #[test]
    fn invalid_modes_never_reach_the_driver() {
        for mode in [
            Mode {
                width: 0,
                height: 1080,
                refresh_hz: 60,
            },
            Mode {
                width: 1921,
                height: 1080,
                refresh_hz: 60,
            },
            Mode {
                width: 1920,
                height: 1080,
                refresh_hz: 0,
            },
            Mode {
                width: 8192,
                height: 8192,
                refresh_hz: 120,
            },
        ] {
            let frame = encode(&Request::Create { mode }).unwrap();
            assert!(decode_request(&frame[2..]).is_err());
        }
    }
}
