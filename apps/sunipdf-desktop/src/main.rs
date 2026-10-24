#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod annot_adv;
mod annotate;
mod annotation_library;
mod annotation_register;
mod annotation_rich;
mod app;
mod appearance_ui;
mod boundary_fill_ui;
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
mod issue_report_ui;
mod library_store;
mod local_review;
#[cfg(target_os = "macos")]
mod mac_menu;
#[cfg(target_os = "macos")]
mod mac_open;
#[cfg(test)]
mod measure_integration_tests;
mod measure_tools;
mod measurement_area_ui;
mod measurement_geometry;
mod measurement_saved;
mod measurement_scales_ui;
mod measurement_shapes_ui;
mod objects;
mod ocr;
mod office;
mod organize;
mod pagesetup;
mod plan_links_ui;
mod plan_set_view;
mod plan_sets;
mod plan_zones_ui;
mod printing;
mod processing;
mod quantity_table;
mod quantity_workbook_ui;
mod reader_layout;
#[cfg(all(test, windows))]
mod reader_ui_evidence;
mod reading;
mod recovery;
mod redact;
mod revision_outlines_ui;
mod revision_overlay;
mod revision_transfer_ui;
mod settings;
mod shell;
mod shortcut_defaults;
mod shortcuts;
mod sign;
mod stamp_presets;
mod tabs;
mod theme;
mod tool_hub;
mod tools;
mod updater;
mod updater_linux;
mod view;
mod visual_search_ui;
#[cfg(any(target_os = "linux", test))]
mod window_chrome;
mod worker;
mod workspace;
mod workspace_table;

fn main() -> eframe::Result<()> {
    initialize_dll_search().map_err(|error| eframe::Error::AppCreation(Box::new(error)))?;
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    // Chemins absolus dès l'entrée : fichiers récents et titres ne dépendent pas du dossier courant.
    let initial_files: Vec<std::path::PathBuf> = std::env::args_os()
        .skip(1)
        .map(|a| std::path::absolute(&a).unwrap_or_else(|_| a.into()))
        .collect();
    let viewport = egui::ViewportBuilder::default()
        .with_title("SuniPDF")
        .with_app_id("sunipdf")
        .with_inner_size([1280.0, 780.0])
        .with_min_inner_size([720.0, 480.0])
        .with_drag_and_drop(true)
        .with_icon(window_icon());
    #[cfg(target_os = "linux")]
    let viewport = viewport.with_decorations(false);
    let options = eframe::NativeOptions {
        renderer: renderer(std::env::var("SUNIPDF_RENDERER").ok().as_deref()),
        viewport,
        ..Default::default()
    };
    log::info!("Rendering backend: {}", options.renderer);
    #[cfg(target_os = "macos")]
    mac_open::install();
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
