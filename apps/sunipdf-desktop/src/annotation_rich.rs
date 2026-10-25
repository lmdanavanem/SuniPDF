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
use sunipdf_core::measurement::{
    Measurement, MeasurementDepth, MeasurementKind, MeasurementRecord, MeasurementUnit,
    PageCalibration,
};
use sunipdf_core::{AnnotStyle, EngineError, FontFamily, PointRect, TextStyle};
use sunipdf_i18n::{tr, tr_args};

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
    measure_holes: Vec<Vec<(f32, f32)>>,
    measure_depth: Option<MeasurementDepth>,
    pub calibration: String,
    pub known_distance: f64,
    pub precision: usize,
    pub filename: String,
    pub bytes: Vec<u8>,
    pub target: Option<usize>,
    pub replace: bool,
    pub font_chosen: bool,
    pub preview: Option<RichPreview>,
    pub prepared: bool,
    pub preview_index: usize,
    pub busy: bool,
    pub applying: bool,
    pub error: Option<String>,
    interactive_measurement: bool,
    measurement_edit: Option<MeasurementRecord>,
    measurement_recalibration: Option<PageCalibration>,
    measurement_geometry_edit: bool,
    rectangle: Option<crate::measurement_shapes_ui::RectanglePlacement>,
    author: Option<String>,
    library_entry: Option<LibraryEntry>,
    library_placement: LibraryPlacement,
    library_name: String,
    library_source: String,
    library_license: String,
    library_credit: String,
}

pub struct RichPreview {
    texture: egui::TextureHandle,
    source: sunipdf_core::RenderedPage,
    logical_size: egui::Vec2,
    pixels_per_point: f32,
}

fn filtered_rich_preview(
    source: &sunipdf_core::RenderedPage,
    pixels_per_point: f32,
) -> Result<(egui::ColorImage, egui::Vec2), EngineError> {
    let pixels = u64::from(source.width) * u64::from(source.height);
    if pixels == 0
        || pixels > 4 * 1024 * 1024
        || source.rgba.len() as u64 != pixels * 4
        || !pixels_per_point.is_finite()
        || pixels_per_point <= 0.0
    {
        return Err(EngineError::RenderTooLarge);
    }
    let source_size = egui::vec2(source.width as f32, source.height as f32);
    let fit = (600.0 / source_size.x).min(240.0 / source_size.y).min(1.0);
    let logical_size = source_size * fit;
    let pixel_scale = (fit * pixels_per_point).min(1.0);
    let width = (source.width as f32 * pixel_scale).round().max(1.0) as u32;
    let height = (source.height as f32 * pixel_scale).round().max(1.0) as u32;
    let image = image::RgbaImage::from_raw(source.width, source.height, source.rgba.clone())
        .ok_or(EngineError::RenderTooLarge)?;
    // A texture's bilinear sampler skips thin strokes when minifying by several pixels.
    // Filter the whole footprint before upload; the PDF candidate remains untouched.
    let image = if (width, height) == image.dimensions() {
        image
    } else {
        image::imageops::resize(&image, width, height, image::imageops::FilterType::Triangle)
    };
    Ok((
        egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], image.as_raw()),
        logical_size,
    ))
}

impl RichPreview {
    fn new(ctx: &egui::Context, source: sunipdf_core::RenderedPage) -> Result<Self, EngineError> {
        let pixels_per_point = ctx.pixels_per_point();
        let (image, logical_size) = filtered_rich_preview(&source, pixels_per_point)?;
        Ok(Self {
            texture: ctx.load_texture(
                "rich-annotation-preview",
                image,
                egui::TextureOptions::LINEAR,
            ),
            source,
            logical_size,
            pixels_per_point,
        })
    }

    fn show(&mut self, ui: &mut egui::Ui) {
        let density = ui.ctx().pixels_per_point();
        if density != self.pixels_per_point {
            if let Ok((image, _)) = filtered_rich_preview(&self.source, density) {
                self.texture.set(image, egui::TextureOptions::LINEAR);
                self.pixels_per_point = density;
            }
        }
        let fit = (ui.available_width() / self.logical_size.x).clamp(0.0, 1.0);
        ui.add(egui::Image::new(&self.texture).fit_to_exact_size(self.logical_size * fit));
    }
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
        if let Some(rectangle) = &self.rectangle {
            return self.mode == RichMode::Measurement && rectangle.valid();
        }
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
        if self.rectangle.is_some() {
            return None;
        }
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
            holes: self.measure_holes.clone(),
            depth: self.measure_depth,
            kind: self.measure_kind,
            points: points(&self.points, 128)?,
            calibration,
            known_distance: self.known_distance,
            unit: self.measure_unit,
            precision: self.precision,
        };
        measurement.value().ok()?;
        if self.measurement_edit.as_ref().is_some_and(|record| {
            (!record.validity.is_current() && self.measurement_recalibration.is_none())
                || (self.measurement_recalibration.is_some()
                    && measurement.kind == MeasurementKind::Angle)
                || measurement.kind != record.measurement.kind
                || (!self.measurement_geometry_edit
                    && (measurement.points != record.measurement.points
                        || measurement.holes != record.measurement.holes))
                || !crate::measurement_geometry::geometry_count_valid(
                    measurement.kind,
                    measurement.points.len(),
                )
                || (self.measurement_recalibration.is_none()
                    && measurement.calibration != record.measurement.calibration)
                || self
                    .measurement_recalibration
                    .as_ref()
                    .is_some_and(|calibration| {
                        measurement.calibration != calibration.points
                            || measurement.known_distance != calibration.known_distance
                            || measurement.unit != calibration.unit
                    })
                || (self.measurement_geometry_edit
                    && (measurement.known_distance != record.measurement.known_distance
                        || measurement.unit != record.measurement.unit
                        || measurement.depth != record.measurement.depth
                        || measurement.precision != record.measurement.precision))
        }) {
            return None;
        }
        Some(measurement)
    }

    fn use_current_measurement_reference(&mut self, calibration: PageCalibration) -> bool {
        if self.measurement_edit.is_none()
            || self.measurement_geometry_edit
            || self.measure_kind == MeasurementKind::Angle
            || calibration.validate().is_err()
        {
            return false;
        }
        self.calibration = calibration
            .points
            .iter()
            .map(|(x, y)| format!("{x},{y}"))
            .collect::<Vec<_>>()
            .join("\n");
        self.known_distance = calibration.known_distance;
        self.measure_unit = calibration.unit;
        self.measurement_recalibration = Some(calibration);
        self.clear_preview();
        true
    }

    fn load_measurement(&mut self, measurement: &Measurement) {
        self.mode = RichMode::Measurement;
        self.interactive_measurement = true;
        self.measure_kind = measurement.kind;
        self.measure_unit = measurement.unit;
        self.measure_holes = measurement.holes.clone();
        self.measure_depth = measurement.depth;
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

    pub(crate) fn open_dimensioned_rectangle_preview(
        &mut self,
        page: usize,
        rectangle: crate::measurement_shapes_ui::RectanglePlacement,
    ) -> bool {
        if self.annotate.adv.rich.is_some()
            || !rectangle.valid()
            || !self
                .document
                .as_ref()
                .is_some_and(|document| document.page == page)
        {
            return false;
        }
        self.open_rich_draft(false);
        let Some(draft) = self.annotate.adv.rich.as_mut() else {
            return false;
        };
        draft.page = page;
        draft.mode = RichMode::Measurement;
        draft.interactive_measurement = true;
        draft.measure_kind = MeasurementKind::Area;
        draft.rectangle = Some(rectangle);
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
            || !crate::measurement_geometry::geometry_count_valid(
                replacement.kind,
                replacement.points.len(),
            )
            || replacement.calibration != original.measurement.calibration
            || replacement.unit != original.measurement.unit
            || replacement.known_distance != original.measurement.known_distance
            || replacement.precision != original.measurement.precision
            || replacement.depth != original.measurement.depth
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
            measure_holes: Vec::new(),
            measure_depth: None,
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
            measurement_recalibration: None,
            measurement_geometry_edit: false,
            rectangle: None,
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
                && self.document.as_ref().is_some_and(|document| {
                    draft.page < document.info.page_count()
                        && (draft.rectangle.is_none() || document.page == draft.page)
                })
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
        let request = if let Some(rectangle) = &draft.rectangle {
            if !rectangle.valid() {
                return;
            }
            Request::PreviewDimensionedRectangle {
                page: draft.page,
                rectangle: rectangle.clone(),
                author: draft.author.clone(),
                epoch: draft.epoch,
                generation: self.annotate.adv.rich_generation.wrapping_add(1),
            }
        } else if let Some(record) = &draft.measurement_edit {
            let Some(measurement) = draft.measurement() else {
                return;
            };
            if let Some(calibration) = &draft.measurement_recalibration {
                Request::PreviewMeasurementRecalibration {
                    page: draft.page,
                    record: record.clone(),
                    measurement,
                    calibration: calibration.clone(),
                    epoch: draft.epoch,
                    generation: self.annotate.adv.rich_generation.wrapping_add(1),
                }
            } else {
                Request::PreviewMeasurementEdit {
                    page: draft.page,
                    record: record.clone(),
                    measurement,
                    epoch: draft.epoch,
                    generation: self.annotate.adv.rich_generation.wrapping_add(1),
                }
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
        let lang = self.lang();
        let Some(draft) =
            self.annotate.adv.rich.as_mut().filter(|draft| {
                draft.epoch == epoch && draft.generation == generation && draft.busy
            })
        else {
            return;
        };
        draft.busy = false;
        match result.and_then(|preview| {
            RichPreview::new(ctx, preview.image).map(|image| (preview.index, image))
        }) {
            Ok((index, preview)) => {
                draft.preview_index = index;
                draft.prepared = true;
                draft.preview = Some(preview);
            }
            Err(error) => {
                let message = error.to_string();
                draft.error = Some(if draft.rectangle.is_some() {
                    match &error {
                        EngineError::Other(reason) => {
                            if reason == crate::worker::RECTANGLE_REFERENCE_REQUIRED {
                                tr(lang, "measure.rectangle.confirm_reference").into()
                            } else if let Some(cause) =
                                reason.strip_prefix(crate::worker::RECTANGLE_CAPTURE_FAILED)
                            {
                                sunipdf_i18n::tr_args(
                                    lang,
                                    "measure.rectangle.capture_failed",
                                    &[("reason", cause)],
                                )
                            } else {
                                message
                            }
                        }
                        _ => message,
                    }
                } else {
                    message
                });
            }
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
        let current_calibration = self
            .annotate
            .adv
            .rich
            .as_ref()
            .and_then(|draft| self.annotate.measure.local_calibration(draft.page));
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
            .width(680.0_f32.min((ctx.content_rect().width() - 32.0).max(300.0))),
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
                            if let Some(rectangle) = &mut draft.rectangle {
                                changed |= rectangle.summary(ui, lang);
                            } else if draft.interactive_measurement {
                                ui.label(tr(lang, measure_key(draft.measure_kind)));
                                if let Some(record) = &draft.measurement_edit {
                                    ui.label(tr(
                                        lang,
                                        crate::measurement_saved::validity_key(record.validity),
                                    ));
                                    if !record.validity.is_current() {
                                        ui.colored_label(
                                            crate::theme::palette_for(ctx).error,
                                            tr(lang, "measure.validity.historical"),
                                        );
                                        if let Ok(result) = record.measurement.result() {
                                            ui.label(result["label"].as_str().unwrap_or_default());
                                        }
                                        ui.small(tr(lang, "rich.points"));
                                        ui.monospace(&draft.points);
                                        if draft.measure_kind == MeasurementKind::Angle {
                                            ui.label(tr(lang, "measure.validity.recreate_angle"));
                                        } else {
                                            ui.label(tr(lang, "measure.validity.recalibrate_help"));
                                            if ui
                                                .add_enabled(
                                                    current_calibration.is_some(),
                                                    egui::Button::new(tr(
                                                        lang,
                                                        "measure.validity.recalibrate",
                                                    )),
                                                )
                                                .on_disabled_hover_text(tr(
                                                    lang,
                                                    "measure.validity.reference_required",
                                                ))
                                                .clicked()
                                            {
                                                if let Some(calibration) = current_calibration.clone() {
                                                    changed |= draft.use_current_measurement_reference(calibration);
                                                }
                                            }
                                            if draft.measurement_recalibration.is_some() {
                                                ui.small(tr(lang, "measure.validity.reference_selected"));
                                            } else if current_calibration.is_none() {
                                                ui.small(tr(lang, "measure.validity.reference_required"));
                                            }
                                        }
                                    }
                                }
                                if let Some(result) =
                                    draft.measurement().and_then(|m| m.result().ok())
                                {
                                    ui.heading(result["label"].as_str().unwrap_or_default());
                                } else if draft.measurement_edit.as_ref().is_none_or(|record| record.validity.is_current()) {
                                    ui.label(tr(lang, "measure.invalid"));
                                }
                                if matches!(draft.measure_kind, MeasurementKind::Area | MeasurementKind::Volume) {
                                    if let Some(measurement) = draft.measurement() {
                                        crate::measurement_area_ui::area_summary(ui, lang, &measurement);
                                    }
                                }
                                if let Some(depth) = &mut draft.measure_depth {
                                    ui.add_enabled_ui(draft.measurement_edit.as_ref().is_none_or(|record| record.validity.is_current()) && !draft.measurement_geometry_edit, |ui| {
                                        changed |= crate::measurement_area_ui::depth_controls(ui, lang, depth);
                                    });
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
                                                    .measurement()
                                                    .map_or(0, |measurement| {
                                                        measurement.points.len() + measurement.holes.iter().map(Vec::len).sum::<usize>()
                                                    })
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
                                        let scalar_editable = draft.measurement_recalibration.is_none()
                                            && draft.measurement_edit.as_ref().is_some_and(|record| record.validity.is_current());
                                        ui.add_enabled_ui(scalar_editable, |ui| ui.horizontal_wrapped(|ui| {
                                            ui.label(tr(lang, "measure.known_distance"));
                                            changed |= ui
                                                .add(
                                                    egui::DragValue::new(&mut draft.known_distance)
                                                .custom_parser(|text| sunipdf_core::measurement::parse_measurement_distance(text, draft.measure_unit).ok())
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
                                        }));
                                        ui.small(tr(lang, "measure.edit.reference"));
                                        ui.monospace(&draft.calibration);
                                    }
                                    let precision_editable = draft.measurement_recalibration.is_some()
                                        || draft.measurement_edit.as_ref().is_some_and(|record| record.validity.is_current());
                                    ui.add_enabled_ui(precision_editable, |ui| ui.horizontal(|ui| {
                                        ui.label(tr(lang, "measure.precision"));
                                        changed |= ui
                                            .add(
                                                egui::DragValue::new(&mut draft.precision)
                                                    .range(0..=4),
                                            )
                                            .changed();
                                    }));
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
                                if old != draft.measure_kind {
                                    draft.measure_depth = (draft.measure_kind == MeasurementKind::Volume).then_some(MeasurementDepth { value: 1.0, unit: draft.measure_unit });
                                    if !matches!(draft.measure_kind, MeasurementKind::Area | MeasurementKind::Volume) { draft.measure_holes.clear(); }
                                }
                                if let Some(depth) = &mut draft.measure_depth {
                                    changed |= crate::measurement_area_ui::depth_controls(ui, lang, depth);
                                }
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
                                                .custom_parser(|text| sunipdf_core::measurement::parse_measurement_distance(text, draft.measure_unit).ok())
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
                    ui.colored_label(crate::theme::palette_for(ctx).error, error);
                }
                if let Some(preview) = &mut draft.preview {
                    preview.show(ui);
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
        MeasurementKind::Volume => "measure.volume",
        MeasurementKind::Angle => "measure.angle",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtered_preview_preserves_thin_closed_contours_at_display_density() {
        for (offset_x, offset_y) in [(0, 0), (1, 2), (2, 1), (3, 3)] {
            let mut raster = image::RgbaImage::from_pixel(880, 1032, image::Rgba([255; 4]));
            let rings = [
                (20 + offset_x, 20 + offset_y, 860 + offset_x, 980 + offset_y),
                (
                    210 + offset_x,
                    256 + offset_y,
                    360 + offset_x,
                    555 + offset_y,
                ),
            ];
            for &(left, top, right, bottom) in &rings {
                for x in left..=right {
                    for y in [top, top + 1, bottom, bottom + 1] {
                        raster.put_pixel(x, y, image::Rgba([13, 64, 191, 255]));
                    }
                }
                for y in top..=bottom {
                    for x in [left, left + 1, right, right + 1] {
                        raster.put_pixel(x, y, image::Rgba([13, 64, 191, 255]));
                    }
                }
            }
            let source = sunipdf_core::RenderedPage {
                page_index: 0,
                width: raster.width(),
                height: raster.height(),
                rgba: raster.into_raw(),
            };
            let original = source.rgba.clone();
            for density in [1.0, 1.25, 2.0] {
                let (image, logical) = filtered_rich_preview(&source, density).unwrap();
                assert!((logical.x / logical.y - 880.0 / 1032.0).abs() < 0.00001);
                assert_eq!(logical.y, 240.0);
                assert_eq!(image.size[1], (240.0_f32 * density).round() as usize);
                let sx = image.size[0] as f32 / source.width as f32;
                let sy = image.size[1] as f32 / source.height as f32;
                let visible_near = |x: usize, y: usize| {
                    (y.saturating_sub(1)..=(y + 1).min(image.size[1] - 1)).any(|y| {
                        (x.saturating_sub(1)..=(x + 1).min(image.size[0] - 1)).any(|x| {
                            let pixel = image.pixels[y * image.size[0] + x];
                            i16::from(pixel.b()) - i16::from(pixel.r()) > 12
                        })
                    })
                };
                for &(left, top, right, bottom) in &rings {
                    let left = (left as f32 * sx).round() as usize;
                    let right = (right as f32 * sx).round() as usize;
                    let top = (top as f32 * sy).round() as usize;
                    let bottom = (bottom as f32 * sy).round() as usize;
                    for x in left + 2..right - 2 {
                        assert!(visible_near(x, top), "top contour disappeared at {density}");
                        assert!(
                            visible_near(x, bottom),
                            "bottom contour disappeared at {density}"
                        );
                    }
                    for y in top + 2..bottom - 2 {
                        assert!(
                            visible_near(left, y),
                            "left contour disappeared at {density}"
                        );
                        assert!(
                            visible_near(right, y),
                            "right contour disappeared at {density}"
                        );
                    }
                }
            }
            assert_eq!(
                source.rgba, original,
                "display filtering never changes the source raster"
            );
        }
    }

    #[test]
    fn preview_filter_bounds_inputs_and_keeps_small_images_at_original_size() {
        let mut source = preview().image;
        let (image, logical) = filtered_rich_preview(&source, 2.0).unwrap();
        assert_eq!(image.size, [4, 2]);
        assert_eq!(logical, egui::vec2(4.0, 2.0));
        for density in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(filtered_rich_preview(&source, density).is_err());
        }
        source.rgba.pop();
        assert!(filtered_rich_preview(&source, 1.0).is_err());
        source.width = u32::MAX;
        source.height = u32::MAX;
        assert!(filtered_rich_preview(&source, 1.0).is_err());
        source.width = 0;
        assert!(filtered_rich_preview(&source, 1.0).is_err());
    }

    #[test]
    fn rectangle_reference_guidance_is_localized_and_capture_failures_keep_the_reason() {
        for language in ["en-US", "fr-FR"] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            app.settings.language = language.into();
            let rectangle = crate::measurement_shapes_ui::RectanglePlacement {
                anchor: (50.0, 100.0),
                width: 2.0,
                height: 3.0,
                rotation_degrees: 0.0,
                unit: MeasurementUnit::Metres,
                precision: 2,
                scale: crate::measurement_shapes_ui::RectangleScaleSelection::Page,
            };
            assert!(app.open_dimensioned_rectangle_preview(0, rectangle));
            for reason in [
                crate::worker::RECTANGLE_REFERENCE_REQUIRED.to_string(),
                format!(
                    "{}local references are limited to documents up to 128 MiB",
                    crate::worker::RECTANGLE_CAPTURE_FAILED
                ),
            ] {
                let (epoch, generation) = match requests.recv().unwrap() {
                    Request::PreviewDimensionedRectangle {
                        epoch, generation, ..
                    } => (epoch, generation),
                    _ => panic!("rectangle preview request"),
                };
                app.on_rich_preview(
                    &egui::Context::default(),
                    epoch,
                    generation,
                    Err(EngineError::Other(reason.clone())),
                );
                let message = app
                    .annotate
                    .adv
                    .rich
                    .as_ref()
                    .unwrap()
                    .error
                    .as_ref()
                    .unwrap();
                if reason == crate::worker::RECTANGLE_REFERENCE_REQUIRED {
                    assert_eq!(
                        message,
                        tr(app.lang(), "measure.rectangle.confirm_reference")
                    );
                    app.request_rich_preview();
                } else {
                    assert_eq!(
                        message,
                        &sunipdf_i18n::tr_args(
                            app.lang(),
                            "measure.rectangle.capture_failed",
                            &[(
                                "reason",
                                "local references are limited to documents up to 128 MiB"
                            )]
                        )
                    );
                }
                assert!(!app.annotate.adv.rich.as_ref().unwrap().prepared);
            }
            assert!(requests.try_recv().is_err());
        }
    }

    #[test]
    fn dimensioned_rectangle_preview_does_not_enter_the_free_vertex_editor_or_accept_a_changed_page(
    ) {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let size = app.document.as_ref().unwrap().info.page_sizes[0];
        app.document.as_mut().unwrap().info.page_sizes.push(size);
        let rectangle = crate::measurement_shapes_ui::RectanglePlacement {
            anchor: (50.0, 100.0),
            width: 2.0,
            height: 3.0,
            rotation_degrees: 0.0,
            unit: MeasurementUnit::Metres,
            precision: 2,
            scale: crate::measurement_shapes_ui::RectangleScaleSelection::Page,
        };
        assert!(app.open_dimensioned_rectangle_preview(0, rectangle));
        let (epoch, generation) = match requests.recv().unwrap() {
            Request::PreviewDimensionedRectangle {
                rectangle,
                epoch,
                generation,
                ..
            } => {
                assert_eq!((rectangle.width, rectangle.height), (2.0, 3.0));
                (epoch, generation)
            }
            _ => panic!("dedicated rectangle preview"),
        };
        let draft = app.annotate.adv.rich.as_ref().unwrap();
        assert!(draft.valid_inputs());
        assert!(
            draft.operation().is_none(),
            "generic vertices cannot replace physical dimensions"
        );
        app.document.as_mut().unwrap().page = 1;
        app.on_rich_preview(&egui::Context::default(), epoch, generation, Ok(preview()));
        app.apply_rich_draft();
        assert!(requests
            .try_iter()
            .all(|request| !matches!(request, Request::Edit(_))));
        assert!(!app.annotate.adv.rich.as_ref().unwrap().prepared);
    }

    #[test]
    fn changing_rectangle_angle_in_the_bilingual_preview_requires_a_fresh_candidate() {
        fn text_position(shape: &egui::Shape) -> Option<egui::Pos2> {
            match shape {
                egui::Shape::Text(text)
                    if text.galley.text().trim_end_matches('\u{b0}').parse::<f64>() == Ok(0.0) =>
                {
                    Some(text.pos + text.galley.size() / 2.0)
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(text_position),
                _ => None,
            }
        }
        for (language, height) in ["en-US", "fr-FR"]
            .into_iter()
            .flat_map(|language| [480.0_f32, 680.0].map(move |height| (language, height)))
        {
            let (mut app, requests, _) = crate::app::tests::test_app();
            app.settings.language = language.into();
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            ctx.all_styles_mut(|style| style.animation_time = 0.0);
            assert!(app.open_dimensioned_rectangle_preview(
                0,
                crate::measurement_shapes_ui::RectanglePlacement {
                    anchor: (100.0, 100.0),
                    width: 2.0,
                    height: 3.0,
                    rotation_degrees: 0.0,
                    unit: MeasurementUnit::Metres,
                    precision: 2,
                    scale: crate::measurement_shapes_ui::RectangleScaleSelection::Page,
                }
            ));
            let (epoch, first) = match requests.recv().unwrap() {
                Request::PreviewDimensionedRectangle {
                    epoch, generation, ..
                } => (epoch, generation),
                _ => panic!("first rectangle preview"),
            };
            app.on_rich_preview(&ctx, epoch, first, Ok(preview()));
            assert!(app.annotate.adv.rich.as_ref().unwrap().prepared);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(560.0, height));
            let mut time = 0.0;
            let mut frame = |app: &mut SuniApp, events| {
                time += 0.05;
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        time: Some(time),
                        events,
                        ..Default::default()
                    },
                    |_| app.rich_annotation_dialog(&ctx),
                );
                output.textures_delta.clear();
                output
            };
            let mut position = None;
            let mut visible = std::collections::HashSet::new();
            for frame_index in 0..3 {
                let output = frame(&mut app, Vec::new());
                if frame_index < 2 {
                    continue;
                }
                position = output
                    .shapes
                    .iter()
                    .find_map(|shape| {
                        text_position(&shape.shape).filter(|point| shape.clip_rect.contains(*point))
                    })
                    .or(position);
                if let Some(tree) = output.platform_output.accesskit_update {
                    for (_, node) in tree.nodes {
                        for key in [
                            "measure.rectangle.rotation",
                            "bar.apply",
                            "rich.preview",
                            "dialog.password.cancel",
                        ] {
                            if node.label() == Some(tr(app.lang(), key)) {
                                if let Some(bounds) = node.bounds() {
                                    assert!(
                                        bounds.x0 >= 0.0
                                            && bounds.y0 >= 0.0
                                            && bounds.x1 <= 560.0
                                            && bounds.y1 <= f64::from(height),
                                        "{language}: {key} outside 560 x {height}: {bounds:?}"
                                    );
                                    visible.insert(key);
                                }
                            }
                        }
                    }
                }
            }
            assert_eq!(
                visible.len(),
                4,
                "angle and footer controls must remain visible: {language}"
            );
            let position = position.expect("visible angle editor");
            let button = |pos, pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            frame(
                &mut app,
                vec![egui::Event::PointerMoved(position), button(position, true)],
            );
            let moved = position + egui::vec2(24.0, 0.0);
            frame(&mut app, vec![egui::Event::PointerMoved(moved)]);
            frame(&mut app, vec![button(moved, false)]);
            let draft = app.annotate.adv.rich.as_ref().unwrap();
            let angle = draft.rectangle.as_ref().unwrap().rotation_degrees;
            assert!(angle > 0.0 && angle < 360.0);
            assert!(!draft.prepared && draft.preview.is_none());
            app.apply_rich_draft();
            assert!(
                requests.try_recv().is_err(),
                "angle changes cannot apply the old candidate"
            );
            app.request_rich_preview();
            let second = match requests.recv().unwrap() {
                Request::PreviewDimensionedRectangle {
                    rectangle,
                    generation,
                    ..
                } => {
                    assert_eq!(rectangle.rotation_degrees, angle);
                    generation
                }
                _ => panic!("new angle preview"),
            };
            assert_ne!(second, first);
            app.on_rich_preview(&ctx, epoch, first, Ok(preview()));
            assert!(!app.annotate.adv.rich.as_ref().unwrap().prepared);
            app.on_rich_preview(&ctx, epoch, second, Ok(preview()));
            assert!(app.annotate.adv.rich.as_ref().unwrap().prepared);
            frame(
                &mut app,
                vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert!(app.annotate.adv.rich.is_none());
            assert!(requests
                .try_iter()
                .all(|request| matches!(request, Request::DiscardRichAnnotationPreview { .. })));
        }
    }

    #[test]
    fn interactive_measurement_bridge_preserves_geometry_and_only_requests_a_preview() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let measurement = Measurement {
            holes: Vec::new(),
            depth: None,
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
    fn volume_preview_keeps_deductions_depth_and_rejects_the_previous_preview_after_an_edit() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let measurement = Measurement {
            kind: MeasurementKind::Volume,
            points: vec![(40.0, 40.0), (240.0, 40.0), (240.0, 240.0), (40.0, 240.0)],
            holes: vec![vec![
                (90.0, 90.0),
                (140.0, 90.0),
                (140.0, 140.0),
                (90.0, 140.0),
            ]],
            depth: Some(MeasurementDepth {
                value: 20.0,
                unit: MeasurementUnit::Centimetres,
            }),
            calibration: [(40.0, 20.0), (140.0, 20.0)],
            known_distance: 10.0,
            unit: MeasurementUnit::Metres,
            precision: 2,
        };
        assert!(app.open_measurement_preview(0, measurement.clone()));
        let (epoch, generation) = match requests.recv().unwrap() {
            Request::PreviewRichAnnotation {
                annotation: RichAnnotation::Measurement(sent),
                epoch,
                generation,
                ..
            } => {
                assert_eq!(sent, measurement);
                (epoch, generation)
            }
            _ => panic!("volume preview"),
        };
        let draft = app.annotate.adv.rich.as_mut().unwrap();
        draft.set_measure_unit(MeasurementUnit::Centimetres);
        assert_eq!(draft.measurement().unwrap().holes, measurement.holes);
        assert_eq!(draft.measurement().unwrap().depth, measurement.depth);
        draft.busy = false;
        app.request_rich_preview();
        let newer = match requests.recv().unwrap() {
            Request::PreviewRichAnnotation { generation, .. } => generation,
            _ => panic!("new preview"),
        };
        assert_ne!(newer, generation);
        app.on_rich_preview(&egui::Context::default(), epoch, generation, Ok(preview()));
        app.apply_rich_draft();
        assert!(!app.annotate.adv.rich.as_ref().unwrap().prepared);
        assert!(requests.try_recv().is_err());
        app.on_rich_preview(&egui::Context::default(), epoch, newer, Ok(preview()));
        assert!(app.annotate.adv.rich.as_ref().unwrap().prepared);
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
    fn historical_measurement_metadata_edits_cannot_reestablish_page_provenance() {
        use sunipdf_core::measurement::MeasurementValidity;
        for validity in [
            MeasurementValidity::LegacyUnverified,
            MeasurementValidity::Stale,
        ] {
            for kind in [MeasurementKind::Length, MeasurementKind::Angle] {
                let (mut app, requests, _) = crate::app::tests::test_app();
                app.open_rich_annotations(&egui::Context::default());
                let measurement = Measurement {
                    holes: Vec::new(),
                    depth: None,
                    kind,
                    points: if kind == MeasurementKind::Angle {
                        vec![(40.0, 70.0), (140.0, 70.0), (140.0, 170.0)]
                    } else {
                        vec![(40.0, 70.0), (140.0, 70.0)]
                    },
                    calibration: [(40.0, 40.0), (140.0, 40.0)],
                    known_distance: 10.0,
                    unit: MeasurementUnit::Metres,
                    precision: 2,
                };
                let draft = app.annotate.adv.rich.as_mut().unwrap();
                draft.load_measurement(&measurement);
                draft.measurement_edit = Some(MeasurementRecord {
                    index: 0,
                    annotation_id: Some("historical-measurement".into()),
                    measurement: measurement.clone(),
                    validity,
                });
                draft.precision = 3;
                draft.set_measure_unit(MeasurementUnit::Feet);
                draft.known_distance *= 2.0;
                assert!(!draft.valid_inputs());
                assert!(draft.measurement().is_none());
                let calibration = PageCalibration {
                    points: [(50.0, 50.0), (150.0, 50.0)],
                    known_distance: 20.0,
                    unit: MeasurementUnit::Metres,
                };
                assert_eq!(
                    draft.use_current_measurement_reference(calibration.clone()),
                    kind != MeasurementKind::Angle,
                );
                if kind == MeasurementKind::Length {
                    let replacement = draft.measurement().unwrap();
                    assert_eq!(replacement.points, measurement.points);
                    assert_eq!(replacement.calibration, calibration.points);
                    assert_eq!(replacement.known_distance, 20.0);
                    draft.known_distance = 21.0;
                    assert!(
                        draft.measurement().is_none(),
                        "the explicit reference stays authoritative"
                    );
                } else {
                    assert!(!draft.valid_inputs(), "historical angles must be recreated");
                }
                app.request_rich_preview();
                assert!(requests.try_recv().is_err());
            }
        }
    }

    #[test]
    fn saved_measurement_unit_changes_preserve_physical_size_and_references_recalculate() {
        for kind in [MeasurementKind::Length, MeasurementKind::Area] {
            let (mut app, _, _) = crate::app::tests::test_app();
            app.open_rich_annotations(&egui::Context::default());
            let measurement = Measurement {
                holes: Vec::new(),
                depth: None,
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
                validity: sunipdf_core::measurement::MeasurementValidity::Current,
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
    fn geometry_preview_changes_vertex_count_only_in_explicit_geometry_mode() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.open_rich_annotations(&egui::Context::default());
        let measurement = Measurement {
            holes: Vec::new(),
            depth: None,
            kind: MeasurementKind::Polyline,
            points: vec![(40.0, 70.0), (140.0, 70.0), (140.0, 170.0)],
            calibration: [(40.0, 40.0), (140.0, 40.0)],
            known_distance: 10.0,
            unit: MeasurementUnit::Feet,
            precision: 3,
        };
        let record = MeasurementRecord {
            validity: sunipdf_core::measurement::MeasurementValidity::Current,
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
        draft.points.push_str("\n200,200");
        assert_eq!(draft.measurement().unwrap().points.len(), 4);
        draft.points = "40,70\n140,170".into();
        assert_eq!(draft.measurement().unwrap().points.len(), 2);
        assert!(
            draft.operation().is_none(),
            "geometry editing cannot insert a new annotation"
        );
        for invalid in 0..5 {
            draft.load_measurement(&measurement);
            match invalid {
                0 => draft.points = "40,70".into(),
                1 => draft.known_distance = 20.0,
                2 => draft.precision = 2,
                3 => draft.measure_unit = MeasurementUnit::Metres,
                _ => draft.calibration = "40,40\n160,40".into(),
            }
            assert!(
                draft.measurement().is_none(),
                "geometry mode must preserve calibration and valid cardinality"
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

    fn document_changed(app: &SuniApp) -> crate::worker::Response {
        crate::worker::Response::DocumentChanged(crate::worker::DocState {
            info: app.document.as_ref().unwrap().info.clone(),
            can_undo: true,
            can_redo: false,
            pages_unchanged: true,
            measurement_scales_unchanged: true,
            modified: true,
            has_signatures: false,
        })
    }

    #[test]
    fn successful_rich_apply_closes_before_mutation_finished_and_ignores_late_preview() {
        use crate::worker::Response;

        for mode in [
            RichMode::FreeText,
            RichMode::Polygon,
            RichMode::Polyline,
            RichMode::Cloud,
            RichMode::Stamp,
            RichMode::Measurement,
        ] {
            let (mut app, requests, responses) = crate::app::tests::test_app();
            let ctx = egui::Context::default();
            if mode == RichMode::Measurement {
                assert!(app.open_dimensioned_rectangle_preview(
                    0,
                    crate::measurement_shapes_ui::RectanglePlacement {
                        anchor: (50.0, 100.0),
                        width: 2.0,
                        height: 3.0,
                        rotation_degrees: 0.0,
                        unit: MeasurementUnit::Metres,
                        precision: 2,
                        scale: crate::measurement_shapes_ui::RectangleScaleSelection::Page,
                    },
                ));
            } else {
                app.open_rich_annotations(&ctx);
                let draft = app.annotate.adv.rich.as_mut().unwrap();
                draft.mode = mode;
                draft.text = "Applied draft".into();
                app.request_rich_preview();
            }
            let (epoch, generation) = match requests.recv().unwrap() {
                Request::PreviewRichAnnotation {
                    epoch, generation, ..
                }
                | Request::PreviewDimensionedRectangle {
                    epoch, generation, ..
                } => (epoch, generation),
                _ => panic!("expected prepared annotation request"),
            };
            responses
                .send(Response::RichAnnotationPreview {
                    epoch,
                    generation,
                    result: Ok(preview()),
                })
                .unwrap();
            app.handle_responses(&ctx);
            app.apply_rich_draft();
            assert!(matches!(
                requests.recv().unwrap(),
                Request::Edit(EditOp::CommitRichAnnotationPreview {
                    epoch: sent_epoch,
                    generation: sent_generation,
                }) if sent_epoch == epoch && sent_generation == generation
            ));
            assert!(matches!(requests.recv().unwrap(), Request::FinishMutation));
            assert!(app.annotate.adv.rich.as_ref().unwrap().applying);
            assert!(app.worker.has_pending_mutations());

            // Deliver the success separately, as it can arrive a frame before
            // the mutation marker. The modal must already be gone in that frame.
            responses.send(document_changed(&app)).unwrap();
            app.handle_responses(&ctx);
            assert!(app.annotate.adv.rich.is_none());
            assert_ne!(app.view.epoch, epoch);
            assert!(app.worker.has_pending_mutations());
            assert!(app.dirty && app.can_undo && !app.can_redo);
            assert_eq!(app.annotate.adv.reselect, Some((0, 7)));
            assert!(!app.annotate.adv.dialog_open());
            app.rich_annotation_dialog(&ctx);

            responses.send(Response::MutationFinished).unwrap();
            responses
                .send(Response::RichAnnotationPreview {
                    epoch,
                    generation,
                    result: Ok(preview()),
                })
                .unwrap();
            app.handle_responses(&ctx);
            assert!(!app.worker.has_pending_mutations());
            assert!(app.annotate.adv.rich.is_none());
            assert!(!requests.try_iter().any(|request| matches!(
                request,
                Request::Edit(_) | Request::DiscardRichAnnotationPreview { .. }
            )));
        }
    }

    #[test]
    fn failed_rich_apply_retains_editable_draft_and_allows_a_fresh_preview() {
        use crate::worker::Response;

        let (mut app, requests, responses) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.open_rich_annotations(&ctx);
        app.annotate.adv.rich.as_mut().unwrap().text = "Keep this input".into();
        app.request_rich_preview();
        let (epoch, generation) = match requests.recv().unwrap() {
            Request::PreviewRichAnnotation {
                epoch, generation, ..
            } => (epoch, generation),
            _ => panic!("preview request"),
        };
        responses
            .send(Response::RichAnnotationPreview {
                epoch,
                generation,
                result: Ok(preview()),
            })
            .unwrap();
        app.handle_responses(&ctx);
        app.apply_rich_draft();
        assert!(matches!(requests.recv().unwrap(), Request::Edit(_)));
        assert!(matches!(requests.recv().unwrap(), Request::FinishMutation));
        responses
            .send(Response::OperationFailed(EngineError::Other(
                "commit refused".into(),
            )))
            .unwrap();
        app.handle_responses(&ctx);
        let draft = app.annotate.adv.rich.as_ref().unwrap();
        assert_eq!(draft.text, "Keep this input");
        assert!(!draft.applying && !draft.prepared && !draft.busy);
        assert!(draft.error.as_ref().unwrap().contains("commit refused"));
        assert!(!app.dirty && !app.can_undo);
        app.request_rich_preview();
        assert!(requests.try_recv().is_err(), "wait for the FIFO marker");
        responses.send(Response::MutationFinished).unwrap();
        app.handle_responses(&ctx);
        app.request_rich_preview();
        assert!(matches!(
            requests.recv().unwrap(),
            Request::PreviewRichAnnotation { generation: next, .. } if next != generation
        ));
    }

    #[test]
    fn unrelated_document_change_preserves_an_unapplied_rich_draft_and_cancel() {
        let (mut app, requests, responses) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.open_rich_annotations(&ctx);
        app.annotate.adv.rich.as_mut().unwrap().text = "Unapplied input".into();
        responses.send(document_changed(&app)).unwrap();
        app.handle_responses(&ctx);
        assert_eq!(
            app.annotate.adv.rich.as_ref().unwrap().text,
            "Unapplied input"
        );
        let _ = requests.try_iter().collect::<Vec<_>>();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 900.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input.clone(), |_| app.rich_annotation_dialog(&ctx));
        output.textures_delta.clear();
        let draft = app.annotate.adv.rich.as_ref().unwrap();
        assert!(!draft.applying && !draft.prepared && !draft.busy);
        assert_eq!(draft.error.as_deref(), Some(tr(app.lang(), "rich.stale")));
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
        assert!(requests
            .try_iter()
            .all(|request| !matches!(request, Request::Edit(_))));
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
