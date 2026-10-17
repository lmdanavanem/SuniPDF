//! Cadres de page : recadrer (avec aperçu), redimensionner, insérer des pages vierges.
//! Saisie en millimètres, envoi au moteur en points (espace d'affichage).

use crate::app::SuniApp;
use crate::dialog;
use crate::icons::glyph;
use crate::theme;
use crate::worker::{EditOp, Request};
use egui::{Key, RichText, Stroke};
use sunipdf_core::{PageLabelRange, PageLabelStyle, PageMargins, ResizeMode};
use sunipdf_i18n::{tr, tr_args, Language};

/// Formats usuels (mm, portrait). Noms non traduits (exception de `docs/I18N.md`).
pub const PRESETS: [(&str, f32, f32); 5] = [
    ("A3", 297.0, 420.0),
    ("A4", 210.0, 297.0),
    ("A5", 148.0, 210.0),
    ("Letter", 215.9, 279.4),
    ("Legal", 215.9, 355.6),
];

/// Côté minimal d'une page (points), comme le moteur.
const MIN_SIDE_PT: f32 = 18.0;

pub fn mm_to_pt(mm: f32) -> f32 {
    mm * 72.0 / 25.4
}

pub fn pt_to_mm(pt: f32) -> f32 {
    pt * 25.4 / 72.0
}

/// Taille (mm) restante après recadrage, `None` si la zone est trop petite.
pub fn crop_result_mm(page_mm: (f32, f32), f: &CropForm) -> Option<(f32, f32)> {
    if [page_mm.0, page_mm.1, f.left, f.right, f.top, f.bottom]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.0)
    {
        return None;
    }
    let w = page_mm.0 - f.left - f.right;
    let h = page_mm.1 - f.top - f.bottom;
    (mm_to_pt(w) >= MIN_SIDE_PT && mm_to_pt(h) >= MIN_SIDE_PT).then_some((w, h))
}

fn crop_from_drag(
    page_mm: (f32, f32),
    preview: egui::Rect,
    start: egui::Pos2,
    end: egui::Pos2,
) -> Option<CropForm> {
    if !preview.is_positive()
        || !preview.is_finite()
        || !start.is_finite()
        || !end.is_finite()
        || !preview.contains(start)
        || !page_mm.0.is_finite()
        || !page_mm.1.is_finite()
        || page_mm.0 <= 0.0
        || page_mm.1 <= 0.0
    {
        return None;
    }
    let keep = egui::Rect::from_two_pos(start, end.clamp(preview.min, preview.max));
    Some(CropForm {
        left: (keep.left() - preview.left()) / preview.width() * page_mm.0,
        right: (preview.right() - keep.right()) / preview.width() * page_mm.0,
        top: (keep.top() - preview.top()) / preview.height() * page_mm.1,
        bottom: (preview.bottom() - keep.bottom()) / preview.height() * page_mm.1,
        all: false,
    })
}

#[derive(Clone, Default)]
pub struct CropForm {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub all: bool,
}

#[derive(Clone)]
pub struct ResizeForm {
    /// Index dans `PRESETS`, ou `PRESETS.len()` = personnalisé.
    pub preset: usize,
    pub width: f32,
    pub height: f32,
    pub landscape: bool,
    pub scale: bool,
    pub annotations: bool,
    pub all: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BlankPos {
    Before,
    After,
    End,
}

#[derive(Clone)]
pub struct BlankForm {
    /// 0 = comme la page courante, 1..=5 = `PRESETS`, 6 = personnalisé.
    pub size: usize,
    pub width: f32,
    pub height: f32,
    pub landscape: bool,
    pub count: u32,
    pub position: BlankPos,
}

#[derive(Clone)]
pub struct LabelForm {
    pub first: usize,
    pub last: usize,
    pub style: PageLabelStyle,
    pub prefix: String,
    pub start: u32,
}

impl LabelForm {
    fn range(&self, page_count: usize) -> Option<PageLabelRange> {
        let range = PageLabelRange {
            first: self.first.checked_sub(1)?,
            last: self.last.checked_sub(1)?,
            style: self.style,
            prefix: self.prefix.clone(),
            start: if self.style == PageLabelStyle::PrefixOnly {
                1
            } else {
                self.start
            },
        };
        range.validate(page_count).ok()?;
        Some(range)
    }
}

fn label_style_key(style: PageLabelStyle) -> &'static str {
    match style {
        PageLabelStyle::Decimal => "pagesetup.labels.decimal",
        PageLabelStyle::RomanUpper => "pagesetup.labels.roman_upper",
        PageLabelStyle::RomanLower => "pagesetup.labels.roman_lower",
        PageLabelStyle::LettersUpper => "pagesetup.labels.letters_upper",
        PageLabelStyle::LettersLower => "pagesetup.labels.letters_lower",
        PageLabelStyle::PrefixOnly => "pagesetup.labels.prefix_only",
    }
}

pub enum PageDialog {
    Crop(CropForm),
    Resize(ResizeForm),
    Blank(BlankForm),
    Labels(LabelForm),
}

#[derive(Default)]
pub struct PageSetupState {
    pub dialog: Option<PageDialog>,
}

impl PageSetupState {
    pub fn close_labels(&mut self) {
        if matches!(self.dialog, Some(PageDialog::Labels(_))) {
            self.dialog = None;
        }
    }
}

impl ResizeForm {
    pub fn new() -> Self {
        Self {
            preset: 1,
            width: PRESETS[1].1,
            height: PRESETS[1].2,
            landscape: false,
            scale: false,
            annotations: false,
            all: true,
        }
    }

    /// Taille demandée (mm), orientation appliquée.
    pub fn size_mm(&self) -> (f32, f32) {
        let (w, h) = match PRESETS.get(self.preset) {
            Some(p) => (p.1, p.2),
            None => (self.width, self.height),
        };
        orient(w, h, self.landscape)
    }
}

fn orient(w: f32, h: f32, landscape: bool) -> (f32, f32) {
    if landscape == (w < h) {
        (h, w)
    } else {
        (w, h)
    }
}

fn preset_name(lang: Language, i: usize) -> &'static str {
    PRESETS
        .get(i)
        .map_or_else(|| tr(lang, "pagesetup.resize.custom"), |p| p.0)
}

/// Aperçu du recadrage : la page (rendu courant si disponible) et la zone conservée.
fn crop_preview(
    ui: &mut egui::Ui,
    texture: Option<egui::TextureId>,
    page_mm: (f32, f32),
    f: &mut CropForm,
) {
    let max = egui::vec2(300.0, 330.0);
    let s = (max.x / page_mm.0).min(max.y / page_mm.1);
    let size = egui::vec2(page_mm.0 * s, page_mm.1 * s);
    ui.vertical_centered(|ui| {
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::drag());
        let response = response.on_hover_cursor(egui::CursorIcon::Crosshair);
        if response.dragged() || response.drag_stopped() {
            let origin = ui.input(|i| i.pointer.press_origin());
            if let (Some(start), Some(end)) = (origin, response.interact_pointer_pos()) {
                if let Some(crop) = crop_from_drag(page_mm, rect, start, end) {
                    *f = CropForm { all: f.all, ..crop };
                }
            }
        }
        let painter = ui.painter();
        painter.rect_filled(
            rect.translate(egui::vec2(2.0, 2.0)),
            2.0,
            egui::Color32::from_black_alpha(40),
        );
        painter.rect_filled(rect, 0.0, egui::Color32::WHITE);
        if let Some(id) = texture {
            painter.image(
                id,
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        let keep = egui::Rect::from_min_max(
            rect.min + egui::vec2(f.left * s, f.top * s),
            rect.max - egui::vec2(f.right * s, f.bottom * s),
        );
        let shade = egui::Color32::from_black_alpha(110);
        for r in [
            egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, keep.min.y.max(rect.min.y))),
            egui::Rect::from_min_max(egui::pos2(rect.min.x, keep.max.y.min(rect.max.y)), rect.max),
            egui::Rect::from_min_max(
                egui::pos2(rect.min.x, keep.min.y),
                egui::pos2(keep.min.x, keep.max.y),
            ),
            egui::Rect::from_min_max(
                egui::pos2(keep.max.x, keep.min.y),
                egui::pos2(rect.max.x, keep.max.y),
            ),
        ] {
            if r.is_positive() {
                painter.rect_filled(r, 0.0, shade);
            }
        }
        if keep.is_positive() {
            painter.rect_stroke(
                keep,
                0.0,
                Stroke::new(1.5, theme::BLUE),
                egui::StrokeKind::Middle,
            );
        }
    });
}

impl SuniApp {
    pub fn open_page_dialog(&mut self, which: sunipdf_core::Command) {
        use sunipdf_core::Command as C;
        self.pagesetup.dialog = Some(match which {
            C::CropPages => PageDialog::Crop(CropForm::default()),
            C::ResizePages => PageDialog::Resize(ResizeForm::new()),
            C::PageLabels => {
                let Some(doc) = &self.document else { return };
                PageDialog::Labels(LabelForm {
                    first: doc.page + 1,
                    last: doc.info.page_count(),
                    style: PageLabelStyle::Decimal,
                    prefix: String::new(),
                    start: 1,
                })
            }
            _ => PageDialog::Blank(BlankForm {
                size: 0,
                width: PRESETS[1].1,
                height: PRESETS[1].2,
                landscape: false,
                count: 1,
                position: BlankPos::After,
            }),
        });
    }

    /// Pages visées : toutes, ou la page courante (la sélection dans la grille Organiser).
    fn setup_pages(&self, all: bool) -> Vec<usize> {
        match (&self.document, all) {
            (Some(d), true) => (0..d.info.page_count()).collect(),
            _ => self.target_pages(),
        }
    }

    pub fn page_setup_dialog(&mut self, ctx: &egui::Context) {
        let Some(dialog) = self.pagesetup.dialog.take() else {
            return;
        };
        let Some(doc) = &self.document else { return };
        let size_pt = doc
            .info
            .page_sizes
            .get(doc.page)
            .map_or((612.0, 792.0), |s| (s.width_pt, s.height_pt));
        let page_count = doc.info.page_count();
        let current = doc.page;
        // Aperçu : texture de la page entière dans la vue, sinon sa miniature.
        let texture = self
            .view
            .pool
            .whole(current)
            .or_else(|| self.thumbnails.get(&current))
            .map(|t| t.id());
        let lang = self.lang();
        let (dark, icons) = (self.settings.dark_theme, self.has_icons);
        let muted = theme::palette(dark).text_muted;
        let page_mm = (pt_to_mm(size_pt.0), pt_to_mm(size_pt.1));
        let scope = |ui: &mut egui::Ui, all: &mut bool| {
            dialog::field_label(ui, tr(lang, "pagesetup.scope"));
            ui.horizontal(|ui| {
                ui.radio_value(all, false, tr(lang, "pagesetup.scope.current"));
                ui.radio_value(all, true, tr(lang, "pagesetup.scope.all"));
            });
        };
        let mm = |ui: &mut egui::Ui, v: &mut f32, max: f32| {
            ui.add(
                egui::DragValue::new(v)
                    .range(0.0..=max)
                    .speed(0.5)
                    .max_decimals(1)
                    .suffix(" mm"),
            )
        };
        let (mut submit, mut cancel, mut reset) = (false, false, false);
        let enter_esc =
            |ui: &egui::Ui| ui.input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));
        let mut dialog = dialog;
        match &mut dialog {
            PageDialog::Labels(f) => {
                let spec = dialog::Spec::new(
                    "page-labels",
                    tr(lang, "pagesetup.labels.title"),
                    dark,
                    icons,
                )
                .subtitle(tr(lang, "pagesetup.labels.subtitle"))
                .icon(glyph::PAGES, "#")
                .width(520.0);
                let mut labels = f.clone();
                let valid = std::cell::Cell::new(labels.range(page_count).is_some());
                dialog::show(
                    ctx,
                    spec,
                    |ui| {
                        ui.horizontal(|ui| {
                            ui.label(tr(lang, "pagesetup.labels.first"));
                            ui.add(egui::DragValue::new(&mut labels.first).range(1..=page_count));
                            ui.label(tr(lang, "pagesetup.labels.last"));
                            ui.add(egui::DragValue::new(&mut labels.last).range(1..=page_count));
                        });
                        dialog::field_label(ui, tr(lang, "pagesetup.labels.style"));
                        egui::ComboBox::from_id_salt("page-label-style")
                            .selected_text(tr(lang, label_style_key(labels.style)))
                            .show_ui(ui, |ui| {
                                for style in [
                                    PageLabelStyle::Decimal,
                                    PageLabelStyle::RomanUpper,
                                    PageLabelStyle::RomanLower,
                                    PageLabelStyle::LettersUpper,
                                    PageLabelStyle::LettersLower,
                                    PageLabelStyle::PrefixOnly,
                                ] {
                                    ui.selectable_value(
                                        &mut labels.style,
                                        style,
                                        tr(lang, label_style_key(style)),
                                    );
                                }
                            });
                        dialog::field_label(ui, tr(lang, "pagesetup.labels.prefix"));
                        ui.add(
                            egui::TextEdit::singleline(&mut labels.prefix)
                                .desired_width(f32::INFINITY),
                        );
                        ui.horizontal(|ui| {
                            ui.label(tr(lang, "pagesetup.labels.start"));
                            ui.add_enabled(
                                labels.style != PageLabelStyle::PrefixOnly,
                                egui::DragValue::new(&mut labels.start).range(1..=u32::MAX),
                            );
                        });
                        ui.label(
                            RichText::new(tr(lang, "pagesetup.labels.positions")).color(muted),
                        );
                        valid.set(labels.range(page_count).is_some());
                        if !valid.get() {
                            ui.colored_label(
                                theme::palette(dark).error,
                                tr(lang, "pagesetup.labels.invalid"),
                            );
                        }
                    },
                    |ui| {
                        let (enter, esc) = enter_esc(ui);
                        ui.add_enabled_ui(valid.get(), |ui| {
                            if dialog::primary(ui, tr(lang, "pagesetup.labels.apply")) || enter {
                                submit = true;
                            }
                        });
                        if dialog::secondary(ui, tr(lang, "dialog.password.cancel")) || esc {
                            cancel = true;
                        }
                        if dialog::secondary(ui, tr(lang, "pagesetup.labels.reset")) {
                            reset = true;
                        }
                    },
                );
                *f = labels;
            }
            PageDialog::Crop(f) => {
                let spec = dialog::Spec::new("crop", tr(lang, "pagesetup.crop.title"), dark, icons)
                    .subtitle(tr(lang, "pagesetup.crop.subtitle"))
                    .icon(glyph::CROP, "⌗")
                    .width(560.0);
                let valid = std::cell::Cell::new(crop_result_mm(page_mm, f).is_some());
                let mut crop = f.clone();
                dialog::show(
                    ctx,
                    spec,
                    |ui| {
                        ui.horizontal_top(|ui| {
                            ui.vertical(|ui| {
                                for (key, v, max) in [
                                    ("pagesetup.crop.left", &mut crop.left, page_mm.0),
                                    ("pagesetup.crop.right", &mut crop.right, page_mm.0),
                                    ("pagesetup.crop.top", &mut crop.top, page_mm.1),
                                    ("pagesetup.crop.bottom", &mut crop.bottom, page_mm.1),
                                ] {
                                    dialog::field_label(ui, tr(lang, key));
                                    mm(ui, v, max);
                                }
                            });
                            ui.add_space(12.0);
                            ui.vertical(|ui| {
                                ui.set_max_width(300.0);
                                ui.label(tr(lang, "pagesetup.crop.drag"));
                                crop_preview(ui, texture, page_mm, &mut crop);
                            });
                        });
                        let result = crop_result_mm(page_mm, &crop);
                        valid.set(result.is_some());
                        match result {
                            Some((w, h)) => ui.label(
                                RichText::new(tr_args(
                                    lang,
                                    "pagesetup.crop.result",
                                    &[
                                        ("width", &format!("{w:.0}")),
                                        ("height", &format!("{h:.0}")),
                                    ],
                                ))
                                .color(muted),
                            ),
                            None => {
                                let error = theme::palette(ui.visuals().dark_mode).error;
                                ui.colored_label(error, tr(lang, "pagesetup.crop.too_small"))
                            }
                        };
                        scope(ui, &mut crop.all);
                    },
                    |ui| {
                        let (enter, esc) = enter_esc(ui);
                        if valid.get()
                            && (dialog::primary(ui, tr(lang, "pagesetup.crop.ok")) || enter)
                        {
                            submit = true;
                        }
                        if dialog::secondary(ui, tr(lang, "dialog.password.cancel")) || esc {
                            cancel = true;
                        }
                        if dialog::secondary(ui, tr(lang, "pagesetup.crop.reset")) {
                            reset = true;
                        }
                    },
                );
                *f = crop;
            }
            PageDialog::Resize(f) => {
                let spec = dialog::Spec::new(
                    "resize-pages",
                    tr(lang, "pagesetup.resize.title"),
                    dark,
                    icons,
                )
                .subtitle(tr(lang, "pagesetup.resize.subtitle"))
                .icon(glyph::FIT_PAGE, "▭")
                .width(460.0);
                dialog::show(
                    ctx,
                    spec,
                    |ui| {
                        dialog::field_label(ui, tr(lang, "pagesetup.resize.preset"));
                        egui::ComboBox::from_id_salt("resize-preset")
                            .selected_text(preset_name(lang, f.preset))
                            .width(200.0)
                            .show_ui(ui, |ui| {
                                for i in 0..=PRESETS.len() {
                                    ui.selectable_value(&mut f.preset, i, preset_name(lang, i));
                                }
                            });
                        let custom = f.preset >= PRESETS.len();
                        ui.horizontal(|ui| {
                            ui.add_enabled_ui(custom, |ui| {
                                ui.label(tr(lang, "pagesetup.resize.width"));
                                mm(ui, &mut f.width, 5080.0);
                                ui.label(tr(lang, "pagesetup.resize.height"));
                                mm(ui, &mut f.height, 5080.0);
                            });
                        });
                        ui.checkbox(&mut f.landscape, tr(lang, "pagesetup.resize.landscape"));
                        let (w, h) = f.size_mm();
                        ui.label(RichText::new(format!("{w:.0} × {h:.0} mm")).color(muted));
                        dialog::field_label(ui, tr(lang, "pagesetup.resize.mode"));
                        ui.radio_value(&mut f.scale, false, tr(lang, "pagesetup.resize.mode_keep"));
                        ui.radio_value(&mut f.scale, true, tr(lang, "pagesetup.resize.mode_scale"));
                        if f.scale {
                            ui.checkbox(
                                &mut f.annotations,
                                tr(lang, "pagesetup.resize.annotations"),
                            );
                            ui.label(
                                RichText::new(tr(lang, "pagesetup.resize.scale_note"))
                                    .size(12.0)
                                    .color(muted),
                            );
                        }
                        scope(ui, &mut f.all);
                    },
                    |ui| {
                        let (enter, esc) = enter_esc(ui);
                        if dialog::primary(ui, tr(lang, "pagesetup.resize.ok")) || enter {
                            submit = true;
                        }
                        if dialog::secondary(ui, tr(lang, "dialog.password.cancel")) || esc {
                            cancel = true;
                        }
                    },
                );
            }
            PageDialog::Blank(f) => {
                let spec =
                    dialog::Spec::new("blank-page", tr(lang, "pagesetup.blank.title"), dark, icons)
                        .subtitle(tr(lang, "pagesetup.blank.subtitle"))
                        .icon(glyph::ADD, "+")
                        .width(440.0);
                let size_name = |i: usize| match i {
                    0 => tr(lang, "pagesetup.blank.same_size"),
                    i => preset_name(lang, i - 1),
                };
                dialog::show(
                    ctx,
                    spec,
                    |ui| {
                        dialog::field_label(ui, tr(lang, "pagesetup.blank.size"));
                        egui::ComboBox::from_id_salt("blank-size")
                            .selected_text(size_name(f.size))
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                for i in 0..=PRESETS.len() + 1 {
                                    ui.selectable_value(&mut f.size, i, size_name(i));
                                }
                            });
                        if f.size > PRESETS.len() {
                            ui.horizontal(|ui| {
                                ui.label(tr(lang, "pagesetup.resize.width"));
                                mm(ui, &mut f.width, 5080.0);
                                ui.label(tr(lang, "pagesetup.resize.height"));
                                mm(ui, &mut f.height, 5080.0);
                            });
                        }
                        if f.size > 0 {
                            ui.checkbox(&mut f.landscape, tr(lang, "pagesetup.resize.landscape"));
                        }
                        ui.horizontal(|ui| {
                            ui.label(tr(lang, "pagesetup.blank.count"));
                            ui.add(egui::DragValue::new(&mut f.count).range(1..=500));
                        });
                        dialog::field_label(ui, tr(lang, "pagesetup.blank.position"));
                        ui.radio_value(
                            &mut f.position,
                            BlankPos::Before,
                            tr(lang, "pagesetup.blank.before"),
                        );
                        ui.radio_value(
                            &mut f.position,
                            BlankPos::After,
                            tr(lang, "pagesetup.blank.after"),
                        );
                        ui.radio_value(
                            &mut f.position,
                            BlankPos::End,
                            tr(lang, "pagesetup.blank.end"),
                        );
                    },
                    |ui| {
                        let (enter, esc) = enter_esc(ui);
                        if dialog::primary(ui, tr(lang, "pagesetup.blank.ok")) || enter {
                            submit = true;
                        }
                        if dialog::secondary(ui, tr(lang, "dialog.password.cancel")) || esc {
                            cancel = true;
                        }
                    },
                );
            }
        }
        if cancel {
            return;
        }
        if !(submit || reset) {
            self.pagesetup.dialog = Some(dialog);
            return;
        }
        let (op, status) = match dialog {
            PageDialog::Labels(f) => {
                if reset {
                    (
                        EditOp::ResetPageLabels,
                        Some(tr(lang, "status.labels_reset").to_owned()),
                    )
                } else {
                    let Some(range) = f.range(page_count) else {
                        self.pagesetup.dialog = Some(PageDialog::Labels(f));
                        return;
                    };
                    (
                        EditOp::SetPageLabels(range),
                        Some(tr(lang, "status.labels_applied").to_owned()),
                    )
                }
            }
            PageDialog::Crop(f) => {
                let pages = self.setup_pages(f.all);
                let n = pages.len();
                if reset {
                    (EditOp::ResetCrop { pages }, None)
                } else {
                    let margins = PageMargins {
                        left: mm_to_pt(f.left),
                        top: mm_to_pt(f.top),
                        right: mm_to_pt(f.right),
                        bottom: mm_to_pt(f.bottom),
                    };
                    (
                        EditOp::CropPages { pages, margins },
                        Some(tr_args(
                            lang,
                            "status.pages_cropped",
                            &[("count", &n.to_string())],
                        )),
                    )
                }
            }
            PageDialog::Resize(f) => {
                let pages = self.setup_pages(f.all);
                let n = pages.len();
                let (w, h) = f.size_mm();
                (
                    EditOp::ResizePages {
                        pages,
                        width_pt: mm_to_pt(w),
                        height_pt: mm_to_pt(h),
                        mode: if f.scale {
                            if f.annotations {
                                ResizeMode::ScaleContentWithAnnotations
                            } else {
                                ResizeMode::ScaleContent
                            }
                        } else {
                            ResizeMode::KeepContent
                        },
                    },
                    Some(tr_args(
                        lang,
                        "status.pages_resized",
                        &[("count", &n.to_string())],
                    )),
                )
            }
            PageDialog::Blank(f) => {
                let (w, h) = match f.size {
                    0 => size_pt,
                    i if i <= PRESETS.len() => {
                        let (w, h) = orient(PRESETS[i - 1].1, PRESETS[i - 1].2, f.landscape);
                        (mm_to_pt(w), mm_to_pt(h))
                    }
                    _ => {
                        let (w, h) = orient(f.width, f.height, f.landscape);
                        (mm_to_pt(w), mm_to_pt(h))
                    }
                };
                let at = match f.position {
                    BlankPos::Before => current,
                    BlankPos::After => current + 1,
                    BlankPos::End => page_count,
                };
                (
                    EditOp::InsertBlankPages {
                        at,
                        count: f.count as usize,
                        width_pt: w,
                        height_pt: h,
                    },
                    Some(tr_args(
                        lang,
                        "status.blank_inserted",
                        &[("count", &f.count.to_string())],
                    )),
                )
            }
        };
        self.edit.pending_status = status;
        self.worker.send(Request::Edit(op));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label_frame(
        app: &mut SuniApp,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 780.0),
                )),
                events,
                ..Default::default()
            },
            |_| app.page_setup_dialog(ctx),
        );
        output.textures_delta.clear();
        output
    }

    fn click_label_button(app: &mut SuniApp, ctx: &egui::Context, key: &str) {
        let mut target = None;
        for _ in 0..2 {
            let output = label_frame(app, ctx, vec![]);
            if let Some(tree) = output.platform_output.accesskit_update {
                if let Some(bounds) = tree
                    .nodes
                    .iter()
                    .map(|(_, node)| node)
                    .find(|node| node.label() == Some(app.t(key)))
                    .and_then(|node| node.bounds())
                {
                    assert!(bounds.x0 >= 0.0 && bounds.x1 <= 1280.0 && bounds.y1 <= 780.0);
                    target = Some(egui::pos2(
                        ((bounds.x0 + bounds.x1) / 2.0) as f32,
                        ((bounds.y0 + bounds.y1) / 2.0) as f32,
                    ));
                }
            }
        }
        let pos = target.expect("page labels button bounds");
        for pressed in [true, false] {
            label_frame(
                app,
                ctx,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    }

    #[test]
    fn page_label_buttons_apply_and_reset_once_in_both_languages() {
        for language in Language::ALL {
            let (mut app, requests, _) = crate::app::tests::test_app();
            let ctx = egui::Context::default();
            crate::icons::install(&ctx);
            ctx.enable_accesskit();
            app.settings.set_language(language);
            let doc = app.document.as_mut().unwrap();
            doc.info.page_sizes = vec![doc.info.page_sizes[0]; 5];
            doc.page = 1;
            app.execute(&ctx, sunipdf_core::Command::PageLabels);
            assert!(app.dialog_open());
            assert!(requests.try_recv().is_err());
            let Some(PageDialog::Labels(form)) = &mut app.pagesetup.dialog else {
                panic!("page label command must open its form");
            };
            assert_eq!((form.first, form.last, form.start), (2, 5, 1));
            form.last = 4;
            form.style = PageLabelStyle::RomanLower;
            form.prefix = "Intro-".into();
            form.start = 3;
            click_label_button(&mut app, &ctx, "pagesetup.labels.apply");
            assert!(
                matches!(requests.try_recv().unwrap(), Request::Edit(EditOp::SetPageLabels(range))
                if range.first == 1 && range.last == 3 && range.style == PageLabelStyle::RomanLower
                    && range.prefix == "Intro-" && range.start == 3)
            );
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::FinishMutation
            ));
            app.worker.finish_mutation();
            assert!(requests.try_recv().is_err() && app.pagesetup.dialog.is_none());
            assert!(
                !app.dirty && !app.can_undo,
                "only the worker may report a committed edit"
            );
            app.document.as_mut().unwrap().info.page_labels = vec![Some("i".into()); 5];
            app.execute(&ctx, sunipdf_core::Command::PageLabels);
            click_label_button(&mut app, &ctx, "pagesetup.labels.reset");
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::Edit(EditOp::ResetPageLabels)
            ));
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::FinishMutation
            ));
            assert!(requests.try_recv().is_err() && app.pagesetup.dialog.is_none());
            app.worker.finish_mutation();
            app.document.as_mut().unwrap().info.page_labels = vec![None; 5];
            app.execute(&ctx, sunipdf_core::Command::PageLabels);
            click_label_button(&mut app, &ctx, "pagesetup.labels.reset");
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::Edit(EditOp::ResetPageLabels)
            ));
            assert!(matches!(
                requests.try_recv().unwrap(),
                Request::FinishMutation
            ));
            assert!(requests.try_recv().is_err() && app.pagesetup.dialog.is_none());
        }
    }

    #[test]
    fn page_label_keyboard_rejects_invalid_ranges_and_escape_cancels_without_mutation() {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        crate::icons::install(&ctx);
        app.execute(&ctx, sunipdf_core::Command::PageLabels);
        let Some(PageDialog::Labels(form)) = &mut app.pagesetup.dialog else {
            panic!("label form")
        };
        form.prefix = "invalid\nlabel".into();
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        label_frame(&mut app, &ctx, vec![key(egui::Key::Enter)]);
        assert!(app.pagesetup.dialog.is_some() && requests.try_recv().is_err());
        let invalid = LabelForm {
            first: 2,
            last: 1,
            style: PageLabelStyle::Decimal,
            prefix: String::new(),
            start: 1,
        };
        assert!(invalid.range(2).is_none());
        assert!(LabelForm {
            first: 0,
            ..invalid.clone()
        }
        .range(2)
        .is_none());
        assert!(LabelForm {
            first: 1,
            last: 3,
            ..invalid
        }
        .range(2)
        .is_none());
        for start in [0, u32::MAX] {
            let prefix_only = LabelForm {
                first: 1,
                last: 2,
                style: PageLabelStyle::PrefixOnly,
                prefix: "Appendix".into(),
                start,
            };
            assert_eq!(prefix_only.range(2).unwrap().start, 1);
        }
        label_frame(&mut app, &ctx, vec![key(egui::Key::Escape)]);
        assert!(app.pagesetup.dialog.is_none() && requests.try_recv().is_err());
        assert!(!app.dirty && !app.can_undo);
    }

    #[test]
    fn units_presets_and_crop_validation() {
        assert!((mm_to_pt(25.4) - 72.0).abs() < 1e-4);
        assert!((pt_to_mm(mm_to_pt(210.0)) - 210.0).abs() < 1e-3);
        let mut r = ResizeForm::new();
        assert_eq!(r.size_mm(), (210.0, 297.0));
        r.landscape = true;
        assert_eq!(r.size_mm(), (297.0, 210.0));
        r.preset = PRESETS.len();
        r.width = 100.0;
        r.height = 50.0;
        assert_eq!(r.size_mm(), (100.0, 50.0), "personnalisé déjà en paysage");
        let f = CropForm {
            left: 10.0,
            right: 10.0,
            top: 20.0,
            bottom: 0.0,
            all: false,
        };
        assert_eq!(crop_result_mm((210.0, 297.0), &f), Some((190.0, 277.0)));
        let huge = CropForm {
            left: 100.0,
            right: 105.0,
            ..f
        };
        assert_eq!(
            crop_result_mm((210.0, 297.0), &huge),
            None,
            "moins de 18 pt"
        );
    }

    #[test]
    fn mouse_crop_maps_displayed_preview_and_clamps_without_losing_reverse_drags() {
        let preview = egui::Rect::from_min_size(egui::pos2(20.0, 30.0), egui::vec2(210.0, 297.0));
        let f = crop_from_drag(
            (210.0, 297.0),
            preview,
            egui::pos2(210.0, 300.0),
            egui::pos2(30.0, 50.0),
        )
        .unwrap();
        assert!((f.left - 10.0).abs() < 0.001);
        assert!((f.top - 20.0).abs() < 0.001);
        assert!((f.right - 20.0).abs() < 0.001);
        assert!((f.bottom - 27.0).abs() < 0.001);
        assert_eq!(crop_result_mm((210.0, 297.0), &f), Some((180.0, 250.0)));
        let f = crop_from_drag(
            (297.0, 210.0),
            preview,
            preview.min,
            preview.max + egui::vec2(20.0, 30.0),
        )
        .unwrap();
        assert_eq!(crop_result_mm((297.0, 210.0), &f), Some((297.0, 210.0)));
        let tiny = crop_from_drag(
            (210.0, 297.0),
            preview,
            preview.min,
            preview.min + egui::vec2(1.0, 1.0),
        )
        .unwrap();
        assert!(crop_result_mm((210.0, 297.0), &tiny).is_none());
        assert!(crop_from_drag(
            (210.0, 297.0),
            preview,
            egui::pos2(f32::NAN, 50.0),
            preview.max
        )
        .is_none());
        assert!(crop_from_drag(
            (210.0, 297.0),
            preview,
            preview.min - egui::vec2(1.0, 0.0),
            preview.max
        )
        .is_none());
        assert!(crop_result_mm(
            (210.0, 297.0),
            &CropForm {
                left: f32::NAN,
                ..CropForm::default()
            }
        )
        .is_none());
    }
}
