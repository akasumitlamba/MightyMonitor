#![forbid(unsafe_code)]
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = mightymonitor_viewer::app::App::new()?;
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
