#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod annotate;
mod app;
mod docinfo;
mod editor;
mod icons;
mod organize;
mod settings;
mod shell;
mod tabs;
mod theme;
mod tools;
mod worker;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let initial_files: Vec<std::path::PathBuf> = std::env::args_os()
        .skip(1)
        .map(std::path::PathBuf::from)
        .collect();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SuniPDF")
            .with_app_id("sunipdf")
            .with_inner_size([1280.0, 780.0])
            .with_min_inner_size([720.0, 480.0])
            .with_drag_and_drop(true)
            .with_icon(window_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "SuniPDF",
        options,
        Box::new(move |cc| Ok(Box::new(app::SuniApp::new(cc, initial_files)))),
    )
}

/// Icône de fenêtre (PNG 64 px embarqué) ; icône par défaut si le décodage échoue.
fn window_icon() -> egui::IconData {
    let png = include_bytes!("../../../assets/icon/sunipdf-64.png");
    match image::load_from_memory(png) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            egui::IconData {
                width: rgba.width(),
                height: rgba.height(),
                rgba: rgba.into_raw(),
            }
        }
        Err(_) => egui::IconData::default(),
    }
}
