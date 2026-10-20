//! Desktop capture / GStreamer host executable.
#![forbid(unsafe_code)]
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod desktop {
    use codec_gst::encoder::{Encoder as GstEncoder, EncoderConfig, PixelFormat};
    #[cfg(target_os = "linux")]
    use host_linux::{LinuxCapture, X11Capture};
    #[cfg(target_os = "linux")]
    use mightymonitor_core::Capture;
    use mightymonitor_core::{Bytes, CodecError, Encoder};
    use mightymonitor_host::{
        pipeline::{HostEncoder, Pipeline},
        session::{serve_with_revocable_input, SessionConfig},
        Error,
    };
    use std::time::Duration;

    #[cfg(target_os = "linux")]
    struct RgbaCapture(LinuxCapture<X11Capture>);
    #[cfg(target_os = "linux")]
    impl Capture for RgbaCapture {
        fn next_frame(&mut self) -> Option<Bytes> {
            let mut bytes = self.0.next_frame()?;
            for pixel in bytes.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
            Some(bytes)
        }
    }
    #[cfg(target_os = "linux")]
    fn x11_capture(monitor: i32) -> Result<(RgbaCapture, u32, u32), Error> {
        if monitor != -1 {
            return Err("--monitor is currently Windows-only".into());
        }
        let source = X11Capture::open(None)?;
        let (width, height) = (u32::from(source.width()), u32::from(source.height()));
        Ok((RgbaCapture(LinuxCapture::new(source)), width, height))
    }
    #[cfg(target_os = "windows")]
    fn capture(
        fps: u32,
        monitor: i32,
    ) -> Result<(codec_gst::capture::WindowsCapture, u32, u32), Error> {
        let source = codec_gst::capture::WindowsCapture::open(fps, monitor)?;
        let (width, height) = source.dimensions();
        Ok((source, width, height))
    }
    #[cfg(target_os = "linux")]
    enum CaptureFactory {
        X11,
        #[cfg(feature = "pipewire")]
        Wayland {
            capture: host_linux::SharedCapture,
            input: Option<host_linux::PortalInput>,
        },
    }
    #[cfg(target_os = "linux")]
    struct DesktopCapture(Box<dyn Capture + Send>);
    #[cfg(target_os = "linux")]
    impl Capture for DesktopCapture {
        fn next_frame(&mut self) -> Option<Bytes> {
            self.0.next_frame()
        }
    }
    #[cfg(target_os = "linux")]
    impl CaptureFactory {
        fn open(mode: &str, allow_control: bool, extend: bool) -> Result<Self, Error> {
            #[cfg(not(feature = "pipewire"))]
            let _ = allow_control;
            let wayland = match mode {
                "auto" => {
                    std::env::var_os("WAYLAND_DISPLAY").is_some()
                        || std::env::var("XDG_SESSION_TYPE").is_ok_and(|s| s == "wayland")
                }
                "wayland" => true,
                "x11" => false,
                _ => return Err("--capture must be auto, x11 or wayland".into()),
            };
            if !wayland {
                if extend {
                    return Err("Extended displays currently require a Wayland desktop with virtual-monitor portal support".into());
                }
                return Ok(Self::X11);
            }
            #[cfg(feature = "pipewire")]
            {
                eprintln!("Choose a screen in the system sharing dialog.");
                let mut portal =
                    host_linux::PipeWireCapture::open_with_options(allow_control, extend)?;
                let input = portal.input();
                let shared = host_linux::SharedCapture::start(move |timeout| {
                    portal.poll_captured(timeout).map_err(std::io::Error::other)
                })?;
                Ok(Self::Wayland {
                    capture: shared,
                    input,
                })
            }
            #[cfg(not(feature = "pipewire"))]
            {
                Err("This build has no Wayland capture. Install a PipeWire-enabled build, or use --capture x11 in an X11 session.".into())
            }
        }
        fn input(
            &self,
            width: u32,
            height: u32,
        ) -> Result<Option<Box<dyn mightymonitor_host::input::InputTarget>>, Error> {
            match self {
                Self::X11 => Ok(Some(Box::new(
                    mightymonitor_host::input::desktop::DesktopInput::new(width, height)?,
                ))),
                #[cfg(feature = "pipewire")]
                Self::Wayland { input, .. } => Ok(input.clone().map(|input| {
                    Box::new(input) as Box<dyn mightymonitor_host::input::InputTarget>
                })),
            }
        }
        fn capture(&self, _fps: u32, monitor: i32) -> Result<(DesktopCapture, u32, u32), Error> {
            if monitor != -1 {
                return Err(
                    "--monitor is Windows-only; select your Wayland screen in the system dialog"
                        .into(),
                );
            }
            match self {
                Self::X11 => {
                    let (capture, w, h) = x11_capture(monitor)?;
                    Ok((DesktopCapture(Box::new(capture)), w, h))
                }
                #[cfg(feature = "pipewire")]
                Self::Wayland {
                    capture: shared, ..
                } => {
                    let (capture, w, h) = shared.viewer()?;
                    Ok((DesktopCapture(Box::new(capture)), w, h))
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    struct CaptureFactory {
        display: Option<mightymonitor_display_windows::Display>,
    }
    #[cfg(target_os = "windows")]
    impl CaptureFactory {
        fn open(mode: &str, _allow_control: bool, extend: bool) -> Result<Self, Error> {
            if extend {
                return Err("Windows extended displays require a virtual display driver, which is not included yet".into());
            }
            if mode != "auto" {
                return Err("Windows uses DXGI capture; --capture must be auto".into());
            }
            Ok(Self { display: None })
        }
        fn input(
            &self,
            width: u32,
            height: u32,
        ) -> Result<Option<Box<dyn mightymonitor_host::input::InputTarget>>, Error> {
            Ok(Some(Box::new(
                mightymonitor_host::input::desktop::DesktopInput::new(width, height)?,
            )))
        }
        fn capture(
            &self,
            fps: u32,
            monitor: i32,
        ) -> Result<(codec_gst::capture::WindowsCapture, u32, u32), Error> {
            if let Some(display) = &self.display {
                let source = codec_gst::capture::WindowsCapture::open_display(fps, display)?;
                let (width, height) = source.dimensions();
                Ok((source, width, height))
            } else {
                capture(fps, monitor)
            }
        }
    }
    struct Codec(GstEncoder);
    impl Encoder for Codec {
        fn encode(&mut self, raw: &[u8]) -> Result<Bytes, CodecError> {
            self.0.encode(raw)
        }
        fn set_bitrate_kbps(&mut self, kbps: u32) -> Result<(), CodecError> {
            Encoder::set_bitrate_kbps(&mut self.0, kbps)
        }
    }
    impl HostEncoder for Codec {
        fn encode_video(
            &mut self,
            raw: &[u8],
            timestamp_us: u64,
            force: bool,
        ) -> Result<(Bytes, bool), Error> {
            if force {
                self.0.force_keyframe()?;
            }
            let frame = self.0.encode_frame(raw, timestamp_us)?;
            Ok((frame.data, frame.is_keyframe))
        }
    }
    pub async fn run() -> Result<(), Error> {
        #[cfg(target_os = "windows")]
        let _ = enigo::set_dpi_awareness();
        let mut secure = false;
        let mut allow_control = false;
        let mut extend = false;
        let mut managed = false;
        let mut discoverable = false;
        let mut capture_mode = "auto".to_string();
        let mut bind = "127.0.0.1:5000".to_string();
        let mut fps = 30;
        let mut monitor: i32 = -1;
        let mut display_device: Option<String> = None;
        let mut bitrate = 4000;
        let mut idle_timeout_secs = 30;
        let mut args = std::env::args().skip(1);
        while let Some(flag) = args.next() {
            if flag == "--secure" {
                secure = true;
                continue;
            }
            if flag == "--allow-control" {
                allow_control = true;
                continue;
            }
            if flag == "--extend" {
                extend = true;
                continue;
            }
            if flag == "--discoverable" {
                discoverable = true;
                continue;
            }
            if flag == "--managed" {
                managed = true;
                continue;
            }
            if flag == "--list-displays" {
                #[cfg(target_os = "windows")]
                {
                    for display in mightymonitor_display_windows::displays()? {
                        println!(
                            "{}  {} × {}{}",
                            display.device(),
                            display.width,
                            display.height,
                            if display.primary { " (Main)" } else { "" }
                        );
                    }
                    return Ok(());
                }
                #[cfg(not(target_os = "windows"))]
                return Err("--list-displays requires Windows".into());
            }
            if flag == "--help" {
                println!("mm-host [--secure] [--discoverable] [--allow-control] [--extend] [--capture auto|x11|wayland] [--bind ADDRESS:PORT] [--fps 1..240] [--bitrate KBPS] [--idle-timeout-secs SECONDS] [--monitor -1|INDEX] [--display DEVICE] [--list-displays]");
                return Ok(());
            }
            let value = args.next().ok_or("missing flag value")?;
            match flag.as_str() {
                "--bind" => bind = value,
                "--capture" => capture_mode = value,
                "--fps" => fps = value.parse()?,
                "--monitor" => monitor = value.parse()?,
                "--display" => display_device = Some(value),
                "--bitrate" => bitrate = value.parse()?,
                "--idle-timeout-secs" => idle_timeout_secs = value.parse()?,
                _ => return Err(format!("unknown flag: {flag}").into()),
            }
        }
        if monitor < -1
            || idle_timeout_secs == 0
            || !(1..=240).contains(&fps)
            || !(1..=1_000_000).contains(&bitrate)
        {
            return Err("invalid fps, bitrate or idle timeout".into());
        }
        // Obtain Wayland system consent before publishing an invitation or starting
        // network handshake deadlines. The portal lives for this sharing lifetime.
        if allow_control && (!secure || monitor != -1 || display_device.is_some()) {
            return Err("Remote control requires --secure and the primary/default screen".into());
        }
        if discoverable && !secure {
            return Err("Discovery requires secure sharing".into());
        }
        if display_device.is_some() && (monitor != -1 || extend) {
            return Err("--display cannot be combined with --monitor or --extend".into());
        }
        #[cfg(not(target_os = "windows"))]
        if display_device.is_some() {
            return Err("--display requires Windows".into());
        }
        let factory = CaptureFactory::open(&capture_mode, allow_control, extend)?;
        #[cfg(target_os = "windows")]
        let factory = {
            let mut factory = factory;
            if let Some(device) = display_device {
                factory.display = Some(mightymonitor_display_windows::select(&device)?);
            }
            factory
        };
        let (control_owner, control_consent) =
            mightymonitor_host::input::control_consent(allow_control);
        let mut stop = Box::pin(async move {
            // Keep the sender alive for unmanaged CLI hosts as well.
            let control_owner = control_owner;
            if managed {
                let (tx, rx) = tokio::sync::oneshot::channel();
                std::thread::spawn(move || {
                    use std::io::{BufRead, Read};
                    let stdin = std::io::stdin();
                    let mut input = stdin.lock();
                    loop {
                        let mut line = String::new();
                        // This local pipe accepts bounded commands, never launch text.
                        match input.by_ref().take(64).read_line(&mut line) {
                            Ok(0) | Err(_) => break,
                            Ok(_) if line == "REVOKE_CONTROL\n" => {
                                control_owner.revoke();
                            }
                            _ => break, // STOP, an unknown command or an oversized line stops sharing.
                        }
                    }
                    let _ = tx.send(());
                });
                let _ = rx.await;
            } else {
                std::future::pending::<()>().await;
            }
        });
        let listener = tokio::net::TcpListener::bind(&bind).await?;
        let identity = if secure {
            Some(std::sync::Arc::new(
                mightymonitor_security::ShareIdentity::generate()?,
            ))
        } else {
            None
        };
        if let Some(identity) = &identity {
            let invitation = identity.invitation(listener.local_addr()?)?;
            // stdout is an explicit invitation channel; never write it to diagnostics.
            println!("MM_INVITE {}", invitation.to_link());
        }
        if !secure && !listener.local_addr()?.ip().is_loopback() {
            eprintln!(
                "WARNING: non-loopback bind exposes streaming without authentication or encryption"
            );
        }
        let _advertisement = if discoverable {
            match mightymonitor_discovery::Advertisement::new(
                listener.local_addr()?,
                std::env::consts::OS,
                extend,
            ) {
                Ok(service) => Some(service),
                Err(error) => {
                    eprintln!("Local discovery unavailable: {error}");
                    None
                }
            }
        } else {
            None
        };
        eprintln!("mm-host listening on {}", listener.local_addr()?);
        let factory = std::sync::Arc::new(factory);
        let viewers = std::sync::Arc::new(tokio::sync::Semaphore::new(4));
        let controller = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
        let mut tasks = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                _=&mut stop=>{tasks.shutdown().await;return Ok(());},
                finished=tasks.join_next(),if !tasks.is_empty()=>{match finished {Some(Ok(Err(error)))=>eprintln!("viewer session ended: {error}"),Some(Err(error))=>eprintln!("viewer task ended: {error}"),_=>{}}},
                peer=listener.accept()=>{
                    let(stream,_)=peer?;
                    let Ok(permit)=viewers.clone().try_acquire_owned() else {drop(stream);continue;};
                    let identity=identity.clone();let factory=factory.clone();let controller=controller.clone();
                    let consent=control_consent.clone();
                    tasks.spawn(async move {
                        let _permit=permit;
                        let transport:Box<dyn mightymonitor_core::Transport>=if let Some(identity)=identity {Box::new(mightymonitor_core::FramedTransport::from_stream(mightymonitor_security::asynchronous::accept(stream,&identity).await?))} else {Box::new(mightymonitor_core::TcpTransport::new(stream)?)};
                        let capture_consent=consent.clone();
                        let (pipeline,config,input)=tokio::task::spawn_blocking(move ||->Result<_,Error>{
                            let(capture,width,height)=factory.capture(fps,monitor)?;
                            let encoder=Codec(GstEncoder::new(EncoderConfig{width,height,fps,bitrate_kbps:bitrate,format:PixelFormat::Rgba,..Default::default()})?);
                            let pipeline=Pipeline::new(capture,encoder,fps)?.with_bitrate(bitrate)?;
                            let input=if allow_control && capture_consent.is_allowed() {factory.input(width,height)?.map(|backend|Box::new(mightymonitor_host::input::ExclusiveInput::new(backend,controller)) as Box<dyn mightymonitor_host::input::InputTarget>)}else{None};
                            let config=SessionConfig{session_id:1,codec:mightymonitor_core::Codec::H264,width,height,fps,write_timeout:Duration::from_secs(2),handshake_timeout:Duration::from_secs(5),idle_timeout:Duration::from_secs(idle_timeout_secs)};
                            Ok((pipeline,config,input))
                        }).await??;
                        serve_with_revocable_input(transport,pipeline,config,input,consent).await
                    });
                }
            }
        }
    }
}
#[cfg(any(target_os = "linux", target_os = "windows"))]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), mightymonitor_host::Error> {
    desktop::run().await
}
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn main() {
    eprintln!("mm-host capture currently requires Linux X11 or Windows with GStreamer");
}
