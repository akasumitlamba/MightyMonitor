//! Thin egui presentation layer.
#[cfg(any(target_os = "linux", target_os = "windows"))]
use crate::net::ViewerSession;
use crate::net::{Output, SharedOutput, State};
use eframe::egui;
use std::sync::{Arc, Mutex};

pub struct App {
    address: String,
    share_address: String,
    share_fps: u32,
    share_bitrate: u32,
    trusted_network: bool,
    host: Option<crate::host::HostProcess>,
    host_message: String,
    last_host_poll: std::time::Instant,
    output: SharedOutput,
    texture: Option<egui::TextureHandle>,
    task: Option<tokio::task::JoinHandle<()>>,
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    runtime: tokio::runtime::Runtime,
}
impl App {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            address: "127.0.0.1:5000".into(),
            share_address: "127.0.0.1:5000".into(),
            share_fps: 30,
            share_bitrate: 4000,
            trusted_network: false,
            host: None,
            host_message: String::new(),
            last_host_poll: std::time::Instant::now(),
            output: Arc::new(Mutex::new(Output {
                state: State::Closed,
                ..Default::default()
            })),
            texture: None,
            task: None,
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            runtime: tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()?,
        })
    }
    fn disconnect(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.texture = None;
        self.output = Arc::new(Mutex::new(Output {
            state: State::Closed,
            ..Default::default()
        }));
    }
    fn connect(&mut self) {
        self.disconnect();
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        match codec_gst::decoder::Decoder::new() {
            Ok(decoder) => {
                let session = ViewerSession::new(decoder);
                self.output = session.output.clone();
                self.task = Some(self.runtime.spawn(session.run(self.address.clone())));
            }
            Err(e) => {
                let mut output = self.output.lock().unwrap_or_else(|p| p.into_inner());
                output.state = State::Closed;
                output.error = Some(e.to_string());
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let mut output = self.output.lock().unwrap_or_else(|p| p.into_inner());
            output.state = State::Closed;
            output.error =
                Some("Video decoding is currently supported on Linux and Windows.".into());
        }
    }
}
impl Drop for App {
    fn drop(&mut self) {
        if let Some(t) = &self.task {
            t.abort();
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if self.last_host_poll.elapsed() >= std::time::Duration::from_millis(500) {
            self.last_host_poll = std::time::Instant::now();
            if let Some(host) = &mut self.host {
                match host.exited() {
                    Ok(true) => {
                        self.host_message = format!(
                            "Sharing stopped. {}",
                            host.diagnostics().unwrap_or_default()
                        );
                        self.host = None;
                    }
                    Err(error) => {
                        self.host_message = format!("Unable to check sharing: {error}");
                        self.host = None;
                    }
                    Ok(false) => {
                        let diagnostics = host.diagnostics().unwrap_or_default();
                        self.host_message = if diagnostics.contains("mm-host listening on") {
                            format!("Sharing on {} — one viewer at a time", self.share_address)
                        } else {
                            "Starting sharing…".into()
                        };
                    }
                }
            }
        }
        egui::TopBottomPanel::top("brand").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.heading("MightyMonitor");
            ui.label("Your screens, connected.");
            ui.add_space(8.0);
        });
        egui::SidePanel::left("controls").resizable(false).default_width(300.0).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(12.0);
            ui.heading("View another screen");
            ui.label("Enter the sharing device’s IP and port");
            ui.text_edit_singleline(&mut self.address);
            ui.horizontal(|ui| {
                if ui.button("Connect").clicked() { self.connect(); }
                if ui.button("Disconnect").clicked() { self.disconnect(); }
            });
            ui.add_space(18.0);
            ui.separator();
            ui.add_space(12.0);
            ui.heading("Share this screen");
            if cfg!(any(target_os = "linux", target_os = "windows")) {
                ui.label(if cfg!(target_os = "windows") { "Windows desktop • screen viewing only" } else { "Linux X11 • screen viewing only" });
                ui.label("Local IP and port to listen on");
                ui.add_enabled(self.host.is_none(), egui::TextEdit::singleline(&mut self.share_address));
                ui.small("127.0.0.1 is this computer only. Use your LAN IP to share with another device.");
                ui.add_enabled(self.host.is_none(), egui::Slider::new(&mut self.share_fps, 1..=60).text("fps"));
                ui.add_enabled(self.host.is_none(), egui::Slider::new(&mut self.share_bitrate, 500..=20_000).text("kbit/s"));
                ui.label("Connections are not encrypted or password protected. Only share on a network you trust.");
                ui.add_enabled(self.host.is_none(), egui::Checkbox::new(&mut self.trusted_network, "I’m using a trusted network"));
                if ui.add_enabled(self.host.is_none() && self.trusted_network, egui::Button::new("Start sharing")).clicked() {
                    match crate::host::HostProcess::start(&self.share_address, self.share_fps, self.share_bitrate) {
                        Ok(host) => { self.host = Some(host); self.host_message = "Starting sharing…".into(); }
                        Err(error) => self.host_message = error.to_string(),
                    }
                }
                if ui.add_enabled(self.host.is_some(), egui::Button::new("Stop sharing")).clicked() {
                    self.host = None;
                    self.host_message = "Sharing stopped".into();
                }
                ui.label(&self.host_message);
            } else {
                ui.label("Screen sharing requires Linux or Windows.");
            }
            ui.add_space(18.0);
            ui.separator();
            ui.small("Remote control, extended displays and Android support are under development.");
            });
        });
        let mut out = self.output.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(p) = out.picture.take() {
            if let Some(rgb) = crate::render::rgb(&p) {
                let image = egui::ColorImage::from_rgb([p.width as usize, p.height as usize], &rgb);
                match &mut self.texture {
                    Some(t) => t.set(image, egui::TextureOptions::LINEAR),
                    None => {
                        self.texture =
                            Some(ctx.load_texture("video", image, egui::TextureOptions::LINEAR))
                    }
                }
                out.stats.present_ms = p.decoded.elapsed().as_secs_f64() * 1000.0;
                eprintln!("decode->present submission: {:.2} ms", out.stats.present_ms);
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(match out.state {
                State::Connecting => "Connecting…",
                State::Connected => "Connected",
                State::Reconnecting => "Reconnecting…",
                State::Closed => "Not connected",
            });
            if out.state == State::Connected {
                ui.collapsing("Connection details", |ui| {
                    ui.label(format!(
                        "{:.1} fps · {} kbit/s · response {} ms",
                        out.stats.fps, out.stats.kbps, out.stats.rtt_ms
                    ));
                });
            }
            if let Some(e) = &out.error {
                ui.colored_label(egui::Color32::RED, e);
            }
            if self.texture.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.label("Connect to a sharing device to see its screen here.");
                });
            }
            if let Some(t) = &self.texture {
                ui.add(egui::Image::new(t).fit_to_exact_size({
                    let size = t.size_vec2();
                    size * (ui.available_width() / size.x)
                        .min(ui.available_height() / size.y)
                        .max(0.01)
                }));
            }
        });
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}
