//! Local revision alignment. The engine worker owns rendering and checked exports.

use crate::{
    app::{EngineState, SuniApp},
    dialog::{self, Spec},
    worker::Request,
};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use sunipdf_core::{
    revision_overlay::{
        stack::{self as core, OverlayStackLayer, OverlayStackOptions, OverlayStackPreview},
        OverlayAlignment, OverlayTransform,
    },
    EngineError,
};
use sunipdf_i18n::{tr, tr_args};
use zeroize::Zeroizing;

pub enum Job {
    Preview {
        options: OverlayStackOptions,
        passwords: Vec<Zeroizing<String>>,
        cancel: Arc<AtomicBool>,
    },
    Export {
        preview: Arc<OverlayStackPreview>,
        path: PathBuf,
        json: bool,
        cancel: Arc<AtomicBool>,
    },
}

impl Job {
    pub(crate) fn touches_source(&self, path: &Path) -> bool {
        let options = match self {
            Self::Preview { options, .. } => options,
            Self::Export { preview, .. } => preview.options(),
        };
        options
            .layers
            .iter()
            .any(|layer| same_path(&layer.path, path))
    }
}

pub enum Outcome {
    Preview(Arc<OverlayStackPreview>),
    Exported(PathBuf),
}

pub fn run_job(engine: &sunipdf_core::Engine, job: Job) -> Result<Outcome, EngineError> {
    match job {
        Job::Preview {
            options,
            passwords,
            cancel,
        } => {
            let passwords: Vec<_> = passwords
                .iter()
                .map(|password| (!password.is_empty()).then_some(password.as_str()))
                .collect();
            core::prepare_overlay_stack(engine, options, &passwords, &cancel)
                .map(|preview| Outcome::Preview(Arc::new(preview)))
        }
        Job::Export {
            preview,
            path,
            json,
            cancel,
        } => {
            let bytes = if json {
                serde_json::to_vec_pretty(&preview.export_json(&cancel)?)
                    .map_err(|error| EngineError::Other(error.to_string()))?
            } else {
                preview.export_png(&cancel)?
            };
            preview.revalidate_sources(&cancel)?;
            if cancel.load(Ordering::Relaxed) {
                return Err(EngineError::Cancelled);
            }
            sunipdf_core::write_new_copy(&path, &bytes)?;
            Ok(Outcome::Exported(path))
        }
    }
}

struct SourceImage {
    texture: egui::TextureHandle,
    size_pt: [f64; 2],
}
struct Images {
    sources: [SourceImage; 2],
    selected: usize,
    combined: egui::TextureHandle,
}

struct LayerState {
    path: Option<PathBuf>,
    page: usize,
    password: Zeroizing<String>,
    two_points: bool,
    anchors: [Vec<[f64; 2]>; 2],
    transform: OverlayTransform,
    color: [u8; 3],
    opacity: f32,
    visible: bool,
}
impl LayerState {
    fn new(index: usize) -> Self {
        let layer = OverlayStackLayer::new(PathBuf::new(), 1, index);
        Self {
            path: None,
            page: 1,
            password: Default::default(),
            two_points: false,
            anchors: Default::default(),
            transform: OverlayTransform::default(),
            color: layer.color,
            opacity: layer.opacity,
            visible: true,
        }
    }
    fn reset_alignment(&mut self) {
        self.anchors = Default::default();
        self.transform = OverlayTransform::default();
    }
    fn options(
        &self,
        index: usize,
        source_preview: bool,
    ) -> Result<OverlayStackLayer, EngineError> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| EngineError::Other("Choose a PDF for every overlay layer".into()))?;
        let path = std::path::absolute(path)?;
        let alignment = if index == 0 {
            OverlayAlignment::default()
        } else if self.two_points {
            if self.anchors.iter().all(|points| points.len() == 2) {
                OverlayAlignment::TwoPoints {
                    base: [self.anchors[0][0], self.anchors[0][1]],
                    revision: [self.anchors[1][0], self.anchors[1][1]],
                }
            } else if source_preview {
                OverlayAlignment::default()
            } else {
                return Err(EngineError::Other(
                    "Choose two corresponding points on the base and selected layer".into(),
                ));
            }
        } else {
            OverlayAlignment::Manual {
                transform: self.transform,
            }
        };
        Ok(OverlayStackLayer {
            path,
            page: self.page,
            alignment,
            color: self.color,
            opacity: self.opacity,
            visible: self.visible,
        })
    }
}

pub struct OverlayState {
    pub open: bool,
    pub busy: bool,
    pub cancel: Arc<AtomicBool>,
    layers: Vec<LayerState>,
    selected: usize,
    long_side: u32,
    generation: u64,
    pending: Option<u64>,
    preview: Option<Arc<OverlayStackPreview>>,
    images: Option<Images>,
    fingerprints: Vec<String>,
    dirty: bool,
    error: Option<String>,
    message: Option<String>,
    last_export: Option<PathBuf>,
}
impl Default for OverlayState {
    fn default() -> Self {
        Self {
            open: false,
            busy: false,
            cancel: Arc::new(AtomicBool::new(false)),
            layers: (0..2).map(LayerState::new).collect(),
            selected: 1,
            long_side: OverlayStackOptions::default().long_side,
            generation: 0,
            pending: None,
            preview: None,
            images: None,
            fingerprints: Vec::new(),
            dirty: true,
            error: None,
            message: None,
            last_export: None,
        }
    }
}
impl OverlayState {
    fn changed(&mut self) {
        self.dirty = true;
        self.error = None;
        self.message = None;
    }
    fn clear_images(&mut self) {
        self.preview = None;
        self.images = None;
        self.fingerprints.clear();
        self.changed();
    }
    fn source_changed(&mut self, index: usize) {
        if index == 0 {
            for layer in &mut self.layers {
                layer.reset_alignment();
            }
        } else if let Some(layer) = self.layers.get_mut(index) {
            layer.reset_alignment();
        }
        self.clear_images();
    }
    fn add_layer(&mut self) {
        if self.busy || self.layers.len() >= core::MAX_LAYERS {
            return;
        }
        self.layers.push(LayerState::new(self.layers.len()));
        self.selected = self.layers.len() - 1;
        self.clear_images();
    }
    fn remove_layer(&mut self, index: usize) {
        if self.busy
            || index == 0
            || self.layers.len() <= core::MIN_LAYERS
            || index >= self.layers.len()
        {
            return;
        }
        self.layers.remove(index);
        if self.selected > index {
            self.selected -= 1;
        }
        self.selected = self.selected.min(self.layers.len() - 1).max(1);
        self.clear_images();
    }
    fn options(&self, source_preview: bool) -> Result<OverlayStackOptions, EngineError> {
        let options = OverlayStackOptions {
            layers: self
                .layers
                .iter()
                .enumerate()
                .map(|(index, layer)| layer.options(index, source_preview))
                .collect::<Result<_, _>>()?,
            long_side: self.long_side,
        };
        options.validate()?;
        Ok(options)
    }
    fn add_anchor(&mut self, source: usize, point: [f64; 2]) {
        if self.busy || source >= 2 || !self.layers[self.selected].two_points {
            return;
        }
        let Some(images) = &self.images else {
            return;
        };
        if images.selected != self.selected {
            return;
        }
        let size = images.sources[source].size_pt;
        if point
            .iter()
            .enumerate()
            .any(|(i, value)| !value.is_finite() || *value < 0.0 || *value > size[i])
        {
            return;
        }
        let points = &mut self.layers[self.selected].anchors[source];
        if points.len() == 2
            || points
                .last()
                .is_some_and(|last| (last[0] - point[0]).hypot(last[1] - point[1]) < 0.001)
        {
            return;
        }
        points.push(point);
        self.changed();
    }
    fn begin(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.pending = Some(self.generation);
        self.busy = true;
        self.cancel = Arc::new(AtomicBool::new(false));
        self.error = None;
        self.message = None;
        self.generation
    }
    fn cancel_job(&mut self) {
        if self.busy {
            self.cancel.store(true, Ordering::Relaxed);
            self.generation = self.generation.wrapping_add(1);
        }
    }
    pub(crate) fn close(&mut self) {
        if self.busy {
            self.cancel_job();
            return;
        }
        self.open = false;
        self.clear_images();
        for layer in &mut self.layers {
            layer.password = Default::default();
            layer.reset_alignment();
        }
    }
    fn export_ready(&self) -> bool {
        self.open && !self.busy && !self.dirty && self.preview.is_some()
    }
    fn upload_sources(
        &self,
        ctx: &egui::Context,
        preview: &OverlayStackPreview,
    ) -> Result<[SourceImage; 2], EngineError> {
        Ok([
            source_image(ctx, preview, 0, self.generation)?,
            source_image(ctx, preview, self.selected, self.generation)?,
        ])
    }

    fn select_layer(&mut self, ctx: &egui::Context, index: usize) {
        if self.busy || index == 0 || index >= self.layers.len() || index == self.selected {
            return;
        }
        self.selected = index;
        if let Some(preview) = &self.preview {
            match source_image(ctx, preview, index, self.generation) {
                Ok(source) => {
                    if let Some(images) = &mut self.images {
                        images.sources[1] = source;
                        images.selected = index;
                    }
                }
                Err(error) => {
                    self.error = Some(error.to_string());
                    self.images = None;
                    self.dirty = true;
                }
            }
        }
    }
}

fn source_image(
    ctx: &egui::Context,
    preview: &OverlayStackPreview,
    index: usize,
    generation: u64,
) -> Result<SourceImage, EngineError> {
    let report = preview.report();
    let metadata = &report["layers"][index]["source"];
    let (Some(width), Some(height)) = (
        metadata["display_width_pt"].as_f64(),
        metadata["display_height_pt"].as_f64(),
    ) else {
        return Err(EngineError::Other("Missing overlay page geometry".into()));
    };
    let raster = preview
        .layer_image(index)
        .ok_or_else(|| EngineError::Other("Missing overlay source image".into()))?;
    Ok(SourceImage {
        texture: upload(ctx, &format!("source-{index}"), generation, raster),
        size_pt: [width, height],
    })
}

fn upload(
    ctx: &egui::Context,
    name: &str,
    generation: u64,
    page: &sunipdf_core::RenderedPage,
) -> egui::TextureHandle {
    ctx.load_texture(
        format!("revision-overlay-{name}-{generation}"),
        egui::ColorImage::from_rgba_unmultiplied(
            [page.width as usize, page.height as usize],
            &page.rgba,
        ),
        egui::TextureOptions::LINEAR,
    )
}
fn same_path(a: &Path, b: &Path) -> bool {
    let resolved = |path: &Path| {
        path.canonicalize()
            .or_else(|_| std::path::absolute(path))
            .unwrap_or_else(|_| path.into())
    };
    let a = resolved(a);
    let b = resolved(b);
    if cfg!(windows) {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    } else {
        a == b
    }
}

impl SuniApp {
    pub(crate) fn open_revision_overlay(&mut self) {
        if self.overlay.busy || self.review.busy || self.ink_blocks_action() {
            return;
        }
        self.review.open = false;
        self.overlay.open = true;
        self.overlay.error = None;
        if let Some(path) = &self.overlay.last_export {
            self.overlay.message = Some(tr_args(
                self.lang(),
                "overlay.exported",
                &[("name", &crate::app::file_name(path))],
            ));
        }
    }
    pub(crate) fn overlay_from_comparison(
        &mut self,
        base: Option<PathBuf>,
        revision: Option<PathBuf>,
        pages: [usize; 2],
    ) {
        self.open_revision_overlay();
        if self.overlay.open && !self.overlay.busy {
            self.overlay.layers = (0..2).map(LayerState::new).collect();
            self.overlay.selected = 1;
            for (index, path) in [base, revision].into_iter().enumerate() {
                self.overlay.layers[index].path = path;
                self.overlay.layers[index].page = pages[index];
            }
            self.overlay.clear_images();
        }
    }
    fn overlay_has_dirty_source(&self) -> bool {
        self.overlay
            .layers
            .iter()
            .filter_map(|layer| layer.path.as_ref())
            .any(|path| {
                (self.dirty
                    && self
                        .document
                        .as_ref()
                        .is_some_and(|doc| same_path(&doc.info.path, path)))
                    || self.tabs.tabs.iter().any(|tab| {
                        (tab.dirty || tab.ui.as_ref().is_some_and(|ui| ui.dirty))
                            && same_path(&tab.path, path)
                    })
            })
    }
    fn start_overlay_preview(&mut self) {
        if self.overlay.busy || !self.overlay.open || !matches!(self.engine, EngineState::Ready) {
            return;
        }
        if self.overlay_has_dirty_source() {
            self.overlay.error = Some(tr(self.lang(), "overlay.stack.dirty_source").into());
            return;
        }
        let options = match self.overlay.options(true) {
            Ok(options) => options,
            Err(error) => {
                self.overlay.error = Some(error.to_string());
                return;
            }
        };
        let generation = self.overlay.begin();
        self.overlay.dirty = true;
        self.worker.send(Request::RevisionOverlay {
            generation,
            job: Job::Preview {
                options,
                passwords: self
                    .overlay
                    .layers
                    .iter()
                    .map(|layer| layer.password.clone())
                    .collect(),
                cancel: self.overlay.cancel.clone(),
            },
        });
    }
    fn export_overlay_to(&mut self, path: PathBuf, json: bool) {
        if !self.overlay.export_ready() {
            return;
        }
        if self.overlay_has_dirty_source() {
            self.overlay.error = Some(tr(self.lang(), "overlay.stack.dirty_source").into());
            return;
        }
        let preview = self.overlay.preview.as_ref().unwrap().clone();
        let generation = self.overlay.begin();
        self.worker.send(Request::RevisionOverlay {
            generation,
            job: Job::Export {
                preview,
                path,
                json,
                cancel: self.overlay.cancel.clone(),
            },
        });
    }
    pub(crate) fn on_revision_overlay(
        &mut self,
        ctx: &egui::Context,
        generation: u64,
        result: Result<Outcome, EngineError>,
    ) {
        if self.overlay.pending != Some(generation) {
            return;
        }
        self.overlay.pending = None;
        self.overlay.busy = false;
        if let Ok(Outcome::Exported(path)) = &result {
            self.overlay.last_export = Some(path.clone());
            self.overlay.message = Some(tr_args(
                self.lang(),
                "overlay.exported",
                &[("name", &crate::app::file_name(path))],
            ));
            return;
        }
        if !self.overlay.open {
            return;
        }
        if self.overlay.generation != generation || self.overlay.cancel.load(Ordering::Relaxed) {
            self.overlay.message = Some(tr(self.lang(), "review.cancelled").into());
            return;
        }
        match result {
            Ok(Outcome::Preview(preview)) => {
                let report = preview.report();
                let fingerprints: Vec<_> = report["layers"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|layer| {
                        layer["source"]["source_sha256"]
                            .as_str()
                            .unwrap_or_default()
                            .to_owned()
                    })
                    .collect();
                if fingerprints.len() != self.overlay.layers.len()
                    || fingerprints.iter().any(String::is_empty)
                {
                    self.overlay.error = Some("Missing overlay source fingerprint".into());
                    return;
                }
                if !self.overlay.fingerprints.is_empty()
                    && self.overlay.fingerprints != fingerprints
                {
                    self.overlay.source_changed(0);
                    self.overlay.message = Some(tr(self.lang(), "overlay.sources_changed").into());
                    return;
                }
                match self.overlay.upload_sources(ctx, &preview) {
                    Ok(sources) => {
                        self.overlay.dirty = self
                            .overlay
                            .options(false)
                            .map_or(true, |options| &options != preview.options());
                        for index in 1..self.overlay.layers.len() {
                            if let Some(transform) = preview.transform(index) {
                                self.overlay.layers[index].transform = transform;
                            }
                        }
                        self.overlay.images = Some(Images {
                            sources,
                            selected: self.overlay.selected,
                            combined: upload(ctx, "combined", generation, preview.image()),
                        });
                        self.overlay.fingerprints = fingerprints;
                        self.overlay.preview = Some(preview);
                    }
                    Err(error) => self.overlay.error = Some(error.to_string()),
                }
            }
            Ok(Outcome::Exported(_)) => unreachable!(),
            Err(error) => {
                self.overlay.error = Some(format!(
                    "{} ({error})",
                    tr(self.lang(), crate::app::error_key(&error))
                ))
            }
        }
    }

    pub(crate) fn revision_overlay_dialog(&mut self, ctx: &egui::Context) {
        if !self.overlay.open {
            return;
        }
        let lang = self.lang();
        let busy = self.overlay.busy;
        let can_preview = !busy
            && matches!(self.engine, EngineState::Ready)
            && self.overlay.layers.iter().all(|layer| layer.path.is_some());
        let can_export = self.overlay.export_ready();
        let compact = ctx.content_rect().width() < 720.0;
        let (
            mut pick,
            mut selected,
            mut remove,
            mut add,
            mut preview,
            mut export,
            mut close,
            mut cancel,
        ) = (None, None, None, false, false, None, false, false);
        let mut clicked_anchor = None;
        let mut changed = false;
        let mut source_changed = None;
        dialog::show(
            ctx,
            Spec::new(
                "revision_overlay",
                tr(lang, "overlay.title"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .width(960.0_f32.min((ctx.content_rect().width() - 32.0).max(300.0))),
            |ui| {
                ui.label(tr(lang, "overlay.saved_files"));
                ui.small(tr(lang, "overlay.stack.limits"));
                ui.add_enabled_ui(!busy, |ui| {
                    let layer_count = self.overlay.layers.len();
                    let selected_layer = self.overlay.selected;
                    for (index, layer) in self.overlay.layers.iter_mut().enumerate() {
                        ui.push_id(("overlay-layer", index), |ui| {
                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    let label = if index == 0 {
                                        tr(lang, "overlay.stack.base").to_owned()
                                    } else {
                                        tr_args(
                                            lang,
                                            "overlay.stack.layer",
                                            &[("number", &(index + 1).to_string())],
                                        )
                                    };
                                    if index == 0 {
                                        ui.strong(label);
                                    } else if ui
                                        .selectable_label(selected_layer == index, label)
                                        .clicked()
                                    {
                                        selected = Some(index);
                                    }
                                    changed |= ui
                                        .checkbox(
                                            &mut layer.visible,
                                            tr(lang, "overlay.stack.visible"),
                                        )
                                        .changed();
                                    if ui.button(tr(lang, "overlay.stack.choose")).clicked() {
                                        pick = Some(index);
                                    }
                                    if index > 0
                                        && ui
                                            .add_enabled(
                                                layer_count > core::MIN_LAYERS,
                                                egui::Button::new(tr(lang, "overlay.stack.remove")),
                                            )
                                            .clicked()
                                    {
                                        remove = Some(index);
                                    }
                                });
                                if let Some(path) = &layer.path {
                                    ui.add(
                                        egui::Label::new(crate::app::file_name(path)).truncate(),
                                    )
                                    .on_hover_text(path.display().to_string());
                                }
                                ui.horizontal_wrapped(|ui| {
                                    let label = ui.label(tr(lang, "overlay.page"));
                                    if ui
                                        .add(egui::DragValue::new(&mut layer.page).range(1..=65535))
                                        .labelled_by(label.id)
                                        .changed()
                                    {
                                        source_changed = Some(index);
                                    }
                                    let label = ui.label(tr(lang, "overlay.stack.color"));
                                    changed |= ui
                                        .color_edit_button_srgb(&mut layer.color)
                                        .labelled_by(label.id)
                                        .changed();
                                    let label = ui.label(tr(lang, "overlay.opacity"));
                                    changed |= ui
                                        .add(
                                            egui::DragValue::new(&mut layer.opacity)
                                                .range(0.0..=1.0)
                                                .speed(0.01)
                                                .max_decimals(2),
                                        )
                                        .labelled_by(label.id)
                                        .changed();
                                });
                                ui.horizontal(|ui| {
                                    let label = ui.label(tr(lang, "overlay.password"));
                                    changed |= ui
                                        .add(
                                            egui::TextEdit::singleline(&mut *layer.password)
                                                .password(true)
                                                .char_limit(256)
                                                .desired_width(120.0),
                                        )
                                        .labelled_by(label.id)
                                        .changed();
                                });
                            });
                        });
                    }
                    add = ui
                        .add_enabled(
                            self.overlay.layers.len() < core::MAX_LAYERS,
                            egui::Button::new(tr(lang, "overlay.stack.add")),
                        )
                        .clicked();
                    ui.separator();
                    ui.strong(tr_args(
                        lang,
                        "overlay.stack.align",
                        &[("number", &(self.overlay.selected + 1).to_string())],
                    ));
                    let layer = &mut self.overlay.layers[self.overlay.selected];
                    ui.horizontal_wrapped(|ui| {
                        changed |= ui
                            .radio_value(&mut layer.two_points, false, tr(lang, "overlay.manual"))
                            .changed();
                        changed |= ui
                            .radio_value(
                                &mut layer.two_points,
                                true,
                                tr(lang, "overlay.two_points"),
                            )
                            .changed();
                        if ui.button(tr(lang, "overlay.reset")).clicked() {
                            layer.reset_alignment();
                            changed = true;
                        }
                    });
                    if layer.two_points {
                        ui.label(tr(lang, "overlay.points_hint"));
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for (index, key) in ["overlay.offset_x", "overlay.offset_y"]
                                .into_iter()
                                .enumerate()
                            {
                                let label = ui.label(tr(lang, key));
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(
                                            &mut layer.transform.translation[index],
                                        )
                                        .range(-1_000_000.0..=1_000_000.0)
                                        .speed(0.25)
                                        .suffix(" pt"),
                                    )
                                    .labelled_by(label.id)
                                    .changed();
                            }
                            let label = ui.label(tr(lang, "overlay.rotation"));
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut layer.transform.rotation_degrees)
                                        .range(-360.0..=360.0)
                                        .speed(0.1)
                                        .suffix("\u{b0}"),
                                )
                                .labelled_by(label.id)
                                .changed();
                            let label = ui.label(tr(lang, "overlay.scale"));
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut layer.transform.scale)
                                        .range(0.01..=100.0)
                                        .speed(0.001)
                                        .max_decimals(6),
                                )
                                .labelled_by(label.id)
                                .changed();
                        });
                        ui.small(tr(lang, "overlay.coordinates"));
                    }
                    ui.horizontal_wrapped(|ui| {
                        let label = ui.label(tr(lang, "overlay.resolution"));
                        egui::ComboBox::from_id_salt("overlay_resolution")
                            .selected_text(format!("{} px", self.overlay.long_side))
                            .show_ui(ui, |ui| {
                                for size in [512, 1024, 1536, 2048] {
                                    changed |= ui
                                        .selectable_value(
                                            &mut self.overlay.long_side,
                                            size,
                                            format!("{size} px"),
                                        )
                                        .changed();
                                }
                            })
                            .response
                            .labelled_by(label.id);
                    });
                });
                if let Some(index) = source_changed {
                    self.overlay.source_changed(index);
                }
                if changed {
                    self.overlay.changed();
                }
                if busy {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(tr(
                            lang,
                            if self.overlay.cancel.load(Ordering::Relaxed) {
                                "ocr.cancelling"
                            } else {
                                "overlay.working"
                            },
                        ));
                    });
                }
                if let Some(images) = &self.overlay.images {
                    ui.separator();
                    ui.columns(2, |columns| {
                        for (index, column) in columns.iter_mut().enumerate() {
                            let source = &images.sources[index];
                            let label = column.strong(tr(
                                lang,
                                if index == 0 {
                                    "review.original"
                                } else {
                                    "overlay.stack.selected"
                                },
                            ));
                            let layer = &self.overlay.layers[self.overlay.selected];
                            let response =
                                image(column, &source.texture, 220.0, !busy && layer.two_points)
                                    .labelled_by(label.id);
                            if !busy && layer.two_points && response.clicked() {
                                if let Some(pos) = response.interact_pointer_pos() {
                                    clicked_anchor = Some((
                                        index,
                                        point_from_image(response.rect, pos, source.size_pt),
                                    ));
                                }
                            }
                            paint_anchors(
                                column.painter(),
                                response.rect,
                                source.size_pt,
                                &layer.anchors[index],
                            );
                            if layer.two_points {
                                column.small(tr_args(
                                    lang,
                                    "overlay.points_selected",
                                    &[("count", &layer.anchors[index].len().to_string())],
                                ));
                            }
                        }
                    });
                    ui.separator();
                    ui.strong(tr(lang, "overlay.result"));
                    if self.overlay.dirty {
                        ui.label(tr(lang, "overlay.refresh_hint"));
                    }
                    image(ui, &images.combined, 320.0, false);
                    ui.small(tr_args(
                        lang,
                        "overlay.canvas",
                        &[
                            ("width", &images.combined.size()[0].to_string()),
                            ("height", &images.combined.size()[1].to_string()),
                        ],
                    ));
                    ui.collapsing(tr(lang, "overlay.fingerprints"), |ui| {
                        for (index, hash) in self.overlay.fingerprints.iter().enumerate() {
                            ui.label(tr_args(
                                lang,
                                "overlay.stack.layer",
                                &[("number", &(index + 1).to_string())],
                            ));
                            ui.monospace(hash);
                        }
                        ui.small(tr(lang, "overlay.fingerprint_hint"));
                    });
                } else {
                    ui.label(tr(lang, "overlay.load_hint"));
                }
                if let Some(error) = &self.overlay.error {
                    ui.colored_label(crate::theme::ERROR, error);
                }
                if let Some(message) = &self.overlay.message {
                    ui.label(message);
                }
            },
            |ui| {
                if busy {
                    cancel = dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                        || ui.input(|input| input.key_pressed(egui::Key::Escape));
                } else {
                    preview = ui
                        .add_enabled(can_preview, egui::Button::new(tr(lang, "overlay.preview")))
                        .clicked();
                    if ui
                        .add_enabled(
                            can_export,
                            egui::Button::new(if compact {
                                "PNG"
                            } else {
                                tr(lang, "overlay.export_png")
                            }),
                        )
                        .clicked()
                    {
                        export = Some(false);
                    }
                    if ui
                        .add_enabled(
                            can_export,
                            egui::Button::new(if compact {
                                "JSON"
                            } else {
                                tr(lang, "overlay.export_json")
                            }),
                        )
                        .clicked()
                    {
                        export = Some(true);
                    }
                    close = dialog::secondary(ui, tr(lang, "dialog.close"))
                        || ui.input(|input| input.key_pressed(egui::Key::Escape));
                }
            },
        );
        if let Some(index) = selected {
            self.overlay.select_layer(ctx, index);
        }
        if let Some(index) = remove {
            self.overlay.remove_layer(index);
        }
        if add {
            self.overlay.add_layer();
        }
        if let Some(index) = pick {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(tr(lang, "dialog.open.filter_pdf"), &["pdf"])
                .pick_file()
            {
                self.overlay.layers[index].path = Some(path);
                self.overlay.layers[index].password = Default::default();
                self.overlay.source_changed(index);
            }
        }
        if let Some((index, point)) = clicked_anchor {
            self.overlay.add_anchor(index, point);
        }
        if preview {
            self.start_overlay_preview();
        }
        if let Some(json) = export {
            let extension = if json { "json" } else { "png" };
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(extension, &[extension])
                .set_file_name(format!("revision-overlay.{extension}"))
                .save_file()
            {
                self.export_overlay_to(path, json);
            }
        }
        if cancel {
            self.overlay.cancel_job();
        }
        if close {
            self.overlay.close();
        }
    }
}

fn image(
    ui: &mut egui::Ui,
    texture: &egui::TextureHandle,
    max_height: f32,
    clickable: bool,
) -> egui::Response {
    let size = texture.size_vec2();
    let ratio = (ui.available_width() / size.x)
        .min(max_height / size.y)
        .min(1.0);
    ui.add(
        egui::Image::new(texture)
            .fit_to_exact_size(size * ratio)
            .sense(if clickable {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            }),
    )
}

fn point_from_image(rect: egui::Rect, pos: egui::Pos2, size: [f64; 2]) -> [f64; 2] {
    [
        f64::from(((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0)) * size[0],
        f64::from(((pos.y - rect.top()) / rect.height()).clamp(0.0, 1.0)) * size[1],
    ]
}

fn paint_anchors(painter: &egui::Painter, rect: egui::Rect, size: [f64; 2], points: &[[f64; 2]]) {
    for (index, point) in points.iter().enumerate() {
        let pos = rect.min
            + egui::vec2(
                (point[0] / size[0]) as f32 * rect.width(),
                (point[1] / size[1]) as f32 * rect.height(),
            );
        painter.circle_filled(pos, 5.0, egui::Color32::from_rgb(220, 120, 0));
        painter.circle_stroke(pos, 5.0, egui::Stroke::new(1.0, egui::Color32::WHITE));
        painter.text(
            pos + egui::vec2(8.0, 6.0),
            egui::Align2::LEFT_TOP,
            (index + 1).to_string(),
            egui::FontId::proportional(14.0),
            egui::Color32::BLACK,
        );
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::app::tests::test_app;
    use egui::{Event, FullOutput, Modifiers, PointerButton, Pos2, Rect};
    use sunipdf_core::Command;
    use sunipdf_i18n::Language;

    struct Controls {
        ctx: egui::Context,
        time: f64,
        size: egui::Vec2,
    }
    impl Controls {
        fn new() -> Self {
            let ctx = egui::Context::default();
            ctx.all_styles_mut(|style| style.animation_time = 0.0);
            Self {
                ctx,
                time: 1.0,
                size: egui::vec2(1500.0, 1500.0),
            }
        }
        fn frame(&mut self, app: &mut SuniApp, events: Vec<Event>) -> FullOutput {
            self.time += 0.025;
            let mut output = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
                    time: Some(self.time),
                    events,
                    ..Default::default()
                },
                |ui| {
                    app.local_review_dialog(ui.ctx());
                    app.revision_overlay_dialog(ui.ctx());
                },
            );
            output.textures_delta.clear();
            output
        }
        fn click_at(&mut self, app: &mut SuniApp, pos: Pos2) {
            self.time += 0.6;
            for pressed in [true, false] {
                self.frame(
                    app,
                    vec![
                        Event::PointerMoved(pos),
                        Event::PointerButton {
                            pos,
                            button: PointerButton::Primary,
                            pressed,
                            modifiers: Modifiers::NONE,
                        },
                    ],
                );
            }
            self.frame(app, vec![]);
        }
        fn click(&mut self, app: &mut SuniApp, key: &str) {
            let text = tr(app.lang(), key).to_owned();
            self.click_text(app, &text);
        }
        fn click_text(&mut self, app: &mut SuniApp, text: &str) {
            fn find(shape: &egui::Shape, text: &str) -> Option<Pos2> {
                match shape {
                    egui::Shape::Text(value) if value.galley.text() == text => {
                        Some(value.pos + value.galley.size() / 2.0)
                    }
                    egui::Shape::Vec(values) => {
                        values.iter().rev().find_map(|value| find(value, text))
                    }
                    _ => None,
                }
            }
            self.frame(app, vec![]);
            let output = self.frame(app, vec![]);
            let pos = output
                .shapes
                .iter()
                .rev()
                .find_map(|shape| find(&shape.shape, text))
                .unwrap_or_else(|| panic!("visible overlay control: {text}"));
            self.click_at(app, pos);
        }
        fn source_point(&mut self, app: &mut SuniApp, index: usize, fraction: [f32; 2]) {
            fn find(shape: &egui::Shape, texture: egui::TextureId) -> Option<Rect> {
                match shape {
                    egui::Shape::Rect(rect)
                        if rect
                            .brush
                            .as_ref()
                            .is_some_and(|brush| brush.fill_texture_id == texture) =>
                    {
                        Some(rect.rect)
                    }
                    egui::Shape::Mesh(mesh) if mesh.texture_id == texture => {
                        let first = mesh.vertices.first()?.pos;
                        let (mut min, mut max) = (first, first);
                        for vertex in &mesh.vertices {
                            min.x = min.x.min(vertex.pos.x);
                            min.y = min.y.min(vertex.pos.y);
                            max.x = max.x.max(vertex.pos.x);
                            max.y = max.y.max(vertex.pos.y);
                        }
                        Some(Rect::from_min_max(min, max))
                    }
                    egui::Shape::Vec(values) => {
                        values.iter().find_map(|value| find(value, texture))
                    }
                    _ => None,
                }
            }
            let texture = app.overlay.images.as_ref().unwrap().sources[index]
                .texture
                .id();
            self.frame(app, vec![]);
            let output = self.frame(app, vec![]);
            let rect = output
                .shapes
                .iter()
                .find_map(|shape| find(&shape.shape, texture))
                .expect("visible source image");
            self.click_at(
                app,
                rect.min + rect.size() * egui::vec2(fraction[0], fraction[1]),
            );
        }
        fn escape(&mut self, app: &mut SuniApp) {
            self.frame(
                app,
                vec![Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
            );
        }
    }

    #[test]
    fn overlay_entry_is_visible_without_document_and_compare_opens_it() {
        use crate::workspace::{Entry, Workspace};
        let entry = &crate::workspace::TABLE[Workspace::RevisionOverlay as usize];
        assert_eq!(entry.entry, Some(Entry::Direct(Command::RevisionOverlay)));
        assert!(!Command::RevisionOverlay.info().needs_document);
        assert_eq!(
            Command::RevisionOverlay.info().menu,
            sunipdf_core::Menu::File
        );
        for lang in Language::ALL {
            let (mut app, requests, _) = test_app();
            app.settings.set_language(lang);
            app.engine = EngineState::Ready;
            app.document = None;
            let mut controls = Controls::new();
            app.execute(&controls.ctx, Command::CompareFiles);
            controls.click(&mut app, "overlay.title");
            assert!(app.overlay.open && !app.review.open && app.dialog_open());
            assert!(app.navigation_blocked());
            controls.click(&mut app, "overlay.preview");
            assert!(requests.try_recv().is_err(), "Preview requires two files");
            controls.click(&mut app, "overlay.two_points");
            assert!(app.overlay.layers[1].two_points);
            controls.click(&mut app, "overlay.manual");
            assert!(!app.overlay.layers[1].two_points);
            app.overlay.layers[1].transform.translation = [12.0, 7.0];
            app.overlay.layers[1].transform.rotation_degrees = 20.0;
            app.overlay.layers[1].transform.scale = 2.0;
            controls.click(&mut app, "overlay.reset");
            assert_eq!(app.overlay.layers[1].transform, OverlayTransform::default());
            app.overlay.layers[0].password = Zeroizing::new("temporary".into());
            controls.escape(&mut app);
            assert!(!app.overlay.open && app.overlay.layers[0].password.is_empty());
        }
    }

    #[test]
    fn layer_buttons_add_select_and_remove_within_the_two_to_four_layer_bounds() {
        for lang in Language::ALL {
            let (mut app, requests, _) = test_app();
            let mut controls = Controls::new();
            app.settings.set_language(lang);
            app.open_revision_overlay();
            controls.click(&mut app, "overlay.stack.add");
            controls.click(&mut app, "overlay.stack.add");
            controls.click(&mut app, "overlay.stack.add");
            assert_eq!(app.overlay.layers.len(), 4);
            let layer_two = tr_args(lang, "overlay.stack.layer", &[("number", "2")]);
            controls.click_text(&mut app, &layer_two);
            assert_eq!(app.overlay.selected, 1);
            controls.click(&mut app, "overlay.stack.remove");
            assert_eq!(app.overlay.layers.len(), 3);
            controls.click(&mut app, "overlay.stack.remove");
            assert_eq!(app.overlay.layers.len(), 2);
            controls.click(&mut app, "overlay.stack.remove");
            assert_eq!(app.overlay.layers.len(), 2);
            assert_eq!(app.overlay.selected, 1);
            assert!(requests.try_recv().is_err());
        }
    }

    #[test]
    fn footer_stays_visible_at_minimum_size_and_quit_releases_the_modal() {
        fn find(shape: &egui::Shape, text: &str) -> Option<Pos2> {
            match shape {
                egui::Shape::Text(value) if value.galley.text() == text => {
                    Some(value.pos + value.galley.size() / 2.0)
                }
                egui::Shape::Vec(values) => values.iter().find_map(|value| find(value, text)),
                _ => None,
            }
        }
        for lang in Language::ALL {
            let (mut app, _, _) = test_app();
            let mut controls = Controls::new();
            controls.size = egui::vec2(720.0, 480.0);
            app.settings.set_language(lang);
            app.open_revision_overlay();
            controls.frame(&mut app, vec![]);
            let output = controls.frame(&mut app, vec![]);
            for key in [
                "overlay.preview",
                "overlay.export_png",
                "overlay.export_json",
                "dialog.close",
            ] {
                let pos = output
                    .shapes
                    .iter()
                    .find_map(|shape| find(&shape.shape, tr(lang, key)))
                    .expect("visible footer label");
                assert!(
                    Rect::from_min_size(Pos2::ZERO, controls.size).contains(pos),
                    "footer inside the window: {key}"
                );
            }
            app.dirty = true;
            app.execute(&controls.ctx, Command::Quit);
            assert!(
                !app.overlay.open,
                "an idle overlay cannot hide the unsaved-document prompt"
            );
            assert!(app.dialog_open() && app.has_unsaved_changes());
            app.overlay.open = true;
            app.overlay.busy = true;
            app.execute(&controls.ctx, Command::Quit);
            assert!(app.overlay.cancel.load(Ordering::Relaxed));
            assert!(
                app.overlay.busy,
                "quit waits for the worker acknowledgement"
            );
        }
    }

    #[test]
    fn four_layer_controls_scroll_and_compact_footer_remains_accessible_in_both_languages() {
        fn position(shape: &egui::Shape, text: &str) -> Option<Pos2> {
            match shape {
                egui::Shape::Text(value) if value.galley.text() == text => {
                    Some(value.pos + value.galley.size() / 2.0)
                }
                egui::Shape::Vec(values) => values.iter().find_map(|shape| position(shape, text)),
                _ => None,
            }
        }
        for lang in Language::ALL {
            let (mut app, requests, _) = test_app();
            let mut controls = Controls::new();
            controls.ctx.enable_accesskit();
            controls.size = egui::vec2(560.0, 520.0);
            app.settings.set_language(lang);
            app.open_revision_overlay();
            app.overlay.add_layer();
            app.overlay.add_layer();
            let targets = [
                tr(lang, "overlay.stack.base").to_owned(),
                tr_args(lang, "overlay.stack.layer", &[("number", "4")]),
                tr(lang, "overlay.stack.add").to_owned(),
            ];
            let mut seen = std::collections::HashSet::new();
            let mut accessible = std::collections::HashSet::new();
            for frame in 0..18 {
                let events = if frame < 3 {
                    Vec::new()
                } else {
                    vec![
                        Event::PointerMoved(egui::pos2(280.0, 250.0)),
                        Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: egui::vec2(0.0, -100.0),
                            phase: egui::TouchPhase::Move,
                            source: egui::MouseWheelSource::Wheel,
                            modifiers: Modifiers::NONE,
                        },
                    ]
                };
                let output = controls.frame(&mut app, events);
                for target in &targets {
                    if output.shapes.iter().any(|shape| {
                        position(&shape.shape, target).is_some_and(|point| {
                            shape.clip_rect.contains(point)
                                && Rect::from_min_size(Pos2::ZERO, controls.size).contains(point)
                        })
                    }) {
                        seen.insert(target.clone());
                    }
                }
                if frame > 1 {
                    for label in [
                        tr(lang, "overlay.preview"),
                        "PNG",
                        "JSON",
                        tr(lang, "dialog.close"),
                    ] {
                        assert!(
                            output
                                .shapes
                                .iter()
                                .any(|shape| position(&shape.shape, label).is_some_and(
                                    |point| shape.clip_rect.contains(point)
                                        && Rect::from_min_size(Pos2::ZERO, controls.size)
                                            .contains(point)
                                )),
                            "footer remains inside narrow window: {label}"
                        );
                    }
                }
                if let Some(tree) = output.platform_output.accesskit_update {
                    for (_, node) in &tree.nodes {
                        if let Some(label) = node
                            .label()
                            .filter(|label| *label != tr(lang, "overlay.stack.color"))
                        {
                            accessible.insert(label.to_owned());
                        }
                        // Color wells expose their swatch value separately from the
                        // accessible name inherited from the visible localized label.
                        if node.role() == egui::accesskit::Role::ColorWell
                            && node
                                .value()
                                .is_some_and(|value| value.starts_with('#') && value.len() == 9)
                            && node.labelled_by().iter().any(|label_id| {
                                tree.nodes.iter().any(|(id, label)| {
                                    id == label_id
                                        && label.value() == Some(tr(lang, "overlay.stack.color"))
                                })
                            })
                            && node.bounds().is_some_and(|bounds| {
                                bounds.x0 >= 0.0
                                    && bounds.y0 >= 0.0
                                    && bounds.x1 <= f64::from(controls.size.x)
                                    && bounds.y1 <= f64::from(controls.size.y)
                            })
                        {
                            accessible.insert(tr(lang, "overlay.stack.color").to_owned());
                        }
                    }
                }
            }
            assert_eq!(
                seen.len(),
                targets.len(),
                "every layer and Add control can be reached by scrolling"
            );
            for key in [
                "overlay.stack.choose",
                "overlay.stack.visible",
                "overlay.stack.color",
                "overlay.stack.remove",
            ] {
                assert!(
                    accessible.contains(tr(lang, key)),
                    "accessible layer control: {key}"
                );
            }
            assert!(requests.try_recv().is_err());
        }
    }

    #[test]
    fn coordinates_use_displayed_top_left_and_invalid_alignment_is_refused() {
        let rect = Rect::from_min_max(egui::pos2(20.0, 30.0), egui::pos2(220.0, 130.0));
        assert_eq!(point_from_image(rect, rect.min, [600.0, 300.0]), [0.0, 0.0]);
        assert_eq!(
            point_from_image(rect, rect.max, [600.0, 300.0]),
            [600.0, 300.0]
        );
        let mut state = OverlayState::default();
        state.layers[0].path = Some(PathBuf::from("base.pdf"));
        state.layers[1].path = Some(PathBuf::from("revision.pdf"));
        state.layers[1].two_points = true;
        assert!(state.options(false).is_err());
        assert!(matches!(
            state.options(true).unwrap().layers[1].alignment,
            OverlayAlignment::Manual { .. }
        ));
        state.layers[1].anchors = [vec![[1.0, 1.0], [1.0, 1.0]], vec![[2.0, 2.0], [3.0, 3.0]]];
        assert!(state.options(false).is_err());
        state.layers[1].two_points = false;
        state.layers[1].transform.scale = f64::NAN;
        assert!(state.options(true).is_err());
    }

    #[test]
    fn changing_one_layer_resets_only_its_alignment_and_base_changes_reset_all() {
        let mut state = OverlayState::default();
        state.add_layer();
        state.add_layer();
        state.add_layer();
        assert_eq!(state.layers.len(), 4);
        for (index, layer) in state.layers.iter_mut().enumerate() {
            layer.path = Some(PathBuf::from(format!("layer-{index}.pdf")));
            layer.anchors = [
                vec![[1.0, 2.0], [30.0, 40.0]],
                vec![[2.0, 3.0], [31.0, 41.0]],
            ];
            layer.transform.translation = [index as f64, 2.0];
        }
        state.source_changed(2);
        assert!(state.layers[2].anchors.iter().all(Vec::is_empty));
        assert_eq!(state.layers[1].anchors[0].len(), 2);
        assert_eq!(state.layers[3].anchors[0].len(), 2);
        assert!(state.dirty);
        state.source_changed(0);
        assert!(state
            .layers
            .iter()
            .all(|layer| layer.anchors.iter().all(Vec::is_empty)
                && layer.transform == OverlayTransform::default()));
        state.remove_layer(0);
        assert_eq!(state.layers.len(), 4);
        state.remove_layer(2);
        assert_eq!(state.layers.len(), 3);
        assert_eq!(state.selected, 2);
        state.remove_layer(1);
        state.remove_layer(1);
        assert_eq!(state.layers.len(), 2);
        state.busy = true;
        state.add_layer();
        assert_eq!(state.layers.len(), 2);
        state.busy = false;
        for layer in &mut state.layers {
            layer.visible = false;
        }
        assert!(
            state.options(true).is_err(),
            "an entirely hidden stack cannot be prepared"
        );
        state.layers[0].visible = true;
        assert_eq!(
            state.options(true).unwrap().layers[0].alignment,
            OverlayAlignment::default()
        );
    }

    #[test]
    fn dirty_hidden_or_inactive_sources_block_preview_without_losing_history() {
        let (mut app, requests, _) = test_app();
        app.open_revision_overlay();
        app.overlay.add_layer();
        app.overlay.add_layer();
        for (index, layer) in app.overlay.layers.iter_mut().enumerate() {
            layer.path = Some(format!("layer-{index}.pdf").into());
        }
        app.overlay.layers[3].path = Some(app.document.as_ref().unwrap().info.path.clone());
        app.overlay.layers[3].visible = false;
        app.dirty = true;
        app.can_undo = true;
        app.start_overlay_preview();
        assert!(app.overlay.error.is_some());
        assert!(requests.try_recv().is_err());
        assert!(app.dirty && app.can_undo);
        app.dirty = false;
        app.overlay.layers[3].path = Some("inactive.pdf".into());
        app.tabs.tabs.push(crate::tabs::Tab {
            path: "inactive.pdf".into(),
            dirty: true,
            recovery_id: 99,
            ui: None,
            doc: None,
        });
        app.start_overlay_preview();
        assert!(
            requests.try_recv().is_err(),
            "a hidden inactive source is still protected"
        );
        let job = Job::Preview {
            options: app.overlay.options(true).unwrap(),
            passwords: (0..4).map(|_| Default::default()).collect(),
            cancel: Arc::new(AtomicBool::new(false)),
        };
        assert!(job.touches_source(Path::new("inactive.pdf")));
        assert!(!job.touches_source(Path::new("unrelated.pdf")));
    }

    #[test]
    fn cancelled_export_keeps_its_committed_receipt_even_after_the_modal_closes() {
        let (mut app, _, _) = test_app();
        app.overlay.open = true;
        let generation = app.overlay.begin();
        app.overlay.cancel_job();
        app.overlay.open = false;
        let output = PathBuf::from("committed-overlay.png");
        app.on_revision_overlay(
            &egui::Context::default(),
            generation,
            Ok(Outcome::Exported(output.clone())),
        );
        assert_eq!(app.overlay.last_export.as_ref(), Some(&output));
        app.open_revision_overlay();
        assert!(app
            .overlay
            .message
            .as_ref()
            .unwrap()
            .contains("committed-overlay.png"));
    }

    fn set_preview_sources(state: &mut OverlayState, preview: &OverlayStackPreview) {
        state.layers = preview
            .options()
            .layers
            .iter()
            .enumerate()
            .map(|(index, source)| {
                let mut layer = LayerState::new(index);
                layer.path = Some(source.path.clone());
                layer.page = source.page;
                layer.color = source.color;
                layer.opacity = source.opacity;
                layer.visible = source.visible;
                layer
            })
            .collect();
        state.selected = 1;
    }

    /// Uses a real preview from the single shared engine integration test.
    pub(crate) fn verify_prepared_controls(preview: &Arc<OverlayStackPreview>) {
        verify_replaced_sources_invalidate_alignment(preview);
        for lang in Language::ALL {
            let (mut app, requests, _) = test_app();
            let mut controls = Controls::new();
            app.settings.set_language(lang);
            app.engine = EngineState::Ready;
            app.execute(&controls.ctx, Command::RevisionOverlay);
            set_preview_sources(&mut app.overlay, preview);
            app.overlay.long_side = preview.options().long_side;
            controls.click(&mut app, "overlay.preview");
            let generation = match requests.try_recv().unwrap() {
                Request::RevisionOverlay {
                    generation,
                    job: Job::Preview { options, .. },
                } => {
                    assert_eq!(&options, preview.options());
                    generation
                }
                _ => panic!("overlay preview request"),
            };
            app.on_revision_overlay(
                &controls.ctx,
                generation + 1,
                Ok(Outcome::Preview(preview.clone())),
            );
            assert!(app.overlay.busy && app.overlay.images.is_none());
            app.on_revision_overlay(
                &controls.ctx,
                generation,
                Ok(Outcome::Preview(preview.clone())),
            );
            assert!(app.overlay.export_ready());
            let texture = app.overlay.images.as_ref().unwrap().combined.id();
            controls.frame(&mut app, vec![]);
            controls.frame(&mut app, vec![]);
            assert_eq!(app.overlay.images.as_ref().unwrap().combined.id(), texture);
            assert!(
                requests.try_recv().is_err(),
                "idle frames do not render or recompose"
            );
            let base_texture = app.overlay.images.as_ref().unwrap().sources[0].texture.id();
            for selected in 2..preview.options().layers.len() {
                app.overlay.select_layer(&controls.ctx, selected);
                let images = app.overlay.images.as_ref().unwrap();
                assert_eq!(images.selected, selected);
                assert_eq!(images.sources[0].texture.id(), base_texture);
                assert_eq!(images.combined.id(), texture);
                assert!(
                    app.overlay.export_ready(),
                    "selection alone does not change the reviewed composite"
                );
            }
            app.overlay.select_layer(&controls.ctx, 1);
            controls.click(&mut app, "overlay.two_points");
            for index in 0..2 {
                controls.source_point(&mut app, index, [0.2, 0.25]);
                controls.source_point(&mut app, index, [0.7, 0.65]);
            }
            assert_eq!(app.overlay.layers[1].anchors[0].len(), 2);
            assert_eq!(app.overlay.layers[1].anchors[1].len(), 2);
            assert!(matches!(
                app.overlay.options(false).unwrap().layers[1].alignment,
                OverlayAlignment::TwoPoints { .. }
            ));
            assert!(!app.overlay.export_ready());
            controls.click(&mut app, "overlay.preview");
            let (generation, cancel) = match requests.try_recv().unwrap() {
                Request::RevisionOverlay {
                    generation,
                    job:
                        Job::Preview {
                            options, cancel, ..
                        },
                } => {
                    assert!(matches!(
                        options.layers[1].alignment,
                        OverlayAlignment::TwoPoints { .. }
                    ));
                    (generation, cancel)
                }
                _ => panic!("aligned preview request"),
            };
            controls.escape(&mut app);
            assert!(cancel.load(Ordering::Relaxed) && app.overlay.busy);
            app.on_revision_overlay(
                &controls.ctx,
                generation,
                Ok(Outcome::Preview(preview.clone())),
            );
            assert!(!app.overlay.busy && app.overlay.dirty);
            assert_eq!(
                app.overlay.images.as_ref().unwrap().combined.id(),
                texture,
                "cancelled results cannot replace textures"
            );
            controls.click(&mut app, "overlay.reset");
            assert!(app.overlay.layers[1].anchors.iter().all(Vec::is_empty));
            controls.click(&mut app, "overlay.manual");
            controls.click(&mut app, "overlay.preview");
            let generation = match requests.try_recv().unwrap() {
                Request::RevisionOverlay { generation, .. } => generation,
                _ => panic!("fresh manual preview"),
            };
            app.on_revision_overlay(
                &controls.ctx,
                generation,
                Ok(Outcome::Preview(preview.clone())),
            );
            assert!(app.overlay.export_ready());
            let last = app.overlay.layers.len() - 1;
            app.overlay.layers[last].visible = false;
            app.overlay.changed();
            app.export_overlay_to("must-not-export-stale.png".into(), false);
            assert!(requests.try_recv().is_err());
            app.overlay.layers[last].visible = true;
            let original_color = app.overlay.layers[last].color;
            app.overlay.layers[last].color = [20, 40, 60];
            app.overlay.changed();
            assert_ne!(
                app.overlay.options(true).unwrap().layers[last].color,
                preview.options().layers[last].color
            );
            app.export_overlay_to("must-not-export-color.png".into(), false);
            assert!(requests.try_recv().is_err());
            app.overlay.layers[last].color = original_color;
            let generation = app.overlay.begin();
            app.on_revision_overlay(
                &controls.ctx,
                generation,
                Ok(Outcome::Preview(preview.clone())),
            );
            assert!(app.overlay.export_ready());
            app.dirty = true;
            let active_path = app.document.as_ref().unwrap().info.path.clone();
            app.document.as_mut().unwrap().info.path = preview.options().layers[last].path.clone();
            app.export_overlay_to("must-not-export-dirty.png".into(), false);
            assert!(requests.try_recv().is_err());
            app.document.as_mut().unwrap().info.path = active_path;
            app.dirty = false;
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("overlay.png");
            app.export_overlay_to(path.clone(), false);
            let generation = match requests.try_recv().unwrap() {
                Request::RevisionOverlay {
                    generation,
                    job:
                        Job::Export {
                            path: sent,
                            json: false,
                            ..
                        },
                } => {
                    assert_eq!(sent, path);
                    generation
                }
                _ => panic!("checked export request"),
            };
            assert!(!app.overlay.export_ready());
            controls.escape(&mut app);
            app.on_revision_overlay(&controls.ctx, generation, Ok(Outcome::Exported(path)));
            assert!(
                app.overlay
                    .message
                    .as_ref()
                    .unwrap()
                    .contains("overlay.png"),
                "an export committed before cancellation is acknowledged"
            );
            controls.click(&mut app, "dialog.close");
            assert!(
                !app.overlay.open && app.overlay.images.is_none() && app.overlay.preview.is_none()
            );
            app.on_revision_overlay(
                &controls.ctx,
                generation,
                Ok(Outcome::Preview(preview.clone())),
            );
            assert!(app.overlay.images.is_none());
        }
    }

    fn verify_replaced_sources_invalidate_alignment(preview: &Arc<OverlayStackPreview>) {
        for changed_source in 0..preview.options().layers.len() {
            for two_points in [false, true] {
                let (mut app, requests, _) = test_app();
                let ctx = egui::Context::default();
                app.overlay.open = true;
                set_preview_sources(&mut app.overlay, preview);
                app.overlay.long_side = preview.options().long_side;
                let generation = app.overlay.begin();
                app.on_revision_overlay(&ctx, generation, Ok(Outcome::Preview(preview.clone())));
                assert!(app.overlay.export_ready());
                app.overlay.layers[1].two_points = two_points;
                app.overlay.layers[1].anchors = [
                    vec![[1.0, 2.0], [20.0, 30.0]],
                    vec![[3.0, 4.0], [40.0, 50.0]],
                ];
                app.overlay.layers[1].transform.translation = [15.0, 25.0];
                app.overlay.fingerprints[changed_source] = "previous-source-bytes".into();
                let generation = app.overlay.begin();
                app.on_revision_overlay(&ctx, generation, Ok(Outcome::Preview(preview.clone())));
                assert!(app.overlay.images.is_none() && app.overlay.preview.is_none());
                assert!(app.overlay.layers[1].anchors.iter().all(Vec::is_empty));
                assert_eq!(app.overlay.layers[1].transform, OverlayTransform::default());
                assert!(app.overlay.dirty && !app.overlay.export_ready());
                app.export_overlay_to(PathBuf::from("must-not-export.png"), false);
                assert!(requests.try_recv().is_err());
                let generation = app.overlay.begin();
                app.on_revision_overlay(&ctx, generation, Ok(Outcome::Preview(preview.clone())));
                assert!(app.overlay.images.is_some());
                assert_eq!(app.overlay.export_ready(), !two_points);
                assert!(app.overlay.layers[1].anchors.iter().all(Vec::is_empty));
            }
        }
    }
}
