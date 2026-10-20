//! Thin egui presentation layer.
#[cfg(any(target_os = "linux", target_os = "windows"))]
use crate::net::ViewerSession;
use crate::net::{Output, SharedOutput, State};
use eframe::egui;
use std::sync::{Arc, Mutex};

#[derive(PartialEq)]
enum Page {
    View,
    Share,
}

pub struct App {
    page: Page,
    themed: bool,
    fullscreen: bool,
    show_keyboard: bool,
    keyboard_text: String,
    keyboard_editor: Option<egui::Id>,
    address: String,
    share_address: String,
    share_fps: u32,
    share_bitrate: u32,
    allow_legacy: bool,
    allow_control: bool,
    extend: bool,
    discoverable: bool,
    #[cfg(target_os = "windows")]
    screens: Vec<mightymonitor_display_windows::Display>,
    #[cfg(target_os = "windows")]
    screen: Option<String>,
    #[cfg(target_os = "windows")]
    screen_error: Option<String>,
    discovery: Option<mightymonitor_discovery::Browser>,
    discovery_error: Option<String>,
    networks: Vec<mightymonitor_discovery::LocalAddress>,
    commands: Option<tokio::sync::mpsc::Sender<crate::net::Command>>,
    input_focused: bool,
    input_modifiers: egui::Modifiers,
    host: Option<crate::host::HostProcess>,
    host_message: String,
    share_qr: Option<(String, qrcode::QrCode)>,
    host_error: bool,
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
            page: Page::View,
            themed: false,
            fullscreen: false,
            show_keyboard: false,
            keyboard_text: String::new(),
            keyboard_editor: None,
            address: String::new(),
            share_address: local_endpoint(),
            share_fps: 30,
            share_bitrate: 4000,
            allow_legacy: false,
            allow_control: false,
            extend: false,
            discoverable: false,
            #[cfg(target_os = "windows")]
            screens: mightymonitor_display_windows::displays().unwrap_or_default(),
            #[cfg(target_os = "windows")]
            screen: None,
            #[cfg(target_os = "windows")]
            screen_error: None,
            discovery: None,
            discovery_error: None,
            networks: mightymonitor_discovery::local_addresses().unwrap_or_default(),
            commands: None,
            input_focused: false,
            input_modifiers: egui::Modifiers::NONE,
            host: None,
            host_message: String::new(),
            share_qr: None,
            host_error: false,
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
        self.commands = None;
        self.input_focused = false;
        self.input_modifiers = egui::Modifiers::NONE;
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.texture = None;
        self.output = Arc::new(Mutex::new(Output {
            state: State::Closed,
            ..Default::default()
        }));
    }
    fn set_fullscreen(&mut self, ctx: &egui::Context, enabled: bool) {
        if self.fullscreen == enabled {
            return;
        }
        self.command(crate::net::ControlAction::Input(
            mightymonitor_core::InputEvent::ReleaseAll,
        ));
        let enabled =
            enabled && self.task.is_some() && self.texture.is_some() && self.page == Page::View;
        self.fullscreen = enabled;
        self.input_focused = false;
        self.input_modifiers = egui::Modifiers::NONE;
        self.close_keyboard(ctx);
        ctx.memory_mut(|memory| {
            if let Some(id) = memory.focused() {
                memory.surrender_focus(id);
            }
        });
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(enabled));
    }
    fn close_keyboard(&mut self, ctx: &egui::Context) {
        self.show_keyboard = false;
        self.keyboard_text.clear();
        if let Some(id) = self.keyboard_editor.take() {
            ctx.data_mut(|data| data.remove::<egui::text_edit::TextEditState>(id));
        }
    }
    /// Prefill only: the user must still press Connect and request input separately.
    pub fn prefill_invitation(&mut self, link: &str) {
        if link.len() <= 512 && link.parse::<mightymonitor_security::Invitation>().is_ok() {
            self.address = link.to_owned();
        } else {
            self.output.lock().unwrap_or_else(|e|e.into_inner()).error=Some("This connection link is invalid. Copy a fresh private link from the sharing device.".into());
        }
    }
    fn command(&mut self, action: crate::net::ControlAction) {
        let command = crate::net::Command {
            generation: self
                .output
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .control_epoch,
            action,
        };
        if self
            .commands
            .as_ref()
            .is_some_and(|sender| sender.try_send(command).is_err())
        {
            self.disconnect();
        }
    }
    fn connect(&mut self) {
        self.disconnect();
        if let Err(error) =
            mightymonitor_security::Endpoint::parse(&self.address, self.allow_legacy)
        {
            self.output.lock().unwrap_or_else(|e| e.into_inner()).error = Some(error.to_string());
            return;
        }
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        match codec_gst::decoder::Decoder::new() {
            Ok(decoder) => {
                let session = ViewerSession::new(decoder);
                self.output = session.output.clone();
                self.commands = Some(session.commands.clone());
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
        if self.task.as_ref().is_some_and(|task| task.is_finished()) {
            self.task = None;
        }
        if self.last_host_poll.elapsed() >= std::time::Duration::from_millis(500) {
            self.last_host_poll = std::time::Instant::now();
            if let Some(host) = &mut self.host {
                match host.exited() {
                    Ok(true) => {
                        self.host_message = format!(
                            "Sharing stopped. {}",
                            host.diagnostics().unwrap_or_default()
                        );
                        self.host_error = true;
                        self.host = None;
                    }
                    Err(error) => {
                        self.host_message = format!("Unable to check sharing: {error}");
                        self.host_error = true;
                        self.host = None;
                    }
                    Ok(false) => {
                        let diagnostics = host.diagnostics().unwrap_or_default();
                        self.host_message = if diagnostics.contains("mm-host listening on") {
                            format!("Sharing on {}", self.share_address)
                        } else {
                            "Starting sharing…".into()
                        };
                    }
                }
            }
        }
        if !self.themed {
            let mut visuals = egui::Visuals::light();
            visuals.panel_fill = egui::Color32::from_rgb(247, 248, 247);
            visuals.window_fill = egui::Color32::WHITE;
            visuals.override_text_color = Some(egui::Color32::from_rgb(30, 39, 35));
            visuals.selection.bg_fill = egui::Color32::from_rgb(218, 237, 226);
            visuals.selection.stroke.color = egui::Color32::from_rgb(31, 95, 63);
            visuals.hyperlink_color = egui::Color32::from_rgb(38, 115, 77);
            ctx.set_visuals(visuals);
            let mut style = (*ctx.style()).clone();
            style.spacing.item_spacing = egui::vec2(12.0, 12.0);
            style.spacing.button_padding = egui::vec2(16.0, 10.0);
            style.spacing.interact_size.y = 38.0;
            style
                .text_styles
                .insert(egui::TextStyle::Heading, egui::FontId::proportional(26.0));
            style
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
            style
                .text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(16.0));
            ctx.set_style(style);
            self.themed = true;
        }
        if self.fullscreen
            && (ctx.input_mut(|input| crate::input::take_fullscreen_escape(&mut input.events))
                || self.page != Page::View
                || self.texture.is_none()
                || self.task.is_none())
        {
            self.set_fullscreen(ctx, false);
        }
        egui::SidePanel::left("navigation")
            .resizable(false)
            .exact_width(174.0)
            .show_animated(ctx, !self.fullscreen, |ui| {
                ui.add_space(18.0);
                ui.label(egui::RichText::new("MightyMonitor").size(20.0).strong());
                ui.add_space(30.0);
                ui.selectable_value(&mut self.page, Page::View, "View a screen");
                ui.selectable_value(&mut self.page, Page::Share, "Share this screen");
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(12.0);
                    ui.collapsing("About", |ui| {
                        ui.small("Alpha · viewing and control");
                        ui.small("Extended displays are in development.");
                    });
                });
            });
        egui::TopBottomPanel::top("toolbar").show_animated(
            ctx,
            self.host.is_some() || self.texture.is_some() || self.host_error,
            |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if !self.fullscreen && (self.host.is_some() || self.host_error) {
                        ui.label(if self.host.is_some() {
                            "Sharing is on"
                        } else {
                            "Sharing stopped"
                        });
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.page == Page::View
                            && self.texture.is_some()
                            && ui
                                .button(if self.fullscreen {
                                    "Exit full screen"
                                } else {
                                    "Full screen"
                                })
                                .clicked()
                        {
                            self.set_fullscreen(ctx, !self.fullscreen);
                        }
                    });
                });
                ui.add_space(8.0);
            },
        );
        egui::TopBottomPanel::top("connection_controls").show_animated(
            ctx,
            self.page == Page::View && !self.fullscreen,
            |ui| {
                ui.add_space(16.0);
                ui.heading("View a screen");
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [ui.available_width() - 130.0, 40.0],
                        egui::TextEdit::singleline(&mut self.address)
                            .hint_text("Paste a connection link"),
                    );
                    if self.task.is_some() {
                        if ui.button("Disconnect").clicked() {
                            self.disconnect();
                        }
                    } else if ui
                        .add_enabled(
                            !self.address.trim().is_empty(),
                            egui::Button::new("Connect"),
                        )
                        .clicked()
                    {
                        self.address = self.address.trim().to_owned();
                        self.connect();
                    }
                });
                ui.collapsing("Nearby devices", |ui| {
                    let mut scanning=self.discovery.is_some();
                    if ui.checkbox(&mut scanning,"Find nearby sharing devices").changed(){
                        self.discovery=None; self.discovery_error=None;
                        if scanning {match mightymonitor_discovery::Browser::new(){Ok(browser)=>self.discovery=Some(browser),Err(_)=>self.discovery_error=Some("Discovery unavailable. You can still paste a private link.".into())}}
                    }
                    if let Some(error)=&self.discovery_error {ui.label(error);}
                    if let Some(browser)=&self.discovery {
                        let peers=browser.peers();
                        if browser.unavailable(){ui.label("Discovery unavailable on this network. You can still paste a private link.");}
                        else if peers.is_empty(){ui.label("Looking for devices. Enable Show on local network on the sharing device.");}
                        else {
                            egui::ScrollArea::vertical().id_salt("nearby_devices").max_height(120.0).show(ui, |ui| {
                                for peer in peers {ui.horizontal_wrapped(|ui|{ui.label(format!("{} · {}",peer.platform,if peer.extend {"Extra display"}else{"Screen"})); for address in peer.addresses {ui.label(address.to_string());}});}
                            });
                            ui.label("Paste the private link from the sharing device to connect.");
                        }
                    }
                });
                ui.collapsing("Connection settings", |ui| {
                    ui.checkbox(
                        &mut self.allow_legacy,
                        "Connect to an older alpha (unencrypted)",
                    );
                });
                ui.add_space(12.0);
            },
        );
        if self.page != Page::View && self.input_focused {
            self.command(crate::net::ControlAction::Input(
                mightymonitor_core::InputEvent::ReleaseAll,
            ));
            self.input_focused = false;
            self.input_modifiers = egui::Modifiers::NONE;
        }
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
        let control_available = out.control_available;
        let controlling = out.controlling;
        let state = out.state.clone();
        let stats = out.stats.clone();
        let error = out.error.clone();
        drop(out);
        if self.fullscreen && state == State::Closed {
            self.set_fullscreen(ctx, false);
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.page == Page::Share {
                egui::ScrollArea::vertical().id_salt("share_content").auto_shrink([false, false]).show(ui, |ui| {
                ui.add_space(24.0);
                ui.heading("Share this screen");
                ui.add_space(10.0);
                ui.label("Scan with your phone or copy the connection link");
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&self.share_address).size(28.0).strong());
                    let invitation = self.host.as_ref().and_then(|host| host.invitation());
                    if ui
                        .add_enabled(
                            invitation.is_some(),
                            egui::Button::new("Copy connection link"),
                        )
                        .clicked()
                    {
                        if let Some(invitation) = invitation {
                            ctx.copy_text(invitation.to_link());
                        }
                    }
                });
                let link = self
                    .host
                    .as_ref()
                    .and_then(|host| host.invitation())
                    .map(|invite| invite.to_link());
                match link {
                    Some(link) => {
                        if self
                            .share_qr
                            .as_ref()
                            .is_none_or(|(cached, _)| *cached != link)
                        {
                            self.share_qr = qrcode::QrCode::new(link.as_bytes())
                                .ok()
                                .map(|code| (link, code));
                        }
                    }
                    None => self.share_qr = None,
                }
                if let Some((_, code)) = &self.share_qr {
                    ui.add_space(16.0);
                    let modules = code.width();
                    let scale = (240.0 / (modules + 8) as f32).floor().max(2.0);
                    let side = (modules + 8) as f32 * scale;
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 0.0, egui::Color32::WHITE);
                    for y in 0..modules {
                        for x in 0..modules {
                            if code[(x, y)] == qrcode::Color::Dark {
                                let pos = rect.min
                                    + egui::vec2((x + 4) as f32 * scale, (y + 4) as f32 * scale);
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_size(pos, egui::vec2(scale, scale)),
                                    0.0,
                                    egui::Color32::BLACK,
                                );
                            }
                        }
                    }
                }
                ui.add_space(18.0);
                ui.add_enabled_ui(self.host.is_none(), |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.extend, false, "Mirror");
                        let available = cfg!(target_os = "linux") && (std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var("XDG_SESSION_TYPE").is_ok_and(|value| value == "wayland"));
                        ui.add_enabled_ui(available, |ui| {ui.selectable_value(&mut self.extend, true, "Extend");}).response.on_hover_text("Create an extra desktop display. Requires a Wayland desktop with virtual-monitor support.");
                    });
                });
                #[cfg(target_os = "windows")]
                ui.add_enabled_ui(self.host.is_none(), |ui| {
                    ui.horizontal(|ui| {
                        let label = self.screen.as_ref().and_then(|device| self.screens.iter().find(|screen| screen.device() == device)).map(|screen| screen.label()).unwrap_or_else(|| {
                            if self.screen.is_some() { "Screen unavailable".into() } else { "Main screen".into() }
                        });
                        egui::ComboBox::from_id_salt("sharing_screen").selected_text(label).show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.screen, None, "Main screen");
                            for screen in &self.screens {
                                if ui.selectable_value(&mut self.screen, Some(screen.device().to_owned()), screen.label()).clicked() { self.allow_control = false; }
                            }
                        });
                        if ui.button("Refresh screens").clicked() {
                            match mightymonitor_display_windows::displays() {
                                Ok(screens) => { self.screens = screens; self.screen_error = None; }
                                Err(error) => { self.screens.clear(); self.screen_error = Some(error.to_string()); }
                            }
                        }
                    });
                    if let Some(error) = &self.screen_error { ui.label(error); }
                });
                let control_available = self.host.is_none();
                #[cfg(target_os = "windows")]
                let control_available = control_available && self.screen.is_none();
                ui.add_enabled(control_available,
                    egui::Checkbox::new(&mut self.allow_control, "Allow remote control"),
                ).on_disabled_hover_text("Control is currently available for the main screen.");
                ui.add_enabled(self.host.is_none(),egui::Checkbox::new(&mut self.discoverable,"Show on local network")).on_hover_text("Lets nearby devices find this sharing address. Your private link and control permission are not broadcast.");
                if self.host.is_some() {
                    if ui.button("Stop sharing").clicked() {
                        self.host = None;
                        self.share_qr = None;
                        self.host_message = "Sharing stopped".into();
                        self.host_error = false;
                    }
                } else {
                    if ui
                        .add_enabled(
                            true,
                            egui::Button::new(if self.extend { "Share extra display" } else { "Share this screen" })
                                .fill(egui::Color32::from_rgb(218, 237, 226)),
                        )
                        .clicked()
                    {
                        #[cfg(not(target_os = "windows"))]
                        let screen: Option<&str> = None;
                        #[cfg(target_os = "windows")]
                        let screen = self.screen.as_deref();
                        match crate::host::HostProcess::start_with_display(
                            &self.share_address,
                            self.share_fps,
                            self.share_bitrate,
                            self.allow_control,
                            self.extend,
                            self.discoverable,
                            screen,
                        ) {
                            Ok(host) => {
                                self.host = Some(host);
                                self.host_message = "Starting sharing…".into();
                                self.host_error = false;
                            }
                            Err(error) => {
                                self.host_message = error.to_string();
                                self.host_error = true;
                            }
                        }
                    }
                }
                if self.host_error {
                    ui.colored_label(
                        egui::Color32::from_rgb(166, 48, 36),
                        "Sharing could not continue. Open sharing details below.",
                    );
                }
                ui.add_space(20.0);
                ui.collapsing("Settings", |ui| {
                    ui.add_enabled_ui(self.host.is_none(), |ui| {
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt("sharing_network").selected_text("Choose network").show_ui(ui,|ui|{
                                for network in &self.networks {
                                    if ui.selectable_label(false,format!("{} · {}",network.interface,network.address)).clicked(){
                                        let port=self.share_address.parse::<std::net::SocketAddr>().map(|address|address.port()).unwrap_or(5000);
                                        self.share_address=std::net::SocketAddr::new(network.address.into(),port).to_string();
                                    }
                                }
                            });
                            if ui.button("Refresh networks").clicked(){self.networks=mightymonitor_discovery::local_addresses().unwrap_or_default();}
                        });
                        ui.label("Sharing address");
                        ui.text_edit_singleline(&mut self.share_address);
                        ui.add(
                            egui::Slider::new(&mut self.share_fps, 1..=60)
                                .text("Frames per second"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.share_bitrate, 500..=20_000)
                                .text("Video quality (kbit/s)"),
                        );
                    });
                });
                if !self.host_message.is_empty() {
                    ui.collapsing("Sharing details", |ui| {
                        ui.label(&self.host_message);
                    });
                }
                });
            } else {
                if !self.fullscreen || state != State::Connected {ui.horizontal(|ui| {
                    ui.label(match state {
                        State::Connecting => "Connecting…",
                        State::Connected => "Connected",
                        State::Reconnecting => "Reconnecting…",
                        State::Closed => "Not connected",
                    });
                    if state == State::Connected && control_available {
                        let mut enabled = controlling;
                        if ui.checkbox(&mut enabled, "Control").changed() {
                            self.command(crate::net::ControlAction::Control(enabled));
                        }
                    }
                    if controlling && ui.button("Keyboard").clicked(){self.close_keyboard(ctx);self.show_keyboard=true;}
                    if state == State::Connected {
                        ui.collapsing("Details", |ui| {
                            ui.label(format!(
                                "{:.0} fps · {} kbit/s · {} ms",
                                stats.fps, stats.kbps, stats.rtt_ms
                            ));
                        });
                    }
                });}
                if let Some(error) = error {
                    ui.colored_label(egui::Color32::from_rgb(166, 48, 36), error);
                }
                if let Some(texture) = &self.texture {
                    let size = texture.size_vec2();
                    let scale = (ui.available_width() / size.x)
                        .min(ui.available_height() / size.y)
                        .max(0.01);
                    let response = ui.add(
                        egui::Image::new(texture)
                            .fit_to_exact_size(size * scale)
                            .sense(if controlling {
                                egui::Sense::click_and_drag()
                            } else {
                                egui::Sense::hover()
                            }),
                    );
                    if controlling
                        && (response.clicked()
                            || response.drag_started()
                            || (response.hovered() && ctx.input(|i| i.pointer.any_pressed())))
                    {
                        response.request_focus();
                    }
                    let focused = controlling && !self.show_keyboard && response.has_focus() && ctx.input(|i| i.focused);
                    if focused {
                        let (mut events, modifiers) =
                            ctx.input(|i| (i.events.clone(), i.modifiers));
                        if !response.hovered() {
                            events.retain(|event| !matches!(event, egui::Event::MouseWheel { .. }));
                        }
                        let events = crate::input::translate(
                            &events,
                            response.rect,
                            &mut self.input_modifiers,
                            modifiers,
                        );
                        for event in events {
                            self.command(crate::net::ControlAction::Input(event));
                        }
                    } else if self.input_focused {
                        self.command(crate::net::ControlAction::Input(
                            mightymonitor_core::InputEvent::ReleaseAll,
                        ));
                        self.input_modifiers = egui::Modifiers::NONE;
                    }
                    self.input_focused = focused;
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space((ui.available_height() * 0.42).max(20.0));
                        let (response, painter) =
                            ui.allocate_painter(egui::vec2(180.0, 96.0), egui::Sense::hover());
                        let origin = response.rect.min;
                        let ink = egui::Color32::from_rgb(128, 145, 135);
                        for (x, y, w, h) in [(4.0, 4.0, 106.0, 66.0), (86.0, 30.0, 86.0, 54.0)] {
                            let rect = egui::Rect::from_min_size(
                                origin + egui::vec2(x, y),
                                egui::vec2(w, h),
                            );
                            painter.rect_filled(rect, 5.0, egui::Color32::WHITE);
                            painter.rect_stroke(
                                rect,
                                5.0,
                                egui::Stroke::new(2.0_f32, ink),
                                egui::StrokeKind::Inside,
                            );
                            painter.line_segment(
                                [
                                    rect.center_bottom(),
                                    rect.center_bottom() + egui::vec2(0.0, 8.0),
                                ],
                                egui::Stroke::new(2.0_f32, ink),
                            );
                        }
                        painter.circle_filled(
                            origin + egui::vec2(105.0, 63.0),
                            10.0,
                            egui::Color32::from_rgb(38, 115, 77),
                        );
                        ui.add_space(10.0);
                        ui.label("Your other screen will appear here");
                    });
                }
            }
        });
        if self.show_keyboard {
            if state != State::Connected || !controlling || self.page != Page::View {
                self.close_keyboard(ctx);
            } else {
                let mut open = true;
                let mut send = false;
                let mut enter = false;
                let mut cancel = false;

                egui::Window::new("Keyboard")
                    .open(&mut open)
                    .collapsible(false)
                    .resizable(false)
                    .default_width(360.0)
                    .show(ctx, |ui| {
                        let response = ui.add(
                            egui::TextEdit::multiline(&mut self.keyboard_text)
                                .hint_text("Type on the other device")
                                .char_limit(4096)
                                .desired_rows(3),
                        );
                        if self.keyboard_editor.is_none() {
                            response.request_focus();
                        }
                        self.keyboard_editor = Some(response.id);
                        if self.keyboard_text.contains('\0') {
                            ui.colored_label(
                                egui::Color32::from_rgb(166, 48, 36),
                                "Text contains an unsupported character.",
                            );
                        }
                        let valid =
                            !self.keyboard_text.is_empty() && !self.keyboard_text.contains('\0');
                        ui.horizontal(|ui| {
                            send = ui
                                .add_enabled(valid, egui::Button::new("Send text"))
                                .clicked();
                            enter = ui.button("Enter").clicked();
                            cancel = ui.button("Cancel").clicked();
                        });
                    });
                if send {
                    let mut modifiers = egui::Modifiers::NONE;
                    let events = crate::input::translate(
                        &[egui::Event::Paste(std::mem::take(&mut self.keyboard_text))],
                        egui::Rect::NOTHING,
                        &mut modifiers,
                        egui::Modifiers::NONE,
                    );
                    for event in events {
                        self.command(crate::net::ControlAction::Input(event));
                    }
                }
                if enter {
                    for pressed in [true, false] {
                        self.command(crate::net::ControlAction::Input(
                            mightymonitor_core::InputEvent::Key { code: 40, pressed },
                        ));
                    }
                }
                if send || cancel || !open {
                    self.close_keyboard(ctx);
                }
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

/// UDP connect selects a local route without sending packets or discovering peers.
fn local_endpoint() -> String {
    let ip = std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|socket| {
            socket.connect("192.0.2.1:9")?;
            socket.local_addr()
        })
        .map(|a| a.ip())
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
    format!("{ip}:5000")
}
