#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod annotate;
mod app;
mod icons;
mod organize;
mod settings;
mod shell;
mod theme;
mod worker;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let initial_file = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SuniPDF")
            .with_app_id("sunipdf")
            .with_inner_size([1280.0, 780.0])
            .with_min_inner_size([720.0, 480.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "SuniPDF",
        options,
        Box::new(move |cc| Ok(Box::new(app::SuniApp::new(cc, initial_file)))),
    )
}
