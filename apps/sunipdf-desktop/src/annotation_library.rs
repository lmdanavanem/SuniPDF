//! Local annotation templates, explicit library changes and page placement.

use crate::{app::SuniApp, dialog, library_store, settings::Settings};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use sunipdf_core::annotation_library::{
    AnnotationLibrary, LibraryEntry, LibraryPlacement, LibraryProvenance,
};
use sunipdf_core::EngineError;
use sunipdf_i18n::tr;

pub(crate) fn new_entry_id() -> String {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(
        "template-{time:x}-{:x}-{:x}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}

pub(crate) struct Placing {
    entry: LibraryEntry,
    epoch: u64,
    scale: f32,
    hover: Option<(usize, (f32, f32))>,
    bounds: [f32; 4],
}

#[derive(Default)]
struct GroupDraft {
    name: String,
    source: String,
    license: String,
    credit: String,
    members: Vec<(LibraryEntry, f32, f32)>,
}

#[derive(Default)]
pub(crate) struct LibraryState {
    pub(crate) open: bool,
    pub(crate) placing: Option<Placing>,
    data: AnnotationLibrary,
    selected: BTreeSet<String>,
    filter: String,
    loaded: bool,
    load_failed: bool,
    original: Option<Vec<u8>>,
    path: Option<PathBuf>,
    error: Option<String>,
    message: Option<String>,
    deleting: bool,
    group: Option<GroupDraft>,
}

impl LibraryState {
    pub(crate) fn blocks_document(&self) -> bool {
        self.open || self.placing.is_some()
    }

    fn reload(&mut self) -> Result<(), EngineError> {
        let path = self
            .path
            .clone()
            .or_else(|| Settings::path().map(|p| p.with_file_name("annotation-library.json")))
            .ok_or_else(|| {
                EngineError::Other("local annotation library directory is unavailable".into())
            })?;
        let original = library_store::read(&path)?;
        let data = match original.as_deref() {
            Some(bytes) => AnnotationLibrary::from_json(bytes)?,
            None => AnnotationLibrary::default(),
        };
        self.data = data;
        self.original = original;
        self.path = Some(path);
        self.loaded = true;
        self.load_failed = false;
        self.selected.retain(|id| self.data.entry(id).is_some());
        self.error = None;
        Ok(())
    }

    fn ensure_loaded(&mut self) -> Result<(), EngineError> {
        if !self.loaded {
            self.reload()?;
        }
        if self.load_failed {
            return Err(EngineError::Other(
                "reload the annotation library after resolving the read error".into(),
            ));
        }
        Ok(())
    }

    fn commit(&mut self, candidate: AnnotationLibrary) -> Result<(), EngineError> {
        self.ensure_loaded()?;
        let bytes = candidate.to_json()?;
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| EngineError::Other("annotation library path is unavailable".into()))?;
        library_store::commit(path, self.original.as_deref(), &bytes)?;
        self.data = candidate;
        self.original = Some(bytes);
        self.selected.retain(|id| self.data.entry(id).is_some());
        self.error = None;
        Ok(())
    }

    fn copy_data(&self) -> AnnotationLibrary {
        self.data.clone()
    }

    fn add(&mut self, entry: LibraryEntry) -> Result<(), EngineError> {
        self.ensure_loaded()?;
        let mut candidate = self.copy_data();
        candidate.insert(entry)?;
        self.commit(candidate)
    }

    fn import(&mut self, path: &Path) -> Result<(), EngineError> {
        self.ensure_loaded()?;
        let bytes = library_store::read(path)?
            .ok_or_else(|| EngineError::Other("selected library file is missing".into()))?;
        let imported = AnnotationLibrary::from_json(&bytes)?;
        let mut candidate = self.copy_data();
        for entry in imported.entries() {
            candidate.insert(entry.clone())?;
        }
        self.commit(candidate)
    }

    fn export(&mut self, path: &Path, selected: bool) -> Result<(), EngineError> {
        self.ensure_loaded()?;
        let mut exported = AnnotationLibrary::default();
        for entry in self
            .data
            .entries()
            .iter()
            .filter(|entry| !selected || self.selected.contains(&entry.id))
        {
            exported.insert(entry.clone())?;
        }
        sunipdf_core::write_new_copy(path, &exported.to_json()?)
    }

    fn delete_selected(&mut self) -> Result<(), EngineError> {
        self.ensure_loaded()?;
        let mut candidate = self.copy_data();
        for id in &self.selected {
            candidate.remove(id)?;
        }
        self.commit(candidate)
    }

    fn begin_group(&mut self) {
        let members: Vec<_> = self
            .data
            .entries()
            .iter()
            .filter(|entry| self.selected.contains(&entry.id))
            .map(|entry| (entry.clone(), 0.0, 0.0))
            .collect();
        if members.len() < 2 {
            return;
        }
        let combine = |field: fn(&LibraryProvenance) -> &Option<String>| {
            members
                .iter()
                .filter_map(|(entry, _, _)| field(&entry.provenance).clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join("; ")
        };
        self.group = Some(GroupDraft {
            source: combine(|p| &p.source),
            license: combine(|p| &p.license),
            credit: combine(|p| &p.author),
            members,
            ..Default::default()
        });
    }

    fn save_group(&mut self) -> Result<(), EngineError> {
        let group = self
            .group
            .as_ref()
            .ok_or_else(|| EngineError::Other("no annotation group draft".into()))?;
        let mut primitives = Vec::new();
        for (entry, x, y) in &group.members {
            for primitive in &entry.primitives {
                primitives.push(primitive.translated(*x, *y)?);
            }
        }
        let value = |s: &str| (!s.trim().is_empty()).then(|| s.trim().to_owned());
        let entry = LibraryEntry {
            id: new_entry_id(),
            name: group.name.trim().into(),
            provenance: LibraryProvenance {
                author: value(&group.credit),
                source: value(&group.source),
                license: value(&group.license),
            },
            primitives,
        }
        .normalized()?;
        self.add(entry)?;
        self.group = None;
        Ok(())
    }
}

impl SuniApp {
    pub(crate) fn open_annotation_library(&mut self) {
        if self.library.blocks_document()
            || self.dialog_open()
            || self.ink_draft_active()
            || self.annotate.measure.is_active_draft()
            || self.worker.has_pending_mutations()
            || self.navigation_blocked()
        {
            return;
        }
        if let Err(error) = self.library.ensure_loaded() {
            self.library.error = Some(error.to_string());
            self.library.load_failed = true;
        }
        self.library.open = true;
        self.library.message = None;
    }

    pub(crate) fn library_add_entry(&mut self, entry: LibraryEntry) -> Result<(), EngineError> {
        self.library.add(entry)
    }

    pub(crate) fn annotation_library_dialog(&mut self, ctx: &egui::Context) {
        if !self.library.open {
            return;
        }
        let lang = self.lang();
        let can_place = self.document.is_some()
            && matches!(self.engine, crate::app::EngineState::Ready)
            && !self.worker.has_pending_mutations();
        let state = &mut self.library;
        let (
            mut close,
            mut create,
            mut import,
            mut export,
            mut export_selected,
            mut delete,
            mut group,
            mut save_group,
            mut cancel_group,
            mut reload,
            mut place,
        ) = (
            false, false, false, false, false, false, false, false, false, false, None,
        );
        dialog::show(
            ctx,
            dialog::Spec::new(
                "annotation-library",
                tr(lang, "library.title"),
                self.settings.dark_theme,
                self.has_icons,
            )
            .subtitle(tr(lang, "library.description"))
            .width((ctx.content_rect().width() - 50.0).clamp(420.0, 850.0)),
            |ui| {
                ui.label(tr(lang, "library.owned_only"));
                if let Some(error) = &state.error {
                    ui.colored_label(egui::Color32::RED, error);
                }
                if let Some(message) = &state.message {
                    ui.label(message);
                }
                ui.horizontal_wrapped(|ui| {
                    import = ui.button(tr(lang, "library.import")).clicked();
                    export = ui
                        .add_enabled(
                            !state.data.entries().is_empty(),
                            egui::Button::new(tr(lang, "library.export")),
                        )
                        .clicked();
                    export_selected = ui
                        .add_enabled(
                            !state.selected.is_empty(),
                            egui::Button::new(tr(lang, "library.export_selected")),
                        )
                        .clicked();
                    reload = ui.button(tr(lang, "library.reload")).clicked();
                });
                ui.add(
                    egui::TextEdit::singleline(&mut state.filter)
                        .hint_text(tr(lang, "library.filter"))
                        .char_limit(128),
                );
                if state.data.entries().is_empty() {
                    ui.label(tr(lang, "library.empty"));
                }
                let filter = state.filter.to_lowercase();
                for entry in state
                    .data
                    .entries()
                    .iter()
                    .filter(|entry| entry.name.to_lowercase().contains(&filter))
                {
                    ui.push_id(&entry.id, |ui| {
                        ui.group(|ui| {
                            ui.horizontal_wrapped(|ui| {
                                let mut selected = state.selected.contains(&entry.id);
                                if ui.checkbox(&mut selected, &entry.name).changed() {
                                    if selected {
                                        state.selected.insert(entry.id.clone());
                                    } else {
                                        state.selected.remove(&entry.id);
                                    }
                                }
                                ui.label(format!(
                                    "{}: {}",
                                    tr(lang, "library.members"),
                                    entry.primitives.len()
                                ));
                                if ui
                                    .add_enabled(
                                        can_place,
                                        egui::Button::new(tr(lang, "library.place")),
                                    )
                                    .clicked()
                                {
                                    place = Some(entry.clone());
                                }
                            });
                            for (key, value) in [
                                ("library.source", &entry.provenance.source),
                                ("library.license", &entry.provenance.license),
                                ("library.credit", &entry.provenance.author),
                            ] {
                                ui.small(format!(
                                    "{}: {}",
                                    tr(lang, key),
                                    value.as_deref().unwrap_or(tr(lang, "library.unknown"))
                                ));
                            }
                        });
                    });
                }
                ui.horizontal_wrapped(|ui| {
                    group = ui
                        .add_enabled(
                            state.selected.len() >= 2,
                            egui::Button::new(tr(lang, "library.group")),
                        )
                        .clicked();
                    if ui
                        .add_enabled(
                            !state.selected.is_empty(),
                            egui::Button::new(tr(lang, "library.delete")),
                        )
                        .clicked()
                    {
                        state.deleting = true;
                    }
                });
                if state.deleting {
                    ui.label(tr(lang, "library.delete_help"));
                    ui.horizontal(|ui| {
                        delete = ui.button(tr(lang, "library.delete_confirm")).clicked();
                        if ui.button(tr(lang, "library.cancel")).clicked() {
                            state.deleting = false;
                        }
                    });
                }
                if let Some(draft) = &mut state.group {
                    ui.separator();
                    ui.heading(tr(lang, "library.group"));
                    ui.label(tr(lang, "library.group_help"));
                    for (key, value) in [
                        ("library.name", &mut draft.name),
                        ("library.source", &mut draft.source),
                        ("library.license", &mut draft.license),
                        ("library.credit", &mut draft.credit),
                    ] {
                        ui.horizontal(|ui| {
                            ui.label(tr(lang, key));
                            ui.add(egui::TextEdit::singleline(value).char_limit(1024));
                        });
                    }
                    for (entry, x, y) in &mut draft.members {
                        ui.push_id(&entry.id, |ui| {
                            ui.label(&entry.name);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(tr(lang, "library.offset_x"));
                                ui.add(egui::DragValue::new(x).range(0.0..=100000.0));
                                ui.label(tr(lang, "library.offset_y"));
                                ui.add(egui::DragValue::new(y).range(0.0..=100000.0));
                            });
                        });
                    }
                    save_group = ui
                        .add_enabled(
                            !draft.name.trim().is_empty(),
                            egui::Button::new(tr(lang, "library.save")),
                        )
                        .clicked();
                    cancel_group = ui.button(tr(lang, "library.cancel")).clicked();
                }
            },
            |ui| {
                close = ui.button(tr(lang, "library.close")).clicked()
                    || ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                create = ui
                    .add_enabled(can_place, egui::Button::new(tr(lang, "library.create")))
                    .clicked();
            },
        );
        if close {
            self.library.open = false;
            self.library.group = None;
            self.library.deleting = false;
            return;
        }
        let result = if reload {
            self.library.reload()
        } else if delete {
            self.library.delete_selected()
        } else if save_group {
            self.library.save_group()
        } else {
            Ok(())
        };
        if let Err(error) = result {
            self.library.error = Some(error.to_string());
        } else if delete {
            self.library.deleting = false;
            self.library.message = Some(tr(lang, "library.deleted").into());
        } else if save_group {
            self.library.message = Some(tr(lang, "library.saved").into());
        }
        if cancel_group {
            self.library.group = None;
        }
        if group {
            self.library.begin_group();
        }
        if import {
            if let Some(path) = rfd::FileDialog::new()
                .set_title(tr(lang, "library.import"))
                .add_filter("SuniPDF annotation library", &["json"])
                .pick_file()
            {
                match self.library.import(&path) {
                    Ok(()) => self.library.message = Some(tr(lang, "library.imported").into()),
                    Err(error) => self.library.error = Some(error.to_string()),
                }
            }
        }
        if export || export_selected {
            if let Some(path) = rfd::FileDialog::new()
                .set_title(tr(lang, "library.export"))
                .set_file_name("annotations.json")
                .add_filter("SuniPDF annotation library", &["json"])
                .save_file()
            {
                match self.library.export(&path, export_selected) {
                    Ok(()) => self.library.message = Some(tr(lang, "library.exported").into()),
                    Err(error) => self.library.error = Some(error.to_string()),
                }
            }
        }
        if create {
            self.library.open = false;
            self.annotate.adv.selected = None;
            self.open_rich_annotations(ctx);
        }
        if let Some(entry) = place {
            self.start_library_placement(entry);
        }
    }

    fn start_library_placement(&mut self, entry: LibraryEntry) {
        self.library.open = false;
        if !self.ink_can_start() {
            self.library.open = true;
            self.library.error = Some(self.t("library.finish_first").into());
            return;
        }
        let entry = match entry.normalized() {
            Ok(entry) => entry,
            Err(error) => {
                self.library.error = Some(error.to_string());
                self.library.open = true;
                return;
            }
        };
        let bounds = match entry.bounds() {
            Ok(bounds) => bounds,
            Err(error) => {
                self.library.error = Some(error.to_string());
                self.library.open = true;
                return;
            }
        };
        self.cancel_placing();
        self.annotate.tool = crate::annotate::Tool::Select;
        self.annotate.drag = None;
        self.annotate.clear_reading();
        self.edit.selection = None;
        self.annotate.adv.selected = None;
        self.library.placing = Some(Placing {
            entry,
            epoch: self.view.epoch,
            scale: 1.0,
            hover: None,
            bounds,
        });
    }

    pub(crate) fn validate_library_context(&mut self, ctx: &egui::Context) {
        if self
            .library
            .placing
            .as_ref()
            .is_some_and(|p| p.epoch != self.view.epoch || self.document.is_none())
        {
            self.library.placing = None;
            self.status_msg = Some(self.t("library.stale").into());
        }
        if self.library.placing.is_some()
            && !ctx.egui_wants_keyboard_input()
            && !egui::Popup::is_any_open(ctx)
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.library.placing = None;
        }
    }

    pub(crate) fn library_placement_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang();
        let Some(placing) = self.library.placing.as_mut() else {
            return;
        };
        let mut cancel = false;
        ui.horizontal_wrapped(|ui| {
            ui.strong(&placing.entry.name);
            ui.label(tr(lang, "library.place_help"));
            ui.label(tr(lang, "library.scale"));
            ui.add(
                egui::DragValue::new(&mut placing.scale)
                    .range(0.01..=100.0)
                    .speed(0.01),
            );
            cancel = ui.button(tr(lang, "library.cancel")).clicked();
        });
        if cancel {
            self.library.placing = None;
        }
    }

    pub(crate) fn library_hover(&mut self, page: usize, point: Option<(f32, f32)>) {
        if let Some(placing) = &mut self.library.placing {
            if let Some(point) = point {
                placing.hover = Some((page, point));
            } else if placing.hover.is_some_and(|(p, _)| p == page) {
                placing.hover = None;
            }
        }
    }

    pub(crate) fn library_click(&mut self, page: usize, origin: (f32, f32)) {
        let Some(placing) = self.library.placing.take() else {
            return;
        };
        if placing.epoch != self.view.epoch
            || !self
                .document
                .as_ref()
                .is_some_and(|d| page < d.info.page_count())
        {
            return;
        }
        let size = self.document.as_ref().unwrap().info.page_sizes[page];
        let b = placing.bounds;
        if !origin.0.is_finite()
            || !origin.1.is_finite()
            || origin.0 < 0.0
            || origin.1 < 0.0
            || origin.0 + b[2] * placing.scale > size.width_pt
            || origin.1 + b[3] * placing.scale > size.height_pt
        {
            self.status_msg = Some(self.t("library.bounds").into());
            self.library.placing = Some(placing);
            return;
        }
        self.make_current(page);
        if !self.open_library_preview(
            page,
            placing.entry.clone(),
            LibraryPlacement {
                origin,
                scale: placing.scale,
            },
        ) {
            self.library.placing = Some(placing);
        }
    }

    pub(crate) fn paint_library_placement(
        &self,
        painter: &egui::Painter,
        page_rect: egui::Rect,
        page: usize,
        height: f32,
    ) {
        let Some(placing) = &self.library.placing else {
            return;
        };
        let Some((p, (x, y))) = placing.hover else {
            return;
        };
        if p != page {
            return;
        }
        let b = placing.bounds;
        let scale = self.effective_zoom * crate::app::SCREEN_PER_PDF_POINT;
        let position = |x: f32, y: f32| {
            egui::pos2(
                page_rect.left() + x * scale,
                page_rect.top() + (height - y) * scale,
            )
        };
        let rect = egui::Rect::from_two_pos(
            position(x + b[0] * placing.scale, y + b[1] * placing.scale),
            position(x + b[2] * placing.scale, y + b[3] * placing.scale),
        );
        painter.rect_stroke(
            rect,
            0,
            egui::Stroke::new(1.5, egui::Color32::from_rgb(50, 110, 225)),
            egui::StrokeKind::Inside,
        );
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::worker::{EditOp, Request};
    use egui::{Event, FullOutput, Modifiers, Pos2, Rect};
    use sunipdf_core::annotation_library::{LibraryPrimitive, LibraryShape, LibraryStyle};

    pub(crate) fn entry(id: &str, name: &str) -> LibraryEntry {
        LibraryEntry {
            id: id.into(),
            name: name.into(),
            provenance: LibraryProvenance::default(),
            primitives: vec![LibraryPrimitive::Shape {
                shape: LibraryShape::Square,
                rect: [0.0, 0.0, 60.0, 40.0],
                style: LibraryStyle::default(),
            }],
        }
    }

    pub(crate) fn storage(app: &mut SuniApp) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        app.library.path = Some(dir.path().join("annotations.json"));
        app.document.as_mut().unwrap().info.form = sunipdf_core::FormKind::None;
        dir
    }

    pub(crate) struct Controls {
        pub(crate) ctx: egui::Context,
        time: f64,
        canvas: bool,
    }
    impl Controls {
        pub(crate) fn new() -> Self {
            let ctx = egui::Context::default();
            ctx.style_mut(|style| style.animation_time = 0.0);
            Self {
                ctx,
                time: 1.0,
                canvas: false,
            }
        }
        pub(crate) fn frame(&mut self, app: &mut SuniApp, events: Vec<Event>) -> FullOutput {
            self.time += 0.025;
            let mut result = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1600.0, 1100.0))),
                    time: Some(self.time),
                    events,
                    ..Default::default()
                },
                |ui| {
                    if self.canvas {
                        app.handle_input(&self.ctx);
                    }
                    app.toolbar(&self.ctx, ui);
                    app.library_placement_bar(ui);
                    if self.canvas {
                        app.document_view(&self.ctx, ui);
                    }
                    app.rich_annotation_dialog(&self.ctx);
                    app.annotation_library_dialog(&self.ctx);
                },
            );
            result.textures_delta.clear();
            result
        }
        pub(crate) fn click(&mut self, app: &mut SuniApp, key: &str) {
            self.click_text(app, &tr(app.lang(), key).to_owned());
        }
        pub(crate) fn click_text(&mut self, app: &mut SuniApp, label: &str) {
            fn find(shape: &egui::Shape, label: &str) -> Option<Pos2> {
                match shape {
                    egui::Shape::Text(text) if text.galley.text() == label => {
                        Some(text.pos + text.galley.size() / 2.0)
                    }
                    egui::Shape::Vec(shapes) => shapes.iter().rev().find_map(|s| find(s, label)),
                    _ => None,
                }
            }
            self.frame(app, vec![]);
            let output = self.frame(app, vec![]);
            let pos = output
                .shapes
                .iter()
                .rev()
                .find_map(|s| find(&s.shape, label))
                .unwrap_or_else(|| panic!("visible library control: {label}"));
            self.time += 0.6;
            for pressed in [true, false] {
                self.frame(
                    app,
                    vec![
                        Event::PointerMoved(pos),
                        Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Modifiers::NONE,
                        },
                    ],
                );
            }
            self.frame(app, vec![]);
        }
    }

    #[test]
    fn library_import_export_reload_and_failed_writes_preserve_active_records() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.json");
        let mut state = LibraryState {
            path: Some(path.clone()),
            ..Default::default()
        };
        state.add(entry("one", "First")).unwrap();
        let original = std::fs::read(&path).unwrap();
        let bad = dir.path().join("bad.json");
        std::fs::write(&bad, b"{\"version\":999,\"scripts\":[]}").unwrap();
        assert!(state.import(&bad).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(state.data.entries().len(), 1);
        let mut source = AnnotationLibrary::default();
        source.insert(entry("two", "Second")).unwrap();
        let imported = dir.path().join("import.json");
        std::fs::write(&imported, source.to_json().unwrap()).unwrap();
        state.import(&imported).unwrap();
        assert_eq!(state.data.entries().len(), 2);
        assert!(state.import(&imported).is_err());
        assert_eq!(state.data.entries().len(), 2);
        state.selected.insert("two".into());
        let export = dir.path().join("export.json");
        state.export(&export, true).unwrap();
        let exported = AnnotationLibrary::from_json(&std::fs::read(&export).unwrap()).unwrap();
        assert_eq!(exported.entries(), [entry("two", "Second")]);
        assert!(
            state.export(&export, false).is_err(),
            "exports never overwrite an existing destination"
        );
        let active = state.data.clone();
        std::fs::write(&path, b"malformed external edit").unwrap();
        assert!(state.reload().is_err());
        assert_eq!(state.data, active);
        assert!(state.add(entry("three", "Third")).is_err());
        assert_eq!(state.data, active);
        assert_eq!(std::fs::read(&path).unwrap(), b"malformed external edit");
    }

    #[test]
    fn visible_library_controls_group_cancel_delete_and_reload_without_editing_pdf() {
        for language in ["en-US", "fr-FR"] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            let _dir = storage(&mut app);
            app.settings.language = language.into();
            app.library_add_entry(entry("one", "First")).unwrap();
            app.library_add_entry(entry("two", "Second")).unwrap();
            let mut controls = Controls::new();
            controls.click(&mut app, "library.short");
            assert!(app.library.open);
            assert!(app.navigation_blocked());
            controls.click_text(&mut app, "First");
            controls.click_text(&mut app, "Second");
            controls.click(&mut app, "library.group");
            let group = app.library.group.as_mut().unwrap();
            group.name = "Grouped symbol".into();
            group.members[1].1 = 90.0;
            controls.click(&mut app, "library.save");
            assert!(app.library.group.is_none());
            let combined = app.library.data.entries().last().unwrap();
            assert_eq!(combined.primitives.len(), 2);
            assert_eq!(combined.bounds().unwrap(), [0.0, 0.0, 150.0, 40.0]);
            controls.click(&mut app, "library.delete");
            controls.click(&mut app, "library.cancel");
            assert_eq!(app.library.data.entries().len(), 3);
            controls.click(&mut app, "library.delete");
            controls.click(&mut app, "library.delete_confirm");
            assert_eq!(app.library.data.entries().len(), 1);
            controls.click(&mut app, "library.reload");
            assert_eq!(app.library.data.entries().len(), 1);
            controls.click(&mut app, "library.close");
            assert!(!app.library.open);
            assert!(!app.navigation_blocked());
            assert!(requests.try_recv().is_err());
        }
    }

    #[test]
    fn library_placement_requires_preview_cancel_discards_and_stale_epochs_refuse() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let _dir = storage(&mut app);
        app.library_add_entry(entry("one", "First")).unwrap();
        let mut controls = Controls::new();
        controls.click(&mut app, "library.short");
        controls.click(&mut app, "library.place");
        assert!(!app.library.open && app.library.placing.is_some());
        assert!(app.navigation_blocked());
        assert!(requests.try_recv().is_err());
        app.library_click(0, (100000.0, 0.0));
        assert!(app.library.placing.is_some());
        assert!(requests.try_recv().is_err());
        app.library_click(0, (40.0, 60.0));
        assert!(app.library.placing.is_none());
        assert!(app.annotate.adv.rich.is_some());
        let (epoch, generation) = match requests.recv().unwrap() {
            Request::PreviewRichAnnotation {
                page: 0,
                annotation:
                    sunipdf_core::annot_rich::RichAnnotation::LibraryPlacement { entry, placement },
                epoch,
                generation,
                ..
            } => {
                assert_eq!(entry.name, "First");
                assert_eq!(placement.origin, (40.0, 60.0));
                (epoch, generation)
            }
            _ => panic!("library placement preview"),
        };
        controls.click(&mut app, "bar.apply");
        assert!(
            requests.try_recv().is_err(),
            "Apply stays disabled while rendering"
        );
        controls.click(&mut app, "dialog.password.cancel");
        assert!(app.annotate.adv.rich.is_none());
        assert!(
            matches!(requests.recv().unwrap(),Request::DiscardRichAnnotationPreview{epoch:e,generation:g} if e==epoch && g==generation)
        );
        let value = entry("one", "First");
        app.start_library_placement(value.clone());
        app.library.placing.as_mut().unwrap().epoch = app.view.epoch.wrapping_sub(1);
        app.validate_library_context(&controls.ctx);
        assert!(app.library.placing.is_none());
        app.start_library_placement(value);
        controls.click(&mut app, "library.cancel");
        assert!(app.library.placing.is_none());
        assert!(requests.try_recv().is_err());
        assert!(!app.dirty);
        assert_eq!(app.library.data.entries().len(), 1);
    }

    #[test]
    fn canvas_click_places_a_library_template_and_escape_cancels_without_mutation() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let _directory = storage(&mut app);
        app.zoom = crate::app::Zoom::Fixed(0.8);
        app.view.mode = crate::view::ViewMode::Single;
        app.library_add_entry(entry("one", "First")).unwrap();
        let mut controls = Controls::new();
        controls.canvas = true;
        controls.click(&mut app, "library.short");
        controls.click(&mut app, "library.place");
        for request in requests.try_iter() {
            assert!(!matches!(
                request,
                Request::Edit(_) | Request::Save { .. } | Request::PreviewRichAnnotation { .. }
            ));
        }
        controls.frame(&mut app, vec![]);
        let (page, rect, height) = app.edit.page_view.unwrap();
        let point = (40.0, 650.0);
        let k = app.effective_zoom * crate::app::SCREEN_PER_PDF_POINT;
        let pos = rect.min + egui::vec2(point.0 * k, (height - point.1) * k);
        assert!(rect.contains(pos));
        controls.time += 0.6;
        for pressed in [true, false] {
            controls.frame(
                &mut app,
                vec![
                    Event::PointerMoved(pos),
                    Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    },
                ],
            );
        }
        let mut previews = 0;
        for request in requests.try_iter() {
            match request {
                Request::PreviewRichAnnotation {
                    page: p,
                    annotation:
                        sunipdf_core::annot_rich::RichAnnotation::LibraryPlacement { placement, .. },
                    ..
                } => {
                    assert_eq!(p, page);
                    assert!((placement.origin.0 - point.0).abs() < 0.01);
                    assert!((placement.origin.1 - point.1).abs() < 0.01);
                    previews += 1;
                }
                Request::Edit(_) | Request::Save { .. } => {
                    panic!("mouse placement must only preview")
                }
                _ => {}
            }
        }
        assert_eq!(previews, 1);
        controls.click(&mut app, "dialog.password.cancel");
        assert!(app.annotate.adv.rich.is_none());
        app.start_library_placement(entry("one", "First"));
        controls.frame(
            &mut app,
            vec![Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        assert!(app.library.placing.is_none());
        for request in requests.try_iter() {
            assert!(!matches!(request, Request::Edit(_) | Request::Save { .. }));
        }
        assert!(!app.dirty);
    }

    #[test]
    fn library_apply_uses_the_prepared_generation_and_never_a_direct_insert() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let _dir = storage(&mut app);
        let mut controls = Controls::new();
        assert!(app.open_library_preview(
            0,
            entry("one", "First"),
            LibraryPlacement {
                origin: (30.0, 40.0),
                scale: 1.0
            }
        ));
        let (epoch, generation) = match requests.recv().unwrap() {
            Request::PreviewRichAnnotation {
                epoch, generation, ..
            } => (epoch, generation),
            _ => panic!("preview"),
        };
        let preview = || sunipdf_core::annot_rich::RichAnnotationPreview {
            image: sunipdf_core::RenderedPage {
                page_index: 0,
                width: 4,
                height: 2,
                rgba: vec![255; 32],
            },
            scale: 1.0,
            region: sunipdf_core::PixelRect::new(0, 0, 4, 2),
            index: 0,
            prepared: None,
        };
        app.on_rich_preview(
            &controls.ctx,
            epoch,
            generation.wrapping_sub(1),
            Ok(preview()),
        );
        assert!(!app.annotate.adv.rich.as_ref().unwrap().prepared);
        app.on_rich_preview(&controls.ctx, epoch, generation, Ok(preview()));
        controls.click(&mut app, "bar.apply");
        assert!(
            matches!(requests.recv().unwrap(),Request::Edit(EditOp::CommitRichAnnotationPreview{epoch:e,generation:g}) if e==epoch && g==generation)
        );
        assert!(matches!(requests.recv().unwrap(), Request::FinishMutation));
        assert!(requests.try_recv().is_err());
        assert!(app.annotate.adv.rich.as_ref().unwrap().applying);
    }
}
