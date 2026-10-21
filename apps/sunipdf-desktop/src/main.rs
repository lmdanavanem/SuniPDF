#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod annot_adv;
mod annotate;
mod annotation_register;
mod annotation_rich;
mod app;
mod brand;
mod certificate;
mod dialog;
mod docinfo;
mod editor;
mod features;
mod form_builder;
mod forms;
mod icons;
mod inline_text;
mod local_review;
mod objects;
mod ocr;
mod office;
mod organize;
mod pagesetup;
mod printing;
mod processing;
mod reading;
mod recovery;
mod redact;
mod settings;
mod shell;
mod shortcut_defaults;
mod shortcuts;
mod sign;
mod stamp_presets;
mod tabs;
mod theme;
mod tools;
mod updater;
mod view;
mod worker;
mod workspace;

fn main() -> eframe::Result<()> {
    initialize_dll_search().map_err(|error| eframe::Error::AppCreation(Box::new(error)))?;
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    // Chemins absolus dès l'entrée : fichiers récents et titres ne dépendent pas du dossier courant.
    let initial_files: Vec<std::path::PathBuf> = std::env::args_os()
        .skip(1)
        .map(|a| std::path::absolute(&a).unwrap_or_else(|_| a.into()))
        .collect();
    let options = eframe::NativeOptions {
        renderer: renderer(std::env::var("SUNIPDF_RENDERER").ok().as_deref()),
        viewport: egui::ViewportBuilder::default()
            .with_title("SuniPDF")
            .with_app_id("sunipdf")
            .with_inner_size([1280.0, 780.0])
            .with_min_inner_size([720.0, 480.0])
            .with_drag_and_drop(true)
            .with_icon(window_icon()),
        ..Default::default()
    };
    log::info!("Rendering backend: {}", options.renderer);
    eframe::run_native(
        "SuniPDF",
        options,
        Box::new(move |cc| Ok(Box::new(app::SuniApp::new(cc, initial_files)))),
    )
}

fn initialize_dll_search() -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::LibraryLoader::{
            SetDefaultDllDirectories, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
        };
        // Remove the working directory and PATH before any runtime library is loaded.
        if unsafe { SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_DEFAULT_DIRS) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

fn renderer(requested: Option<&str>) -> eframe::Renderer {
    if requested == Some("wgpu") {
        eframe::Renderer::Wgpu
    } else {
        eframe::Renderer::Glow
    }
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

#[cfg(test)]
mod tests {
    #[test]
    fn renderer_keeps_opengl_default_and_accepts_explicit_wgpu() {
        assert_eq!(super::renderer(None), eframe::Renderer::Glow);
        assert_eq!(super::renderer(Some("glow")), eframe::Renderer::Glow);
        assert_eq!(super::renderer(Some("wgpu")), eframe::Renderer::Wgpu);
        assert_eq!(super::renderer(Some("invalid")), eframe::Renderer::Glow);
    }
}
