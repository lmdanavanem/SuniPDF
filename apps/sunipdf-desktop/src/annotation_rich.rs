//! Guarded drafts for native-font FreeText and additional standard annotations.

use crate::app::SuniApp;
use crate::editor::{rgb_tuple, TextFont};
use crate::worker::{EditOp, Request};
use std::io::Read;
use std::path::PathBuf;
use sunipdf_core::annot_rich::{
    RichAnnotation, RichAnnotationPreview, RichFreeTextStyle, StampKind, VertexAnnotKind,
};
use sunipdf_core::annotation_library::{LibraryEntry, LibraryPlacement, LibraryProvenance};
use sunipdf_core::measurement::{Measurement, MeasurementKind, MeasurementRecord, MeasurementUnit};
use sunipdf_core::{AnnotStyle, EngineError, FontFamily, PointRect, TextStyle};
use sunipdf_i18n::tr;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RichMode {
    FreeText,
    Polygon,
    Polyline,
    Cloud,
    Stamp,
    ImageStamp,
    Measurement,
    Attachment,
    EraseInk,
    Library,
}

impl RichMode {
    fn key(self) -> &'static str {
        match self {
            Self::FreeText => "rich.free_text",
            Self::Polygon => "rich.polygon",
            Self::Polyline => "rich.polyline",
            Self::Cloud => "rich.cloud",
            Self::Stamp => "rich.stamp",
            Self::ImageStamp => "rich.image_stamp",
            Self::Measurement => "measure.title",
            Self::Attachment => "rich.attachment",
            Self::EraseInk => "rich.eraser",
            Self::Library => "library.preview_title",
        }
    }
}

pub struct RichDraft {
    pub page: usize,
    pub epoch: u64,
    pub generation: u64,
    pub mode: RichMode,
    pub rect: PointRect,
    pub text: String,
    pub points: String,
    pub font: TextFont,
    pub bold: bool,
    pub italic: bool,
    pub size: f32,
    pub rgb: [u8; 3],
    pub fill: bool,
    pub fill_rgb: [u8; 3],
    pub width: f32,
    pub opacity: f32,
    pub radius: f32,
    pub stamp: StampKind,
    pub image_path: Option<PathBuf>,
    pub measure_kind: MeasurementKind,
    pub measure_unit: MeasurementUnit,
    pub calibration: String,
    pub known_distance: f64,
    pub precision: usize,
    pub filename: String,
    pub bytes: Vec<u8>,
    pub target: Option<usize>,
    pub replace: bool,
    pub font_chosen: bool,
    pub preview: Option<egui::TextureHandle>,
    pub prepared: bool,
    pub preview_index: usize,
    pub busy: bool,
    pub applying: bool,
    pub error: Option<String>,
    interactive_measurement: bool,
    measurement_edit: Option<MeasurementRecord>,
    measurement_geometry_edit: bool,
    author: Option<String>,
    library_entry: Option<LibraryEntry>,
    library_placement: LibraryPlacement,
    library_name: String,
    library_source: String,
    library_license: String,
    library_credit: String,
}

fn points(text: &str, limit: usize) -> Option<Vec<(f32, f32)>> {
    let mut result = Vec::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        if result.len() >= limit {
            return None;
        }
        let mut values = line
            .split([',', ';', ' ', '\t'])
            .filter(|value| !value.is_empty());
        let (x, y) = (
            values.next()?.parse::<f32>().ok()?,
            values.next()?.parse::<f32>().ok()?,
        );
        if values.next().is_some() || !x.is_finite() || !y.is_finite() {
            return None;
        }
        result.push((x, y));
    }
    (!result.is_empty()).then_some(result)
}

impl RichDraft {
    fn library_save_supported(&self) -> bool {
        self.target.is_none()
            && self.measurement_edit.is_none()
            && matches!(
                self.mode,
                RichMode::FreeText
                    | RichMode::Polygon
                    | RichMode::Polyline
                    | RichMode::Cloud
                    | RichMode::Stamp
                    | RichMode::ImageStamp
            )
            && (self.mode != RichMode::FreeText || matches!(self.font, TextFont::Standard(_)))
    }
    fn valid_inputs(&self) -> bool {
        match self.mode {
            RichMode::FreeText => {
                !self.text.trim().is_empty() && (!self.replace || self.font_chosen)
            }
            RichMode::Polygon | RichMode::Polyline | RichMode::Cloud => {
                points(&self.points, 512).is_some()
            }
            RichMode::EraseInk => self.target.is_some() && points(&self.points, 256).is_some(),
            RichMode::Attachment => !self.filename.is_empty(),
            RichMode::Stamp => true,
            RichMode::ImageStamp => self.image_path.is_some(),
            RichMode::Measurement => self.measurement().is_some(),
            RichMode::Library => {
                self.library_entry.is_some()
                    && self.library_placement.scale.is_finite()
                    && self.library_placement.scale > 0.0
            }
        }
    }
    fn operation(&self) -> Option<RichAnnotation> {
        let style = AnnotStyle {
            rgb: rgb_tuple(self.rgb),
            fill_rgb: self.fill.then_some(rgb_tuple(self.fill_rgb)),
            width_pt: self.width,
            opacity: self.opacity / 100.0,
            dash_pattern: Vec::new(),
        };
        Some(match self.mode {
            RichMode::FreeText => {
                if self.text.trim().is_empty() || (self.replace && !self.font_chosen) {
                    return None;
                }
                let (family, font_file, font_face_index) = match &self.font {
                    TextFont::Standard(family) => (*family, None, 0),
                    TextFont::System(font) => (
                        FontFamily::Helvetica,
                        Some(font.path.clone()),
                        font.face_index,
                    ),
                };
                RichAnnotation::FreeText {
                    index: if self.replace { self.target } else { None },
                    rect: self.rect,
                    text: self.text.clone(),
                    style: RichFreeTextStyle {
                        text: TextStyle {
                            family,
                            font_file,
                            font_face_index,
                            bold: self.bold,
                            italic: self.italic,
                            size_pt: self.size,
                            rgb: rgb_tuple(self.rgb),
                            opacity: self.opacity / 100.0,
                        },
                        fill_rgb: style.fill_rgb,
                        border_pt: self.width,
                    },
                }
            }
            RichMode::Polygon | RichMode::Polyline | RichMode::Cloud => RichAnnotation::Vertices {
                kind: match self.mode {
                    RichMode::Polygon => VertexAnnotKind::Polygon,
                    RichMode::Polyline => VertexAnnotKind::Polyline,
                    _ => VertexAnnotKind::Cloud,
                },
                points: points(&self.points, 512)?,
                style: if self.mode == RichMode::Polyline {
                    AnnotStyle {
                        fill_rgb: None,
                        ..style
                    }
                } else {
                    style
                },
            },
            RichMode::Stamp => RichAnnotation::Stamp {
                rect: self.rect,
                kind: self.stamp,
            },
            RichMode::ImageStamp => RichAnnotation::ImageStamp {
                rect: self.rect,
                image: self.image_path.clone()?,
                opacity: self.opacity / 100.0,
            },
            RichMode::Library => RichAnnotation::LibraryPlacement {
                entry: self.library_entry.clone()?,
                placement: self.library_placement,
            },
            RichMode::Measurement => {
                if self.measurement_edit.is_some() {
                    return None;
                }
                RichAnnotation::Measurement(self.measurement()?)
            }
            RichMode::Attachment => {
                if self.filename.is_empty() {
                    return None;
                }
                RichAnnotation::Attachment {
                    rect: self.rect,
                    filename: self.filename.clone(),
                    bytes: self.bytes.clone(),
                }
            }
            RichMode::EraseInk => RichAnnotation::EraseInk {
                index: self.target?,
                path: points(&self.points, 256)?,
                radius: self.radius,
            },
        })
    }

    fn clear_preview(&mut self) {
        self.preview = None;
        self.prepared = false;
        self.error = None;
    }

    fn measurement(&self) -> Option<Measurement> {
        let calibration: [(f32, f32); 2] = points(&self.calibration, 2)?.try_into().ok()?;
        let measurement = Measurement {
            kind: self.measure_kind,
            points: points(&self.points, 128)?,
            calibration,
            known_distance: self.known_distance,
            unit: self.measure_unit,
            precision: self.precision,
        };
        measurement.value().ok()?;
        if self.measurement_edit.as_ref().is_some_and(|record| {
            measurement.kind != record.measurement.kind
                || (!self.measurement_geometry_edit
                    && measurement.points != record.measurement.points)
                || measurement.points.len() != record.measurement.points.len()
                || measurement.calibration != record.measurement.calibration
                || (self.measurement_geometry_edit
                    && (measurement.known_distance != record.measurement.known_distance
                        || measurement.unit != record.measurement.unit
                        || measurement.precision != record.measurement.precision))
        }) {
            return None;
        }
        Some(measurement)
    }

    fn load_measurement(&mut self, measurement: &Measurement) {
        self.mode = RichMode::Measurement;
        self.interactive_measurement = true;
        self.measure_kind = measurement.kind;
        self.measure_unit = measurement.unit;
        self.known_distance = measurement.known_distance;
        self.precision = measurement.precision;
        self.points = measurement
            .points
            .iter()
            .map(|(x, y)| format!("{x},{y}"))
            .collect::<Vec<_>>()
            .join("\n");
        self.calibration = measurement
            .calibration
            .iter()
            .map(|(x, y)| format!("{x},{y}"))
            .collect::<Vec<_>>()
            .join("\n");
        self.target = None;
        self.replace = false;
    }

    fn set_measure_unit(&mut self, unit: MeasurementUnit) {
        if unit != self.measure_unit {
            self.known_distance *= self.measure_unit.metres() / unit.metres();
            self.measure_unit = unit;
            self.clear_preview();
        }
    }
}

impl SuniApp {
    pub(crate) fn open_library_preview(
        &mut self,
        page: usize,
        entry: LibraryEntry,
        placement: LibraryPlacement,
    ) -> bool {
        if !self.ink_can_start()
            || entry.validate().is_err()
            || !self
                .document
                .as_ref()
                .is_some_and(|d| page < d.info.page_count())
        {
            return false;
        }
        self.open_rich_draft(false);
        let Some(draft) = self.annotate.adv.rich.as_mut() else {
            return false;
        };
        draft.page = page;
        draft.mode = RichMode::Library;
        draft.target = None;
        draft.replace = false;
        draft.library_entry = Some(entry);
        draft.library_placement = placement;
        self.request_rich_preview();
        true
    }

    fn save_rich_to_library(&mut self) {
        let result = (|| {
            let draft = self
                .annotate
                .adv
                .rich
                .as_ref()
                .filter(|draft| draft.library_save_supported() && !draft.busy && !draft.applying)
                .ok_or_else(|| EngineError::Other(self.t("library.unsupported").into()))?;
            if draft.library_name.trim().is_empty() {
                return Err(EngineError::Other(
                    self.t("library.metadata_required").into(),
                ));
            }
            let operation = draft
                .operation()
                .ok_or_else(|| EngineError::Other(self.t("library.unsupported").into()))?;
            LibraryEntry::from_rich_annotation(
                crate::annotation_library::new_entry_id(),
                draft.library_name.trim().into(),
                LibraryProvenance {
                    author: (!draft.library_credit.trim().is_empty())
                        .then(|| draft.library_credit.trim().into()),
                    source: (!draft.library_source.trim().is_empty())
                        .then(|| draft.library_source.trim().into()),
                    license: (!draft.library_license.trim().is_empty())
                        .then(|| draft.library_license.trim().into()),
                },
                &operation,
            )
        })();
        let result = result.and_then(|entry| self.library_add_entry(entry));
        match result {
            Ok(()) => {
                self.status_msg = Some(self.t("library.saved").into());
                if let Some(draft) = self.annotate.adv.rich.as_mut() {
                    draft.error = None;
                }
            }
            Err(error) => {
                if let Some(draft) = self.annotate.adv.rich.as_mut() {
                    draft.error = Some(error.to_string());
                }
            }
        }
    }

    pub(crate) fn open_measurement_preview(
        &mut self,
        page: usize,
        measurement: Measurement,
    ) -> bool {
        if self.annotate.adv.rich.is_some()
            || measurement.value().is_err()
            || !self
                .document
                .as_ref()
                .is_some_and(|document| page < document.info.page_count())
        {
            return false;
        }
        self.open_rich_draft(false);
        let Some(draft) = self.annotate.adv.rich.as_mut() else {
            return false;
        };
        draft.page = page;
        draft.load_measurement(&measurement);
        self.request_rich_preview();
        true
    }

    pub(crate) fn open_saved_measurement_editor(
        &mut self,
        page: usize,
        record: MeasurementRecord,
    ) -> bool {
        self.validate_measurement_context();
        if !self.measurement_ready()
            || self.annotate.measure.is_active_draft()
            || !self.annotate.measure.saved.contains(page, &record)
            || record.measurement.value().is_err()
        {
            return false;
        }
        self.open_rich_draft(false);
        let Some(draft) = self.annotate.adv.rich.as_mut() else {
            return false;
        };
        draft.page = page;
        draft.load_measurement(&record.measurement);
        draft.measurement_edit = Some(record);
        true
    }

    pub(crate) fn open_measurement_geometry_preview(
        &mut self,
        page: usize,
        original: MeasurementRecord,
        replacement: Measurement,
    ) -> bool {
        if !self.annotate.measure.saved.can_edit(page, &original)
            || replacement.kind != original.measurement.kind
            || replacement.points.len() != original.measurement.points.len()
            || replacement.calibration != original.measurement.calibration
            || replacement.unit != original.measurement.unit
            || replacement.known_distance != original.measurement.known_distance
            || replacement.precision != original.measurement.precision
            || replacement.value().is_err()
            || !self.open_saved_measurement_editor(page, original)
        {
            return false;
        }
        let draft = self.annotate.adv.rich.as_mut().unwrap();
        draft.load_measurement(&replacement);
        draft.measurement_geometry_edit = true;
        self.request_rich_preview();
        true
    }

    pub(crate) fn open_rich_from_free_text_prompt(&mut self, ctx: &egui::Context) {
        let Some(prompt) = self.annotate.adv.free_text.take() else {
            return;
        };
        if prompt.pending {
            self.annotate.adv.free_text = Some(prompt);
            return;
        }
        self.open_rich_annotations(ctx);
        if let Some(draft) = self.annotate.adv.rich.as_mut() {
            draft.page = prompt.page;
            draft.rect = prompt.rect;
            draft.text = prompt.text;
            draft.size = prompt.size;
            draft.rgb = prompt.rgb;
            draft.fill = prompt.fill;
            draft.fill_rgb = prompt.fill_rgb;
            draft.width = prompt.border;
            draft.target = prompt.index;
            draft.replace = prompt.index.is_some();
        } else {
            self.annotate.adv.free_text = Some(prompt);
        }
    }
    pub fn open_rich_annotations(&mut self, _ctx: &egui::Context) {
        self.open_rich_draft(false);
    }

    pub fn open_ink_eraser(&mut self, _ctx: &egui::Context) {
        self.open_rich_draft(true);
    }

    fn open_rich_draft(&mut self, eraser: bool) {
        if !self.ink_can_start() || self.form_builder.active() || self.edit.text_prompt.is_some() {
            return;
        }
        let Some(document) = self.document.as_ref() else {
            return;
        };
        let page = document.page;
        let selected = self
            .annotate
            .adv
            .selected
            .as_ref()
            .filter(|(p, detail)| *p == page && !detail.locked && detail.ours);
        if eraser && !selected.is_some_and(|(_, detail)| detail.subtype == "Ink") {
            self.status_msg = Some(tr(self.lang(), "rich.select_ink").into());
            return;
        }
        let selected = selected.map(|(_, detail)| detail.clone());
        let size = document.info.page_sizes[page];
        let target = selected
            .as_ref()
            .filter(|detail| detail.subtype == if eraser { "Ink" } else { "FreeText" })
            .map(|detail| detail.index);
        let rect = selected
            .as_ref()
            .filter(|detail| detail.subtype == "FreeText")
            .map(|detail| detail.rect)
            .unwrap_or(PointRect {
                left: 20.0,
                bottom: (size.height_pt - 160.0).max(20.0),
                right: 280.0f32.min(size.width_pt - 20.0),
                top: (size.height_pt - 20.0).max(40.0),
            });
        self.annotate.adv.rich_generation = self.annotate.adv.rich_generation.wrapping_add(1);
        self.annotate.adv.rich = Some(RichDraft {
            page,
            epoch: self.view.epoch,
            generation: self.annotate.adv.rich_generation,
            mode: if eraser {
                RichMode::EraseInk
            } else {
                RichMode::FreeText
            },
            rect,
            text: selected
                .as_ref()
                .filter(|detail| detail.subtype == "FreeText")
                .and_then(|detail| detail.contents.clone())
                .unwrap_or_default(),
            points: "40,70\n160,70\n100,150".into(),
            font: TextFont::Standard(FontFamily::Helvetica),
            bold: false,
            italic: false,
            size: 14.0,
            rgb: [180, 20, 35],
            fill: false,
            fill_rgb: [255, 250, 230],
            width: 1.0,
            opacity: 100.0,
            radius: 8.0,
            stamp: StampKind::Approved,
            image_path: None,
            measure_kind: MeasurementKind::Length,
            measure_unit: MeasurementUnit::Metres,
            calibration: String::new(),
            known_distance: 0.0,
            precision: 2,
            filename: String::new(),
            bytes: Vec::new(),
            target,
            replace: false,
            font_chosen: false,
            preview: None,
            prepared: false,
            preview_index: 0,
            busy: false,
            applying: false,
            error: None,
            interactive_measurement: false,
            measurement_edit: None,
            measurement_geometry_edit: false,
            author: self.annot_author(),
            library_entry: None,
            library_placement: LibraryPlacement {
                origin: (0.0, 0.0),
                scale: 1.0,
            },
            library_name: String::new(),
            library_source: String::new(),
            library_license: String::new(),
            library_credit: String::new(),
        });
        if eraser {
            if let Some(detail) = selected {
                let x = (detail.rect.left + detail.rect.right) / 2.0;
                let y = (detail.rect.bottom + detail.rect.top) / 2.0;
                self.annotate.adv.rich.as_mut().unwrap().points = format!("{x:.2},{y:.2}");
            }
        }
    }

    fn rich_ready(&self) -> bool {
        self.document.is_some()
            && matches!(self.engine, crate::app::EngineState::Ready)
            && !self.worker.has_pending_mutations()
            && self.opening.is_none()
            && self.tabs.suspending.is_none()
            && self.tabs.resuming.is_none()
            && !self.processing.busy
            && !self.office.busy
            && !self.sign.busy
            && !self.saving
            && !self.printing.busy
            && !self.ocr.busy
            && !self.certificate.busy
            && !self.document_transition_pending()
    }

    fn rich_context_current(&mut self) -> bool {
        let current = self.annotate.adv.rich.as_ref().is_some_and(|draft| {
            draft.epoch == self.view.epoch
                && self
                    .document
                    .as_ref()
                    .is_some_and(|document| draft.page < document.info.page_count())
                && self.opening.is_none()
                && self.tabs.resuming.is_none()
        });
        if !current {
            let error = tr(self.lang(), "rich.stale").to_string();
            if let Some(draft) = self.annotate.adv.rich.as_mut() {
                draft.busy = false;
                draft.preview = None;
                draft.prepared = false;
                draft.error = Some(error);
            }
        }
        current
    }

    pub(crate) fn request_rich_preview(&mut self) {
        if !self.rich_context_current() || !self.rich_ready() {
            return;
        }
        let Some(draft) = self
            .annotate
            .adv
            .rich
            .as_mut()
            .filter(|draft| !draft.busy && !draft.applying)
        else {
            return;
        };
        let request = if let Some(record) = &draft.measurement_edit {
            let Some(measurement) = draft.measurement() else {
                return;
            };
            Request::PreviewMeasurementEdit {
                page: draft.page,
                record: record.clone(),
                measurement,
                epoch: draft.epoch,
                generation: self.annotate.adv.rich_generation.wrapping_add(1),
            }
        } else {
            let Some(annotation) = draft.operation() else {
                return;
            };
            Request::PreviewRichAnnotation {
                page: draft.page,
                annotation,
                author: draft.author.clone(),
                epoch: draft.epoch,
                generation: self.annotate.adv.rich_generation.wrapping_add(1),
            }
        };
        self.annotate.adv.rich_generation = self.annotate.adv.rich_generation.wrapping_add(1);
        draft.generation = self.annotate.adv.rich_generation;
        draft.clear_preview();
        draft.busy = true;
        self.worker.send(request);
    }

    pub(crate) fn on_rich_preview(
        &mut self,
        ctx: &egui::Context,
        epoch: u64,
        generation: u64,
        result: Result<RichAnnotationPreview, EngineError>,
    ) {
        if !self.rich_context_current() || epoch != self.view.epoch {
            return;
        }
        let Some(draft) =
            self.annotate.adv.rich.as_mut().filter(|draft| {
                draft.epoch == epoch && draft.generation == generation && draft.busy
            })
        else {
            return;
        };
        draft.busy = false;
        match result {
            Ok(preview) => {
                draft.preview_index = preview.index;
                draft.prepared = true;
                draft.preview = Some(ctx.load_texture(
                    "rich-annotation-preview",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [preview.image.width as usize, preview.image.height as usize],
                        &preview.image.rgba,
                    ),
                    egui::TextureOptions::LINEAR,
                ));
            }
            Err(error) => draft.error = Some(error.to_string()),
        }
    }

    pub(crate) fn apply_rich_draft(&mut self) {
        if !self.rich_context_current() || !self.rich_ready() {
            return;
        }
        let Some(draft) = self
            .annotate
            .adv
            .rich
            .as_mut()
            .filter(|draft| draft.prepared && !draft.busy && !draft.applying)
        else {
            return;
        };
        draft.applying = true;
        self.annotate.adv.reselect = Some((draft.page, draft.preview_index));
        self.worker
            .send(Request::Edit(EditOp::CommitRichAnnotationPreview {
                epoch: draft.epoch,
                generation: draft.generation,
            }));
    }

    pub(crate) fn rich_annotation_failed(&mut self, error: &EngineError) {
        if let Some(draft) = self
            .annotate
            .adv
            .rich
            .as_mut()
            .filter(|draft| draft.applying)
        {
            draft.applying = false;
            draft.busy = false;
            draft.prepared = false;
            draft.preview = None;
            draft.error = Some(error.to_string());
        }
    }

    pub(crate) fn rich_annotation_dialog(&mut self, ctx: &egui::Context) {
        let current = self.rich_context_current();
        let ready = self.rich_ready();
        let lang = self.lang();
        let dark = self.settings.dark_theme;
        let has_icons = self.has_icons;
        let fonts = self.edit.system_fonts.clone();
        let Some(draft) = self.annotate.adv.rich.as_mut() else {
            return;
        };
        let mut save_library = false;
        let mut adjust_geometry = false;
        let (mut preview, mut apply, mut cancel, mut choose_file, mut want_fonts) =
            (false, false, false, false, false);
        let can_preview =
            current && ready && !draft.busy && !draft.applying && draft.valid_inputs();
        let can_apply = current && ready && draft.prepared && !draft.busy && !draft.applying;
        let can_cancel = !draft.applying;
        crate::dialog::show(
            ctx,
            crate::dialog::Spec::new(
                "rich-annotations",
                tr(
                    lang,
                    if draft.mode == RichMode::Library {
                        "library.preview_title"
                    } else if draft.measurement_edit.is_some() {
                        "measure.edit.title"
                    } else if draft.interactive_measurement {
                        "measure.preview_title"
                    } else {
                        "rich.title"
                    },
                ),
                dark,
                has_icons,
            )
            .subtitle(tr(
                lang,
                if draft.mode == RichMode::Library {
                    "library.preview_help"
                } else if draft.measurement_edit.is_some() {
                    "measure.edit.help"
                } else if draft.interactive_measurement {
                    "measure.page_scope"
                } else {
                    "rich.note"
                },
            ))
            .width(680.0),
            |ui| {
                let mut changed = false;
                ui.add_enabled_ui(!draft.busy && !draft.applying, |ui| {
                    if !matches!(draft.mode, RichMode::EraseInk | RichMode::Library)
                        && !draft.interactive_measurement
                    {
                        let old = draft.mode;
                        egui::ComboBox::from_id_salt("rich-kind")
                            .selected_text(tr(lang, draft.mode.key()))
                            .show_ui(ui, |ui| {
                                for mode in [
                                    RichMode::FreeText,
                                    RichMode::Polygon,
                                    RichMode::Polyline,
                                    RichMode::Cloud,
                                    RichMode::Stamp,
                                    RichMode::ImageStamp,
                                    RichMode::Measurement,
                                    RichMode::Attachment,
                                ] {
                                    ui.selectable_value(
                                        &mut draft.mode,
                                        mode,
                                        tr(lang, mode.key()),
                                    );
                                }
                            });
                        if old != draft.mode {
                            changed = true;
                            if draft.mode == RichMode::Measurement {
                                draft.points = "40,70\n160,70".into();
                            }
                            if draft.mode == RichMode::Attachment {
                                draft.rect.right = draft.rect.left + 24.0;
                                draft.rect.top = draft.rect.bottom + 32.0;
                            }
                        }
                    }
                    if matches!(
                        draft.mode,
                        RichMode::FreeText
                            | RichMode::Stamp
                            | RichMode::ImageStamp
                            | RichMode::Attachment
                    ) {
                        ui.horizontal_wrapped(|ui| {
                            for (label, value) in [
                                ("rich.left", &mut draft.rect.left),
                                ("rich.bottom", &mut draft.rect.bottom),
                                ("rich.right", &mut draft.rect.right),
                                ("rich.top", &mut draft.rect.top),
                            ] {
                                ui.label(tr(lang, label));
                                changed |= ui
                                    .add(egui::DragValue::new(value).speed(1.0).suffix(" pt"))
                                    .changed();
                            }
                        });
                    }
                    match draft.mode {
                        RichMode::Library => {
                            if let Some(entry) = &draft.library_entry {
                                ui.heading(&entry.name);
                            }
                            for (key, value) in [
                                ("library.origin_x", &mut draft.library_placement.origin.0),
                                ("library.origin_y", &mut draft.library_placement.origin.1),
                            ] {
                                ui.horizontal(|ui| {
                                    ui.label(tr(lang, key));
                                    changed |= ui
                                        .add(
                                            egui::DragValue::new(value)
                                                .range(0.0..=100000.0)
                                                .speed(1.0),
                                        )
                                        .changed();
                                });
                            }
                            ui.horizontal(|ui| {
                                ui.label(tr(lang, "library.scale"));
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut draft.library_placement.scale)
                                            .range(0.01..=100.0)
                                            .speed(0.01),
                                    )
                                    .changed();
                            });
                            ui.small(tr(lang, "library.bounds"));
                        }
                        RichMode::Measurement => {
                            if draft.interactive_measurement {
                                ui.label(tr(lang, measure_key(draft.measure_kind)));
                                if let Some(result) =
                                    draft.measurement().and_then(|m| m.result().ok())
                                {
                                    ui.heading(result["label"].as_str().unwrap_or_default());
                                } else {
                                    ui.label(tr(lang, "measure.invalid"));
                                }
                                if draft.measurement_geometry_edit {
                                    ui.small(tr(lang, "measure.geometry.preserved"));
                                    ui.monospace(&draft.calibration);
                                    ui.small(tr_args(
                                        lang,
                                        "measure.geometry.details",
                                        &[
                                            (
                                                "points",
                                                &draft
                                                    .measurement_edit
                                                    .as_ref()
                                                    .unwrap()
                                                    .measurement
                                                    .points
                                                    .len()
                                                    .to_string(),
                                            ),
                                            ("unit", draft.measure_unit.id()),
                                            ("precision", &draft.precision.to_string()),
                                        ],
                                    ));
                                    adjust_geometry = ui
                                        .add_enabled(
                                            current && ready && !draft.applying,
                                            egui::Button::new(tr(lang, "measure.geometry.adjust")),
                                        )
                                        .clicked();
                                } else if draft.measurement_edit.is_some() {
                                    if draft.measure_kind != MeasurementKind::Angle {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(tr(lang, "measure.known_distance"));
                                            changed |= ui
                                                .add(
                                                    egui::DragValue::new(&mut draft.known_distance)
                                                        .range(0.0..=1e9)
                                                        .speed(0.1),
                                                )
                                                .changed();
                                            let mut unit = draft.measure_unit;
                                            egui::ComboBox::from_id_salt("saved-measurement-unit")
                                                .selected_text(unit.id())
                                                .show_ui(ui, |ui| {
                                                    for candidate in MeasurementUnit::ALL {
                                                        ui.selectable_value(
                                                            &mut unit,
                                                            candidate,
                                                            candidate.id(),
                                                        );
                                                    }
                                                });
                                            if unit != draft.measure_unit {
                                                draft.set_measure_unit(unit);
                                                changed = true;
                                            }
                                        });
                                        ui.small(tr(lang, "measure.edit.reference"));
                                        ui.monospace(&draft.calibration);
                                    }
                                    ui.horizontal(|ui| {
                                        ui.label(tr(lang, "measure.precision"));
                                        changed |= ui
                                            .add(
                                                egui::DragValue::new(&mut draft.precision)
                                                    .range(0..=4),
                                            )
                                            .changed();
                                    });
                                    ui.small(tr(lang, "measure.edit.geometry"));
                                }
                            } else {
                                ui.label(tr(lang, "measure.note"));
                                let old = draft.measure_kind;
                                egui::ComboBox::from_id_salt("measure-kind")
                                    .selected_text(tr(lang, measure_key(draft.measure_kind)))
                                    .show_ui(ui, |ui| {
                                        for kind in MeasurementKind::ALL {
                                            ui.selectable_value(
                                                &mut draft.measure_kind,
                                                kind,
                                                tr(lang, measure_key(kind)),
                                            );
                                        }
                                    });
                                changed |= old != draft.measure_kind;
                                ui.label(tr(lang, "rich.points"));
                                changed |= ui
                                    .add(
                                        egui::TextEdit::multiline(&mut draft.points)
                                            .desired_rows(4)
                                            .char_limit(8192)
                                            .desired_width(f32::INFINITY),
                                    )
                                    .changed();
                                ui.label(tr(lang, "measure.calibration"));
                                changed |= ui
                                    .add(
                                        egui::TextEdit::multiline(&mut draft.calibration)
                                            .desired_rows(2)
                                            .char_limit(512)
                                            .desired_width(f32::INFINITY),
                                    )
                                    .changed();
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(tr(lang, "measure.known_distance"));
                                    changed |= ui
                                        .add(
                                            egui::DragValue::new(&mut draft.known_distance)
                                                .range(0.0..=1e9)
                                                .speed(0.1),
                                        )
                                        .changed();
                                    let mut unit = draft.measure_unit;
                                    egui::ComboBox::from_id_salt("measure-unit")
                                        .selected_text(draft.measure_unit.id())
                                        .show_ui(ui, |ui| {
                                            for candidate in MeasurementUnit::ALL {
                                                ui.selectable_value(
                                                    &mut unit,
                                                    candidate,
                                                    candidate.id(),
                                                );
                                            }
                                        });
                                    if unit != draft.measure_unit {
                                        draft.set_measure_unit(unit);
                                        changed = true;
                                    }
                                    ui.label(tr(lang, "measure.precision"));
                                    changed |= ui
                                        .add(
                                            egui::DragValue::new(&mut draft.precision).range(0..=4),
                                        )
                                        .changed();
                                });
                                if let Some(result) =
                                    draft.measurement().and_then(|m| m.result().ok())
                                {
                                    ui.strong(result["label"].as_str().unwrap_or_default());
                                } else {
                                    ui.label(tr(lang, "measure.invalid"));
                                }
                            }
                        }
                        RichMode::FreeText => {
                            changed |= ui
                                .add(
                                    egui::TextEdit::multiline(&mut draft.text)
                                        .desired_rows(4)
                                        .desired_width(f32::INFINITY)
                                        .hint_text(tr(lang, "dialog.free_text.placeholder")),
                                )
                                .changed();
                            if draft.target.is_some() {
                                changed |= ui
                                    .checkbox(&mut draft.replace, tr(lang, "rich.replace_selected"))
                                    .changed();
                            }
                            let old = draft.font.clone();
                            let (wanted, _) = crate::inline_text::font_picker(
                                ui,
                                lang,
                                &mut draft.font,
                                &mut self.edit.font_filter,
                                fonts.as_deref().map(|fonts| fonts.as_slice()),
                            );
                            want_fonts |= wanted;
                            if old != draft.font {
                                draft.font_chosen = true;
                                changed = true;
                            }
                            if ui.button(tr(lang, "rich.confirm_font")).clicked() {
                                draft.font_chosen = true;
                                changed = true;
                            }
                            ui.horizontal_wrapped(|ui| {
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut draft.size)
                                            .range(4.0..=200.0)
                                            .suffix(" pt"),
                                    )
                                    .changed();
                                ui.add_enabled_ui(
                                    matches!(draft.font, TextFont::Standard(_)),
                                    |ui| {
                                        changed |= ui
                                            .toggle_value(
                                                &mut draft.bold,
                                                egui::RichText::new("B").strong(),
                                            )
                                            .changed();
                                        changed |= ui
                                            .toggle_value(
                                                &mut draft.italic,
                                                egui::RichText::new("I").italics(),
                                            )
                                            .changed();
                                    },
                                );
                            });
                            ui.label(tr(lang, "rich.font_note"));
                        }
                        RichMode::Polygon
                        | RichMode::Polyline
                        | RichMode::Cloud
                        | RichMode::EraseInk => {
                            ui.label(tr(lang, "rich.points"));
                            changed |= ui
                                .add(
                                    egui::TextEdit::multiline(&mut draft.points)
                                        .desired_rows(4)
                                        .desired_width(f32::INFINITY),
                                )
                                .changed();
                            if draft.mode == RichMode::EraseInk {
                                ui.label(tr(lang, "rich.eraser_note"));
                                ui.label(tr(lang, "rich.radius"));
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut draft.radius)
                                            .range(0.1..=100.0)
                                            .suffix(" pt"),
                                    )
                                    .changed();
                            }
                        }
                        RichMode::Stamp => {
                            let old = draft.stamp;
                            egui::ComboBox::from_id_salt("rich-stamp")
                                .selected_text(tr(lang, stamp_key(draft.stamp)))
                                .show_ui(ui, |ui| {
                                    for stamp in [
                                        StampKind::Approved,
                                        StampKind::Draft,
                                        StampKind::Confidential,
                                        StampKind::Rejected,
                                    ] {
                                        ui.selectable_value(
                                            &mut draft.stamp,
                                            stamp,
                                            tr(lang, stamp_key(stamp)),
                                        );
                                    }
                                });
                            changed |= old != draft.stamp;
                        }
                        RichMode::ImageStamp => {
                            if ui.button(tr(lang, "rich.choose_image")).clicked() {
                                choose_file = true;
                            }
                            if let Some(path) = &draft.image_path {
                                ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                            }
                            ui.label(tr(lang, "rich.image_stamp_note"));
                            ui.horizontal(|ui| {
                                ui.label(tr(lang, "bar.opacity"));
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut draft.opacity)
                                            .range(1.0..=100.0)
                                            .suffix(" %"),
                                    )
                                    .changed();
                            });
                        }
                        RichMode::Attachment => {
                            if ui.button(tr(lang, "rich.choose_file")).clicked() {
                                choose_file = true;
                            }
                            ui.label(&draft.filename);
                            ui.label(tr(lang, "rich.attachment_note"));
                        }
                    }
                    if matches!(
                        draft.mode,
                        RichMode::FreeText
                            | RichMode::Polygon
                            | RichMode::Polyline
                            | RichMode::Cloud
                    ) {
                        ui.horizontal_wrapped(|ui| {
                            changed |= ui.color_edit_button_srgb(&mut draft.rgb).changed();
                            ui.label(tr(lang, "bar.width"));
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut draft.width)
                                        .range(if draft.mode == RichMode::FreeText {
                                            0.0..=50.0
                                        } else {
                                            0.1..=50.0
                                        })
                                        .suffix(" pt"),
                                )
                                .changed();
                            ui.label(tr(lang, "bar.opacity"));
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut draft.opacity)
                                        .range(0.0..=100.0)
                                        .suffix(" %"),
                                )
                                .changed();
                            if draft.mode != RichMode::Polyline {
                                changed |=
                                    ui.checkbox(&mut draft.fill, tr(lang, "bar.fill")).changed();
                                if draft.fill {
                                    changed |=
                                        ui.color_edit_button_srgb(&mut draft.fill_rgb).changed();
                                }
                            }
                        });
                    }
                });
                if draft.library_save_supported() {
                    ui.add_enabled_ui(!draft.busy && !draft.applying, |ui| {
                        egui::CollapsingHeader::new(tr(lang, "library.save_draft")).show(
                            ui,
                            |ui| {
                                ui.label(tr(lang, "library.save_help"));
                                ui.small(tr(lang, "library.provenance_help"));
                                for (key, value) in [
                                    ("library.name", &mut draft.library_name),
                                    ("library.source", &mut draft.library_source),
                                    ("library.license", &mut draft.library_license),
                                    ("library.credit", &mut draft.library_credit),
                                ] {
                                    ui.horizontal(|ui| {
                                        ui.label(tr(lang, key));
                                        ui.add(egui::TextEdit::singleline(value).char_limit(256));
                                    });
                                }
                                let complete = !draft.library_name.trim().is_empty();
                                save_library = ui
                                    .add_enabled(
                                        complete,
                                        egui::Button::new(tr(lang, "library.save")),
                                    )
                                    .clicked();
                            },
                        );
                    });
                }
                if changed {
                    draft.clear_preview();
                }
                if draft.busy || draft.applying {
                    ui.spinner();
                }
                if let Some(error) = &draft.error {
                    ui.colored_label(crate::theme::palette(dark).error, error);
                }
                if let Some(texture) = &draft.preview {
                    ui.add(
                        egui::Image::new(texture).fit_to_exact_size(
                            texture.size_vec2()
                                * (600.0 / texture.size_vec2().x)
                                    .min(240.0 / texture.size_vec2().y)
                                    .min(1.0),
                        ),
                    );
                }
            },
            |ui| {
                ui.add_enabled_ui(can_apply, |ui| {
                    apply = crate::dialog::primary(ui, tr(lang, "bar.apply"));
                });
                ui.add_enabled_ui(can_preview, |ui| {
                    preview = crate::dialog::secondary(ui, tr(lang, "rich.preview"));
                });
                ui.add_enabled_ui(can_cancel, |ui| {
                    cancel = crate::dialog::secondary(ui, tr(lang, "dialog.password.cancel"))
                        || ui.input_mut(|input| {
                            input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                        });
                });
            },
        );
        if want_fonts && !self.edit.fonts_requested {
            self.edit.fonts_requested = true;
            self.worker.send(Request::SystemFonts);
        }
        if cancel || adjust_geometry {
            let draft = self.annotate.adv.rich.take().unwrap();
            self.worker.send(Request::DiscardRichAnnotationPreview {
                epoch: draft.epoch,
                generation: draft.generation,
            });
            self.annotate.adv.rich_generation = self.annotate.adv.rich_generation.wrapping_add(1);
            if adjust_geometry {
                if let (Some(original), Some(replacement)) =
                    (draft.measurement_edit.clone(), draft.measurement())
                {
                    self.restore_measurement_geometry(draft.page, original, replacement);
                }
            }
        } else if choose_file {
            if self
                .annotate
                .adv
                .rich
                .as_ref()
                .is_some_and(|draft| draft.mode == RichMode::ImageStamp)
            {
                self.choose_rich_image();
            } else {
                self.choose_rich_attachment();
            }
        } else if save_library {
            self.save_rich_to_library();
        } else if preview {
            self.request_rich_preview();
        } else if apply {
            self.apply_rich_draft();
        }
    }

    fn choose_rich_image(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title(tr(self.lang(), "rich.choose_image"))
            .add_filter(
                "Images",
                &["png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff", "webp"],
            )
            .pick_file()
        else {
            return;
        };
        if let Some(draft) = self
            .annotate
            .adv
            .rich
            .as_mut()
            .filter(|draft| draft.mode == RichMode::ImageStamp)
        {
            draft.image_path = Some(path);
            draft.clear_preview();
        }
    }

    fn choose_rich_attachment(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title(tr(self.lang(), "rich.choose_file"))
            .pick_file()
        else {
            return;
        };
        let result = (|| {
            let filename = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| EngineError::Other(tr(self.lang(), "rich.file_error").into()))?
                .to_string();
            let mut bytes = Vec::new();
            std::fs::File::open(path)?
                .take(16 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > 16 * 1024 * 1024 {
                return Err(EngineError::Other(
                    tr(self.lang(), "rich.file_error").into(),
                ));
            }
            Ok((filename, bytes))
        })();
        if let Some(draft) = self.annotate.adv.rich.as_mut() {
            draft.clear_preview();
            match result {
                Ok((filename, bytes)) => {
                    draft.filename = filename;
                    draft.bytes = bytes;
                }
                Err(error) => draft.error = Some(error.to_string()),
            }
        }
    }
}

fn stamp_key(kind: StampKind) -> &'static str {
    match kind {
        StampKind::Approved => "rich.stamp_approved",
        StampKind::Draft => "rich.stamp_draft",
        StampKind::Confidential => "rich.stamp_confidential",
        StampKind::Rejected => "rich.stamp_rejected",
    }
}

fn measure_key(kind: MeasurementKind) -> &'static str {
    match kind {
        MeasurementKind::Length => "measure.length",
        MeasurementKind::Polyline => "measure.polyline",
        MeasurementKind::Perimeter => "measure.perimeter",
        MeasurementKind::Area => "measure.area",
        MeasurementKind::Angle => "measure.angle",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interactive_measurement_bridge_preserves_geometry_and_only_requests_a_preview() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let measurement = Measurement {
            kind: MeasurementKind::Area,
            points: vec![(40.0, 40.0), (140.0, 40.0), (140.0, 140.0)],
            calibration: [(40.0, 20.0), (140.0, 20.0)],
            known_distance: 10.0,
            unit: MeasurementUnit::Metres,
            precision: 2,
        };
        assert!(app.open_measurement_preview(0, measurement.clone()));
        let draft = app.annotate.adv.rich.as_ref().unwrap();
        assert!(draft.interactive_measurement && draft.busy && !draft.prepared);
        assert_eq!(draft.measurement().unwrap().points, measurement.points);
        assert_eq!(
            draft.measurement().unwrap().calibration,
            measurement.calibration
        );
        assert!((draft.measurement().unwrap().value().unwrap() - 50.0).abs() < 1e-8);
        assert!(matches!(
            requests.recv().unwrap(),
            Request::PreviewRichAnnotation {
                page: 0,
                annotation: RichAnnotation::Measurement(_),
                ..
            }
        ));
        app.apply_rich_draft();
        assert!(
            requests.try_recv().is_err(),
            "preview must complete before Apply"
        );
        assert!(
            !app.open_measurement_preview(0, measurement),
            "a live preview cannot be replaced"
        );
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn library_create_from_a_selected_annotation_saves_only_a_new_explicit_draft() {
        for language in ["en-US", "fr-FR"] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            let dir = crate::annotation_library::tests::storage(&mut app);
            app.settings.language = language.into();
            app.annotate.adv.selected = Some((
                0,
                sunipdf_core::AnnotDetail {
                    index: 7,
                    subtype: "FreeText".into(),
                    rect: PointRect {
                        left: 30.0,
                        bottom: 40.0,
                        right: 180.0,
                        top: 100.0,
                    },
                    contents: Some("Existing private document text".into()),
                    author: None,
                    modified: None,
                    name: Some("existing-identity".into()),
                    rgb: None,
                    fill_rgb: None,
                    width_pt: None,
                    opacity: 1.0,
                    dash_pattern: vec![],
                    font_size: None,
                    parent: None,
                    status: None,
                    link: None,
                    arrow: false,
                    hidden: false,
                    locked: false,
                    ours: true,
                    can_move: true,
                    can_resize: true,
                    can_restyle: true,
                    can_edit_text: true,
                    can_edit_link: false,
                    replies: 0,
                },
            ));
            let mut controls = crate::annotation_library::tests::Controls::new();
            controls.click(&mut app, "library.short");
            controls.click(&mut app, "library.create");
            let draft = app.annotate.adv.rich.as_mut().expect("new template draft");
            assert_eq!(draft.target, None);
            assert!(draft.text.is_empty());
            assert!(draft.library_save_supported());
            draft.mode = RichMode::Stamp;
            draft.library_name = "Personal approval".into();
            controls.click(&mut app, "library.save_draft");
            controls.click(&mut app, "library.save");
            let path = dir.path().join("annotations.json");
            let bytes = std::fs::read(&path).unwrap();
            let library =
                sunipdf_core::annotation_library::AnnotationLibrary::from_json(&bytes).unwrap();
            assert_eq!(library.entries().len(), 1);
            assert_eq!(library.entries()[0].name, "Personal approval");
            assert_eq!(
                library.entries()[0].provenance,
                LibraryProvenance::default()
            );
            assert!(matches!(
                library.entries()[0].primitives.as_slice(),
                [sunipdf_core::annotation_library::LibraryPrimitive::Stamp { .. }]
            ));
            assert!(app.annotate.adv.rich.is_some());
            assert!(
                requests.try_recv().is_err(),
                "saving a local template never edits the PDF"
            );
            controls.click(&mut app, "library.save");
            assert!(
                app.annotate.adv.rich.as_ref().unwrap().error.is_some(),
                "duplicate names are refused"
            );
            assert_eq!(std::fs::read(path).unwrap(), bytes);
            app.annotate.adv.rich.as_mut().unwrap().mode = RichMode::Measurement;
            assert!(!app
                .annotate
                .adv
                .rich
                .as_ref()
                .unwrap()
                .library_save_supported());
            assert!(!app.dirty);
        }
    }

    #[test]
    fn measurement_draft_requires_calibration_and_preview_before_any_mutation() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.open_rich_annotations(&egui::Context::default());
        let draft = app.annotate.adv.rich.as_mut().unwrap();
        draft.mode = RichMode::Measurement;
        draft.points = "40,70\n160,70".into();
        assert!(
            draft.measurement().is_none(),
            "manual entry has no implicit physical scale"
        );
        draft.known_distance = 10.0;
        draft.calibration = "40,40\n40,40".into();
        assert!(!draft.valid_inputs() && draft.operation().is_none());
        draft.calibration = "40,40\n140,40".into();
        assert!((draft.measurement().unwrap().value().unwrap() - 12.0).abs() < 1e-8);
        app.apply_rich_draft();
        assert!(requests.try_recv().is_err());
        app.request_rich_preview();
        assert!(matches!(
            requests.recv().unwrap(),
            Request::PreviewRichAnnotation {
                annotation: RichAnnotation::Measurement(_),
                ..
            }
        ));
    }

    #[test]
    fn manual_measurement_controls_do_not_invent_a_positive_reference_distance() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.open_rich_annotations(&ctx);
        let draft = app.annotate.adv.rich.as_mut().unwrap();
        draft.mode = RichMode::Measurement;
        draft.points = "40,70\n160,70".into();
        draft.calibration = "40,40\n140,40".into();
        for _ in 0..3 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 1000.0),
                    )),
                    ..Default::default()
                },
                |_| app.rich_annotation_dialog(&ctx),
            );
            output.textures_delta.clear();
        }
        let draft = app.annotate.adv.rich.as_ref().unwrap();
        assert_eq!(draft.known_distance, 0.0);
        assert!(!draft.valid_inputs());
        app.request_rich_preview();
        app.apply_rich_draft();
        assert!(requests.try_recv().is_err());
    }

    #[test]
    fn saved_measurement_unit_changes_preserve_physical_size_and_references_recalculate() {
        for kind in [MeasurementKind::Length, MeasurementKind::Area] {
            let (mut app, _, _) = crate::app::tests::test_app();
            app.open_rich_annotations(&egui::Context::default());
            let measurement = Measurement {
                kind,
                points: if kind == MeasurementKind::Length {
                    vec![(40.0, 70.0), (140.0, 70.0)]
                } else {
                    vec![(40.0, 70.0), (140.0, 70.0), (140.0, 170.0)]
                },
                calibration: [(40.0, 40.0), (140.0, 40.0)],
                known_distance: 10.0,
                unit: MeasurementUnit::Metres,
                precision: 2,
            };
            let record = MeasurementRecord {
                index: 3,
                annotation_id: Some("measurement-3".into()),
                measurement: measurement.clone(),
            };
            let draft = app.annotate.adv.rich.as_mut().unwrap();
            draft.load_measurement(&measurement);
            draft.measurement_edit = Some(record);
            let exponent = if kind == MeasurementKind::Area { 2 } else { 1 };
            for unit in MeasurementUnit::ALL {
                draft.prepared = true;
                draft.set_measure_unit(unit);
                let edited = draft.measurement().unwrap();
                let physical = edited.value().unwrap() * unit.metres().powi(exponent);
                assert!((physical - measurement.value().unwrap()).abs() < 1e-8);
                assert_eq!(edited.points, measurement.points);
                assert_eq!(edited.calibration, measurement.calibration);
                assert!(
                    draft.operation().is_none(),
                    "an edit can never fall through to insertion"
                );
            }
            draft.known_distance *= 2.0;
            let edited = draft.measurement().unwrap();
            let physical = edited.value().unwrap() * edited.unit.metres().powi(exponent);
            assert!((physical - measurement.value().unwrap() * 2.0f64.powi(exponent)).abs() < 1e-8);
            draft.points = "50,70\n140,70".into();
            assert!(
                draft.measurement().is_none(),
                "this editor must preserve original vertices"
            );
        }
    }

    #[test]
    fn geometry_preview_changes_only_existing_vertices_in_explicit_geometry_mode() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.open_rich_annotations(&egui::Context::default());
        let measurement = Measurement {
            kind: MeasurementKind::Polyline,
            points: vec![(40.0, 70.0), (140.0, 70.0), (140.0, 170.0)],
            calibration: [(40.0, 40.0), (140.0, 40.0)],
            known_distance: 10.0,
            unit: MeasurementUnit::Feet,
            precision: 3,
        };
        let record = MeasurementRecord {
            index: 7,
            annotation_id: Some("saved-geometry".into()),
            measurement: measurement.clone(),
        };
        let draft = app.annotate.adv.rich.as_mut().unwrap();
        draft.load_measurement(&measurement);
        draft.measurement_edit = Some(record);
        draft.points = "40,70\n150,90\n140,170".into();
        assert!(draft.measurement().is_none());
        draft.measurement_geometry_edit = true;
        assert!(draft.measurement().is_some());
        assert!(
            draft.operation().is_none(),
            "geometry editing cannot insert a new annotation"
        );
        for invalid in 0..5 {
            draft.load_measurement(&measurement);
            match invalid {
                0 => draft.points.push_str("\n200,200"),
                1 => draft.known_distance = 20.0,
                2 => draft.precision = 2,
                3 => draft.measure_unit = MeasurementUnit::Metres,
                _ => draft.calibration = "40,40\n160,40".into(),
            }
            assert!(
                draft.measurement().is_none(),
                "geometry mode must preserve calibration and cardinality"
            );
        }
        assert!(requests.try_recv().is_err());
    }

    fn preview() -> RichAnnotationPreview {
        RichAnnotationPreview {
            image: sunipdf_core::RenderedPage {
                page_index: 0,
                width: 4,
                height: 2,
                rgba: vec![255; 32],
            },
            scale: 1.0,
            region: sunipdf_core::PixelRect::new(0, 0, 4, 2),
            index: 7,
            prepared: None,
        }
    }

    #[test]
    fn image_stamp_draft_requires_a_file_and_repreview_after_settings_change() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.open_rich_annotations(&egui::Context::default());
        let draft = app.annotate.adv.rich.as_mut().unwrap();
        draft.mode = RichMode::ImageStamp;
        assert!(!draft.valid_inputs() && draft.operation().is_none());
        draft.image_path = Some(PathBuf::from("chosen logo.png"));
        draft.opacity = 50.0;
        app.apply_rich_draft();
        assert!(requests.try_recv().is_err());
        app.request_rich_preview();
        let (epoch, generation) = match requests.recv().unwrap() {
            Request::PreviewRichAnnotation {
                annotation: RichAnnotation::ImageStamp { image, opacity, .. },
                epoch,
                generation,
                ..
            } => {
                assert_eq!(image, PathBuf::from("chosen logo.png"));
                assert_eq!(opacity, 0.5);
                (epoch, generation)
            }
            _ => panic!("image stamp preview"),
        };
        app.on_rich_preview(&egui::Context::default(), epoch, generation, Ok(preview()));
        let draft = app.annotate.adv.rich.as_mut().unwrap();
        assert!(draft.prepared);
        draft.opacity = 25.0;
        draft.clear_preview();
        app.apply_rich_draft();
        assert!(
            requests.try_recv().is_err(),
            "changed settings need an explicit fresh preview"
        );
    }

    #[test]
    fn image_stamp_dialog_exposes_localized_controls_and_escape_never_applies() {
        for language in ["en-US", "fr-FR"] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            app.settings.language = language.into();
            let ctx = egui::Context::default();
            let _ = crate::icons::install(&ctx);
            ctx.enable_accesskit();
            app.open_rich_annotations(&ctx);
            let draft = app.annotate.adv.rich.as_mut().unwrap();
            draft.mode = RichMode::ImageStamp;
            draft.image_path = Some(PathBuf::from("chosen image.png"));
            draft.prepared = true;
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 900.0),
                )),
                ..Default::default()
            };
            let mut found = false;
            for _ in 0..2 {
                let mut output = ctx.run_ui(input.clone(), |_| app.rich_annotation_dialog(&ctx));
                if let Some(tree) = output.platform_output.accesskit_update.as_ref() {
                    found |= tree
                        .nodes
                        .iter()
                        .any(|(_, node)| node.label() == Some(tr(app.lang(), "rich.choose_image")));
                }
                output.textures_delta.clear();
            }
            assert!(found);
            input.events.push(egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
            let mut output = ctx.run_ui(input, |_| app.rich_annotation_dialog(&ctx));
            output.textures_delta.clear();
            assert!(app.annotate.adv.rich.is_none());
            assert!(requests
                .try_iter()
                .all(|request| matches!(request, Request::DiscardRichAnnotationPreview { .. })));
        }
    }

    #[test]
    fn rich_draft_requires_preview_preserves_errors_and_rejects_late_tokens() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.open_rich_annotations(&ctx);
        app.annotate.adv.rich.as_mut().unwrap().text = "Keep this draft".into();
        assert!(app.has_unsaved_changes() && app.dialog_open());
        for command in [
            sunipdf_core::Command::Save,
            sunipdf_core::Command::OpenDocument,
            sunipdf_core::Command::Quit,
        ] {
            app.execute(&ctx, command);
        }
        app.apply_rich_draft();
        assert!(requests.try_recv().is_err());
        app.request_rich_preview();
        let (epoch, generation) = match requests.recv().unwrap() {
            Request::PreviewRichAnnotation {
                epoch, generation, ..
            } => (epoch, generation),
            _ => panic!("rich preview"),
        };
        app.on_rich_preview(&ctx, epoch, generation.wrapping_sub(1), Ok(preview()));
        assert!(app.annotate.adv.rich.as_ref().unwrap().busy);
        app.on_rich_preview(
            &ctx,
            epoch,
            generation,
            Err(EngineError::MissingGlyphs("🚀".into())),
        );
        let draft = app.annotate.adv.rich.as_ref().unwrap();
        assert!(!draft.busy && !draft.prepared && draft.error.is_some());
        assert_eq!(draft.text, "Keep this draft");
        app.request_rich_preview();
        let generation = match requests.recv().unwrap() {
            Request::PreviewRichAnnotation { generation, .. } => generation,
            _ => panic!("retry"),
        };
        app.on_rich_preview(&ctx, epoch, generation, Ok(preview()));
        app.apply_rich_draft();
        assert!(
            matches!(requests.recv().unwrap(),Request::Edit(EditOp::CommitRichAnnotationPreview {epoch:sent_epoch,generation:sent_generation}) if sent_epoch==epoch && sent_generation==generation)
        );
        assert!(matches!(requests.recv().unwrap(), Request::FinishMutation));
        assert_eq!(app.annotate.adv.reselect, Some((0, 7)));
        app.rich_annotation_failed(&EngineError::Other("stale candidate".into()));
        app.worker.finish_mutation();
        let draft = app.annotate.adv.rich.as_ref().unwrap();
        assert!(!draft.applying && !draft.prepared && draft.error.is_some());
        assert_eq!(draft.text, "Keep this draft");
        app.view.bump_epoch();
        app.on_rich_preview(&ctx, epoch, generation, Ok(preview()));
        app.request_rich_preview();
        assert!(requests.try_recv().is_err());
        assert!(app.annotate.adv.rich.as_ref().unwrap().error.is_some());
        app.annotate.adv.invalidate();
        app.on_rich_preview(&ctx, epoch, generation, Ok(preview()));
        assert!(app.annotate.adv.rich.is_none());
    }

    #[test]
    fn rich_dialog_english_french_escape_discards_candidate_without_applying() {
        for language in ["en-US", "fr-FR"] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            app.settings.language = language.into();
            let ctx = egui::Context::default();
            let _ = crate::icons::install(&ctx);
            ctx.enable_accesskit();
            app.open_rich_annotations(&ctx);
            app.annotate.adv.rich.as_mut().unwrap().text = "Draft".into();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 900.0),
                )),
                ..Default::default()
            };
            let mut found = false;
            for _ in 0..2 {
                let mut output = ctx.run_ui(input.clone(), |_| app.rich_annotation_dialog(&ctx));
                if let Some(tree) = output.platform_output.accesskit_update.as_ref() {
                    found |= tree
                        .nodes
                        .iter()
                        .any(|(_, node)| node.label() == Some(tr(app.lang(), "rich.preview")));
                }
                output.textures_delta.clear();
            }
            assert!(found);
            let mut input = input;
            input.events.push(egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
            let mut output = ctx.run_ui(input, |_| app.rich_annotation_dialog(&ctx));
            output.textures_delta.clear();
            assert!(app.annotate.adv.rich.is_none());
            assert!(requests.try_iter().all(|request| matches!(
                request,
                Request::DiscardRichAnnotationPreview { .. } | Request::SystemFonts
            )));
        }
    }

    #[test]
    fn free_text_bridge_preserves_input_and_existing_font_replacement_is_explicit() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.annotate.tool = crate::annotate::Tool::FreeText;
        app.annotate.adv.free_text = Some(crate::annot_adv::FreeTextPrompt {
            page: 0,
            rect: PointRect {
                left: 40.0,
                bottom: 100.0,
                right: 280.0,
                top: 220.0,
            },
            index: Some(2),
            text: "Existing text 中文".into(),
            size: 18.0,
            rgb: [20, 30, 40],
            fill: true,
            fill_rgb: [240, 240, 220],
            border: 1.0,
            focus: false,
            epoch: app.view.epoch,
            tool: app.annotate.tool,
            workspace: app.annotate.workspace,
            pending: false,
        });
        app.open_rich_from_free_text_prompt(&ctx);
        assert!(app.annotate.adv.free_text.is_none());
        let draft = app.annotate.adv.rich.as_ref().unwrap();
        assert_eq!(draft.text, "Existing text 中文");
        assert_eq!(draft.target, Some(2));
        assert!(draft.replace && !draft.font_chosen);
        assert!(draft.operation().is_none());
        app.request_rich_preview();
        assert!(
            requests.try_recv().is_err(),
            "existing annotation needs explicit font choice"
        );
        app.annotate.adv.rich.as_mut().unwrap().font_chosen = true;
        assert!(matches!(
            app.annotate.adv.rich.as_ref().unwrap().operation(),
            Some(RichAnnotation::FreeText { index: Some(2), .. })
        ));
        assert!(points("1,2\n3,4", 512).is_some());
        assert!(points("NaN,2", 512).is_none());
        assert!(points("1,2,3", 512).is_none());
    }
}
