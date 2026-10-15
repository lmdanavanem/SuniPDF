#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod annot_adv;
mod annotate;
mod app;
mod brand;
mod dialog;
mod docinfo;
mod editor;
mod forms;
mod icons;
mod inline_text;
mod objects;
mod organize;
mod pagesetup;
mod settings;
mod shell;
mod sign;
mod tabs;
mod theme;
mod tools;
mod view;
mod worker;
mod workspace;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    // Chemins absolus dès l'entrée : fichiers récents et titres ne dépendent pas du dossier courant.
    let initial_files: Vec<std::path::PathBuf> = std::env::args_os()
        .skip(1)
        .map(|a| std::path::absolute(&a).unwrap_or_else(|_| a.into()))
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
