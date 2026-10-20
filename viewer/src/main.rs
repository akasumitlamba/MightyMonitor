#![forbid(unsafe_code)]
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod launch_args;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let launch = launch_args::parse(std::env::args_os().skip(1))?;
    if matches!(launch, launch_args::Launch::CheckRuntime) {
        let _decoder = codec_gst::decoder::Decoder::new()?;
        return Ok(());
    }
    let mut app = mightymonitor_viewer::app::App::new()?;
    if let launch_args::Launch::Invitation(link) = launch {
        app.prefill_invitation(&link);
    }
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../assets/mightymonitor.png"))?;
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([760.0, 480.0])
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native("MightyMonitor", options, Box::new(|_| Ok(Box::new(app))))?;
    Ok(())
}
