//! Authorization and disconnect cleanup exercised over real framed sockets.
use mightymonitor_core::{
    Bytes, Capture, Codec, CodecError, Encoder, InputEvent, Message, Role, TcpTransport, Transport,
    PROTOCOL_VERSION,
};
use mightymonitor_host::{
    input::{control_consent, ControlConsent, InputTarget},
    pipeline::{HostEncoder, Pipeline},
    session::{serve_with_input, serve_with_revocable_input, SessionConfig},
    Error,
};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
struct Source;
impl Capture for Source {
    fn next_frame(&mut self) -> Option<Bytes> {
        Some(vec![0; 16])
    }
}
struct Encode;
impl Encoder for Encode {
    fn encode(&mut self, b: &[u8]) -> Result<Bytes, CodecError> {
        Ok(b.into())
    }
    fn set_bitrate_kbps(&mut self, _: u32) -> Result<(), CodecError> {
        Ok(())
    }
}
impl HostEncoder for Encode {
    fn encode_video(&mut self, b: &[u8], _: u64, key: bool) -> Result<(Bytes, bool), Error> {
        Ok((b.into(), key))
    }
}
struct Recorder {
    events: Arc<Mutex<Vec<InputEvent>>>,
    released: Arc<AtomicUsize>,
}
impl InputTarget for Recorder {
    fn apply(&mut self, e: &InputEvent) -> Result<(), Error> {
        self.events.lock().unwrap().push(e.clone());
        Ok(())
    }
    fn release_all(&mut self) {
        self.released.fetch_add(1, Ordering::SeqCst);
    }
}
async fn send(t: &mut TcpTransport, m: Message) {
    let (k, p) = m.to_frame_parts().unwrap();
    t.send(k, &p).await.unwrap();
}
async fn read(t: &mut TcpTransport) -> Message {
    let f = tokio::time::timeout(Duration::from_secs(2), t.recv())
        .await
        .unwrap()
        .unwrap();
    Message::from_frame_parts(f.kind, &f.payload).unwrap()
}
async fn until_status(t: &mut TcpTransport, enabled: bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Message::ControlStatus { enabled: actual } = read(t).await {
                assert_eq!(actual, enabled);
                return;
            }
        }
    })
    .await
    .expect("Control status deadline");
}
async fn start(
    allowed: bool,
) -> (
    TcpTransport,
    Arc<Mutex<Vec<InputEvent>>>,
    Arc<AtomicUsize>,
    tokio::task::JoinHandle<Result<(), Error>>,
) {
    start_with_consent(allowed, None).await
}
async fn start_with_consent(
    allowed: bool,
    consent: Option<ControlConsent>,
) -> (
    TcpTransport,
    Arc<Mutex<Vec<InputEvent>>>,
    Arc<AtomicUsize>,
    tokio::task::JoinHandle<Result<(), Error>>,
) {
    start_with_options(allowed, consent, None).await
}
async fn start_with_options(
    allowed: bool,
    consent: Option<ControlConsent>,
    pause: Option<Arc<WritePause>>,
) -> (
    TcpTransport,
    Arc<Mutex<Vec<InputEvent>>>,
    Arc<AtomicUsize>,
    tokio::task::JoinHandle<Result<(), Error>>,
) {
    let expected_available = allowed && consent.as_ref().is_none_or(ControlConsent::is_allowed);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let events = Arc::new(Mutex::new(vec![]));
    let released = Arc::new(AtomicUsize::new(0));
    let backend = if allowed {
        Some(Box::new(Recorder {
            events: events.clone(),
            released: released.clone(),
        }) as Box<dyn InputTarget>)
    } else {
        None
    };
    let task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await?;
        let inner = TcpTransport::new(stream)?;
        let transport: Box<dyn Transport> = match pause {
            Some(pause) => Box::new(PausingTransport { inner, pause }),
            None => Box::new(inner),
        };
        let pipeline = Pipeline::new(Source, Encode, 30)?;
        let config = SessionConfig {
            session_id: 42,
            codec: Codec::Raw,
            width: 2,
            height: 2,
            fps: 30,
            write_timeout: Duration::from_secs(1),
            handshake_timeout: Duration::from_secs(1),
            idle_timeout: Duration::from_secs(3),
        };
        if let Some(consent) = consent {
            serve_with_revocable_input(transport, pipeline, config, backend, consent).await
        } else {
            serve_with_input(transport, pipeline, config, backend).await
        }
    });
    let mut peer = TcpTransport::connect(address).await.unwrap();
    send(
        &mut peer,
        Message::Hello {
            proto_version: PROTOCOL_VERSION,
            device_name: "test".into(),
            role: Role::Viewer,
            capabilities: vec!["input-v1".into()],
        },
    )
    .await;
    assert!(
        matches!(read(&mut peer).await,Message::HelloAckV2{control_available,..} if control_available==expected_available)
    );
    assert!(matches!(read(&mut peer).await, Message::VideoConfig { .. }));
    (peer, events, released, task)
}
#[tokio::test]
async fn view_permission_does_not_authorize_input() {
    let (mut peer, events, _, task) = start(false).await;
    send(&mut peer, Message::ControlRequest { enabled: true }).await;
    until_status(&mut peer, false).await;
    send(
        &mut peer,
        Message::Input(InputEvent::Key {
            code: 4,
            pressed: true,
        }),
    )
    .await;
    assert!(tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    assert!(events.lock().unwrap().is_empty());
}
#[tokio::test]
async fn input_requires_both_grants_and_releases_on_revoke_disconnect() {
    let (mut peer, events, released, task) = start(true).await;
    send(&mut peer, Message::ControlRequest { enabled: true }).await;
    until_status(&mut peer, true).await;
    send(
        &mut peer,
        Message::Input(InputEvent::Key {
            code: 4,
            pressed: true,
        }),
    )
    .await;
    send(&mut peer, Message::ControlRequest { enabled: false }).await;
    until_status(&mut peer, false).await;
    assert_eq!(
        *events.lock().unwrap(),
        [InputEvent::Key {
            code: 4,
            pressed: true
        }]
    );
    let revocations = released.load(Ordering::SeqCst);
    assert!(revocations >= 2);
    drop(peer);
    let _ = tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap();
    assert!(released.load(Ordering::SeqCst) > revocations);
}
#[tokio::test]
async fn malformed_input_closes_the_grant_and_releases_held_state() {
    let (mut peer, events, released, task) = start(true).await;
    send(&mut peer, Message::ControlRequest { enabled: true }).await;
    until_status(&mut peer, true).await;
    send(
        &mut peer,
        Message::Input(InputEvent::MouseMove {
            x: f32::NAN,
            y: 0.0,
        }),
    )
    .await;
    let _ = tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap();
    assert!(events.lock().unwrap().is_empty());
    assert!(released.load(Ordering::SeqCst) >= 2);
}

async fn roundtrip(peer: &mut TcpTransport, nonce: u64) {
    send(
        peer,
        Message::Ping {
            nonce,
            t_send_us: 0,
        },
    )
    .await;
    loop {
        if let Message::Pong { nonce: actual, .. } = read(peer).await {
            assert_eq!(actual, nonce);
            break;
        }
    }
}
async fn assert_video_continues(peer: &mut TcpTransport) {
    loop {
        if matches!(read(peer).await, Message::VideoFrame { .. }) {
            break;
        }
    }
}
#[tokio::test]
async fn local_revocation_releases_all_viewers_and_keeps_video() {
    let (owner, consent) = control_consent(true);
    let (mut first, first_events, first_released, first_task) =
        start_with_consent(true, Some(consent.clone())).await;
    let (mut second, second_events, second_released, second_task) =
        start_with_consent(true, Some(consent.clone())).await;
    for peer in [&mut first, &mut second] {
        send(peer, Message::ControlRequest { enabled: true }).await;
        until_status(peer, true).await;
        send(
            peer,
            Message::Input(InputEvent::Key {
                code: 4,
                pressed: true,
            }),
        )
        .await;
        roundtrip(peer, 100).await;
    }
    owner.revoke();
    for peer in [&mut first, &mut second] {
        until_status(peer, false).await;
        // Packets already in flight cannot inject input, re-grant or stop video.
        send(
            peer,
            Message::Input(InputEvent::Key {
                code: 5,
                pressed: true,
            }),
        )
        .await;
        send(peer, Message::ControlRequest { enabled: true }).await;
        until_status(peer, false).await;
        roundtrip(peer, 101).await;
        assert_video_continues(peer).await;
    }
    assert_eq!(first_events.lock().unwrap().len(), 1);
    assert_eq!(second_events.lock().unwrap().len(), 1);
    assert!(first_released.load(Ordering::SeqCst) >= 1);
    assert!(second_released.load(Ordering::SeqCst) >= 1);
    // Reconnecting with the same share lifetime cannot restore local permission.
    let (mut later, later_events, _, later_task) = start_with_consent(true, Some(consent)).await;
    send(&mut later, Message::ControlRequest { enabled: true }).await;
    until_status(&mut later, false).await;
    assert_video_continues(&mut later).await;
    assert!(later_events.lock().unwrap().is_empty());
    for peer in [&mut first, &mut second, &mut later] {
        send(
            peer,
            Message::Bye {
                reason: "done".into(),
            },
        )
        .await;
    }
    for task in [first_task, second_task, later_task] {
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}
#[tokio::test]
async fn losing_local_owner_fails_closed_without_stopping_video() {
    let (owner, consent) = control_consent(true);
    let (mut peer, events, released, task) = start_with_consent(true, Some(consent)).await;
    send(&mut peer, Message::ControlRequest { enabled: true }).await;
    until_status(&mut peer, true).await;
    send(
        &mut peer,
        Message::Input(InputEvent::Key {
            code: 4,
            pressed: true,
        }),
    )
    .await;
    roundtrip(&mut peer, 102).await;
    drop(owner);
    until_status(&mut peer, false).await;
    assert_eq!(events.lock().unwrap().len(), 1);
    assert!(released.load(Ordering::SeqCst) >= 1);
    assert_video_continues(&mut peer).await;
    send(
        &mut peer,
        Message::Bye {
            reason: "done".into(),
        },
    )
    .await;
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[derive(Default)]
struct WritePause {
    armed: std::sync::atomic::AtomicBool,
    entered: tokio::sync::Notify,
    resume: tokio::sync::Notify,
}
struct PausingTransport {
    inner: TcpTransport,
    pause: Arc<WritePause>,
}
#[async_trait::async_trait]
impl Transport for PausingTransport {
    async fn send(
        &mut self,
        kind: u8,
        payload: &[u8],
    ) -> Result<(), mightymonitor_core::TransportError> {
        if kind == 4 && self.pause.armed.swap(false, Ordering::SeqCst) {
            self.pause.entered.notify_one();
            self.pause.resume.notified().await;
        }
        self.inner.send(kind, payload).await
    }
    async fn recv(
        &mut self,
    ) -> Result<mightymonitor_core::Frame, mightymonitor_core::TransportError> {
        self.inner.recv().await
    }
}
#[tokio::test]
async fn revocation_releases_held_input_during_a_blocked_video_write() {
    let (owner, consent) = control_consent(true);
    let pause = Arc::new(WritePause::default());
    let (mut peer, events, released, task) =
        start_with_options(true, Some(consent), Some(pause.clone())).await;
    send(&mut peer, Message::ControlRequest { enabled: true }).await;
    until_status(&mut peer, true).await;
    send(
        &mut peer,
        Message::Input(InputEvent::Key {
            code: 4,
            pressed: true,
        }),
    )
    .await;
    roundtrip(&mut peer, 103).await;
    assert_eq!(events.lock().unwrap().len(), 1);
    let before = released.load(Ordering::SeqCst);
    pause.armed.store(true, Ordering::SeqCst);
    tokio::time::timeout(Duration::from_secs(2), pause.entered.notified())
        .await
        .unwrap();
    owner.revoke();
    // Local release must not wait for a blocked network write or another input packet.
    tokio::time::timeout(Duration::from_secs(2), async {
        while released.load(Ordering::SeqCst) == before {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    pause.resume.notify_one();
    until_status(&mut peer, false).await;
    assert_video_continues(&mut peer).await;
    send(
        &mut peer,
        Message::Bye {
            reason: "done".into(),
        },
    )
    .await;
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
