//! Android video client: original Rust protocol and bounded networking worker.
//! Java is limited to platform UI/MediaCodec; no screen protocol is reimplemented there.
#![deny(unsafe_op_in_unsafe_fn)]
use mightymonitor_core::{
    encode_stream_frame, Codec, FrameDecoder, Message, NetStats, Role, PROTOCOL_VERSION,
};
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    net::{SocketAddr, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
#[cfg(feature = "jni")]
mod native;
type Error = Box<dyn std::error::Error + Send + Sync>;
const QUEUE_CAPACITY: usize = 3;

struct Shared {
    events: Mutex<VecDeque<Vec<u8>>>,
    stopped: AtomicBool,
}
impl Shared {
    fn emit(&self, mut event: Vec<u8>) -> bool {
        loop {
            if self.stopped.load(Ordering::Relaxed) {
                return false;
            }
            {
                let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
                if events.len() < QUEUE_CAPACITY {
                    events.push_back(std::mem::take(&mut event));
                    return true;
                }
            }
            // Preserve H.264 reference order: apply backpressure, never discard
            // an arbitrary encoded picture. Stop remains observable while full.
            thread::sleep(Duration::from_millis(10));
        }
    }
    fn text(&self, kind: u8, text: &str) -> bool {
        let mut event = vec![kind];
        event.extend_from_slice(text.as_bytes());
        self.emit(event)
    }
}
/// A client owns its worker. Poll never performs socket work; drop stops and joins.
pub struct Client {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}
impl Client {
    /// Accept literal IP:port endpoints (including bracketed IPv6), avoiding
    /// unbounded DNS resolution on an Android UI call.
    pub fn connect(address: &str) -> io::Result<Self> {
        let address: SocketAddr = address.parse().map_err(|_| {
            io::Error::other("Enter a PC IP and port, for example 192.168.1.10:5000")
        })?;
        if address.port() == 0 || address.ip().is_unspecified() {
            return Err(io::Error::other(
                "Use a specific PC address and a nonzero port",
            ));
        }
        let shared = Arc::new(Shared {
            events: Mutex::new(VecDeque::new()),
            stopped: AtomicBool::new(false),
        });
        let worker_shared = shared.clone();
        let worker = thread::Builder::new()
            .name("mm-android-network".into())
            .spawn(move || run(worker_shared, address))?;
        Ok(Self {
            shared,
            worker: Some(worker),
        })
    }
    /// Events: 1=UTF-8 status, 2=width/height/fps LE u32, 3=pts LE u64,
    /// keyframe byte and Annex-B H.264; 4=UTF-8 error. Video order is preserved.
    pub fn poll(&self) -> Option<Vec<u8>> {
        self.shared
            .events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pop_front()
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        self.shared.stopped.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn send(stream: &mut TcpStream, message: Message) -> Result<(), Error> {
    let (kind, payload) = message.to_frame_parts()?;
    stream.write_all(&encode_stream_frame(kind, &payload)?)?;
    Ok(())
}
fn run(shared: Arc<Shared>, address: SocketAddr) {
    let mut session_id = None;
    let mut last_seq = 0;
    while !shared.stopped.load(Ordering::Relaxed) {
        shared.text(
            1,
            if session_id.is_some() {
                "Reconnecting…"
            } else {
                "Connecting…"
            },
        );
        if let Err(error) = connection(&shared, address, &mut session_id, &mut last_seq) {
            if shared.stopped.load(Ordering::Relaxed) {
                break;
            }
            // A replacement stream always starts from a fresh config + IDR.
            shared
                .events
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clear();
            shared.text(4, &format!("{error}. Retrying…"));
        }
        for _ in 0..50 {
            if shared.stopped.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
}
fn connection(
    shared: &Shared,
    address: SocketAddr,
    session_id: &mut Option<u64>,
    last_seq: &mut u64,
) -> Result<(), Error> {
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))?;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(Duration::from_millis(100)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let first = match *session_id {
        Some(id) => Message::Resume {
            session_id: id,
            last_seq: *last_seq,
        },
        None => Message::Hello {
            proto_version: PROTOCOL_VERSION,
            device_name: "MightyMonitor Android".into(),
            role: Role::Viewer,
            capabilities: vec!["h264".into()],
        },
    };
    send(&mut stream, first)?;
    let mut decoder = FrameDecoder::new();
    let mut buffer = [0; 64 * 1024];
    let mut configured = false;
    let mut acknowledged = false;
    let mut need_keyframe = true;
    let start = Instant::now();
    let mut feedback_at = start;
    let mut last_message = start;
    let mut bytes = 0_u64;
    let mut nonce = 0_u64;
    let mut pending_ping = None;
    let mut rtt_ms = 0;
    while !shared.stopped.load(Ordering::Relaxed) {
        if (!configured && start.elapsed() > Duration::from_secs(5))
            || last_message.elapsed() > Duration::from_secs(15)
        {
            return Err("Host handshake or video stream timed out".into());
        }
        if configured && feedback_at.elapsed() >= Duration::from_secs(1) {
            let elapsed = feedback_at.elapsed().as_secs_f64();
            send(
                &mut stream,
                Message::NetStats(NetStats {
                    rtt_ms,
                    recv_kbps: (bytes as f64 * 8.0 / 1000.0 / elapsed).min(u32::MAX as f64) as u32,
                    loss_pct: 0.0,
                }),
            )?;
            nonce = nonce.wrapping_add(1);
            let timestamp = start.elapsed().as_micros().min(u64::MAX as u128) as u64;
            send(
                &mut stream,
                Message::Ping {
                    nonce,
                    t_send_us: timestamp,
                },
            )?;
            pending_ping = Some((nonce, timestamp, Instant::now()));
            feedback_at = Instant::now();
            bytes = 0;
        }
        if let Some(frame) = decoder.next_frame()? {
            last_message = Instant::now();
            match Message::from_frame_parts(frame.kind, &frame.payload)? {
                Message::HelloAck {
                    accepted: true,
                    session_id: id,
                } if !acknowledged => {
                    *session_id = Some(id);
                    acknowledged = true;
                }
                Message::HelloAck {
                    accepted: false, ..
                } => {
                    *session_id = None;
                    return Err("Host rejected the session".into());
                }
                Message::VideoConfig {
                    codec: Codec::H264,
                    width,
                    height,
                    fps,
                } if acknowledged => {
                    if width == 0
                        || height == 0
                        || width > 8192
                        || height > 8192
                        || width % 2 != 0
                        || height % 2 != 0
                        || fps == 0
                        || fps > 240
                    {
                        return Err("Invalid H.264 dimensions or frame rate".into());
                    }
                    let mut event = vec![2];
                    for value in [width, height, fps] {
                        event.extend_from_slice(&value.to_le_bytes());
                    }
                    if !shared.emit(event) {
                        break;
                    }
                    configured = true;
                    need_keyframe = true;
                    if !shared.text(1, "Connected — view only") {
                        break;
                    }
                }
                Message::VideoFrame {
                    seq,
                    timestamp_us,
                    keyframe,
                    data,
                } if configured => {
                    bytes = bytes.saturating_add(data.len() as u64);
                    *last_seq = seq;
                    if need_keyframe && !keyframe {
                        continue;
                    }
                    need_keyframe = false;
                    let mut event = vec![3];
                    event.extend_from_slice(&timestamp_us.to_le_bytes());
                    event.push(u8::from(keyframe));
                    event.extend_from_slice(&data);
                    if !shared.emit(event) {
                        break;
                    }
                }
                Message::Ping { nonce, t_send_us } => {
                    send(&mut stream, Message::Pong { nonce, t_send_us })?
                }
                Message::Pong {
                    nonce: received,
                    t_send_us,
                } => {
                    if let Some((expected, timestamp, sent)) = pending_ping {
                        if received == expected && t_send_us == timestamp {
                            rtt_ms = sent.elapsed().as_millis().min(u32::MAX as u128) as u32;
                            pending_ping = None;
                        }
                    }
                }
                Message::Bye { .. } => return Err("Host ended the stream".into()),
                _ => return Err("Unexpected host message or unsupported video format".into()),
            }
            continue;
        }
        match stream.read(&mut buffer) {
            Ok(0) => return Err("Host disconnected".into()),
            Ok(count) => decoder.push(&buffer[..count]),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut
                        | io::ErrorKind::WouldBlock
                        | io::ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_addresses_without_spawning_workers() {
        for value in [
            "",
            "example.org:5000",
            "127.0.0.1:0",
            "0.0.0.0:5000",
            "[::]:5000",
        ] {
            assert!(Client::connect(value).is_err());
        }
    }
    #[test]
    fn full_queue_can_be_cancelled_without_blocking_shutdown() {
        let shared = Arc::new(Shared {
            events: Mutex::new(VecDeque::from([vec![1], vec![1], vec![1]])),
            stopped: AtomicBool::new(false),
        });
        let worker_shared = shared.clone();
        let worker = thread::spawn(move || worker_shared.emit(vec![3; 1024]));
        shared.stopped.store(true, Ordering::Relaxed);
        assert!(!worker.join().unwrap());
        assert_eq!(shared.events.lock().unwrap().len(), QUEUE_CAPACITY);
    }
    #[test]
    fn reconnect_uses_resume_and_discards_frames_before_new_keyframe() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            for attempt in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut decoder = FrameDecoder::new();
                let mut bytes = [0; 1024];
                let first = loop {
                    let count = stream.read(&mut bytes).unwrap();
                    assert!(count > 0);
                    decoder.push(&bytes[..count]);
                    if let Some(frame) = decoder.next_frame().unwrap() {
                        break Message::from_frame_parts(frame.kind, &frame.payload).unwrap();
                    }
                };
                if attempt == 0 {
                    assert!(matches!(first, Message::Hello { .. }));
                } else {
                    assert!(matches!(
                        first,
                        Message::Resume {
                            session_id: 8,
                            last_seq: 1
                        }
                    ));
                }
                for message in [
                    Message::HelloAck {
                        session_id: 8,
                        accepted: true,
                    },
                    Message::VideoConfig {
                        codec: Codec::H264,
                        width: 64,
                        height: 32,
                        fps: 30,
                    },
                ] {
                    send(&mut stream, message).unwrap();
                }
                if attempt == 1 {
                    send(
                        &mut stream,
                        Message::VideoFrame {
                            seq: 2,
                            timestamp_us: 2,
                            keyframe: false,
                            data: vec![99],
                        },
                    )
                    .unwrap();
                }
                send(
                    &mut stream,
                    Message::VideoFrame {
                        seq: if attempt == 0 { 1 } else { 3 },
                        timestamp_us: 3,
                        keyframe: true,
                        data: vec![attempt],
                    },
                )
                .unwrap();
                thread::sleep(Duration::from_millis(300));
            }
        });
        let client = Client::connect(&address.to_string()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(6);
        let mut frames = vec![];
        loop {
            assert!(
                Instant::now() < deadline,
                "reconnect never delivered keyframe: {frames:?}"
            );
            if let Some(event) = client.poll() {
                if event[0] == 3 {
                    assert_eq!(event[9], 1);
                    frames.push(event[10]);
                    if frames.len() == 2 {
                        break;
                    }
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(frames, [0, 1]);
        drop(client);
        server.join().unwrap();
    }
    #[test]
    fn uses_shared_protocol_and_delivers_config_then_keyframe() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut decoder = FrameDecoder::new();
            let mut bytes = [0; 1024];
            loop {
                let count = stream.read(&mut bytes).unwrap();
                decoder.push(&bytes[..count]);
                if let Some(frame) = decoder.next_frame().unwrap() {
                    assert!(matches!(
                        Message::from_frame_parts(frame.kind, &frame.payload).unwrap(),
                        Message::Hello {
                            role: Role::Viewer,
                            ..
                        }
                    ));
                    break;
                }
            }
            for message in [
                Message::HelloAck {
                    session_id: 8,
                    accepted: true,
                },
                Message::VideoConfig {
                    codec: Codec::H264,
                    width: 64,
                    height: 32,
                    fps: 30,
                },
                Message::VideoFrame {
                    seq: 1,
                    timestamp_us: 7,
                    keyframe: true,
                    data: vec![0, 0, 0, 1, 0x65],
                },
            ] {
                send(&mut stream, message).unwrap();
            }
            thread::sleep(Duration::from_millis(500));
        });
        let client = Client::connect(&address.to_string()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut config = false;
        loop {
            assert!(
                Instant::now() < deadline,
                "Android bridge never delivered video"
            );
            if let Some(event) = client.poll() {
                if event[0] == 2 {
                    assert_eq!(&event[1..5], &64_u32.to_le_bytes());
                    config = true;
                }
                if event[0] == 3 {
                    assert!(config);
                    assert_eq!(&event[1..9], &7_u64.to_le_bytes());
                    assert_eq!(&event[10..], &[0, 0, 0, 1, 0x65]);
                    break;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
        drop(client);
        server.join().unwrap();
    }
}
