//! Vue « Organiser les pages » : grille de miniatures, sélection multiple, glisser-déposer,
//! compteur de sélection (les actions de page sont dans la barre de l'outil Organiser).

use crate::app::SuniApp;
use crate::theme;
use crate::worker::{EditOp, Request};
use egui::{Button, RichText, Sense, Stroke};
use std::collections::BTreeSet;
use sunipdf_i18n::{tr, tr_args};

#[derive(Default)]
pub struct OrganizeState {
    pub active: bool,
    pub selected: BTreeSet<usize>,
    pub anchor: Option<usize>,
}

/// Largeur logique d'une cellule de la grille.
const CELL_W: f32 = 150.0;

impl SuniApp {
    /// Pages ciblées par une action : sélection de la vue Organiser, sinon la page courante.
    pub fn target_pages(&self) -> Vec<usize> {
        if self.organize.active && !self.organize.selected.is_empty() {
            self.organize.selected.iter().copied().collect()
        } else {
            self.document
                .as_ref()
                .map(|d| vec![d.page])
                .unwrap_or_default()
        }
    }

    /// Barre interne de la grille : compteur de sélection et « Tout sélectionner ». Les actions de
    /// page sont dans la barre de l'outil Organiser (`workspace.rs`).
    pub fn organize_toolbar(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let count = self.document.as_ref().map_or(0, |d| d.info.page_count());
        let selected = self.organize.selected.len();
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            ui.label(
                RichText::new(tr_args(
                    lang,
                    "organize.selected",
                    &[
                        ("count", &selected.to_string()),
                        ("total", &count.to_string()),
                    ],
                ))
                .color(p.text_muted),
            );
            ui.separator();
            if ui
                .add(Button::new(tr(lang, "organize.select_all")).frame(false))
                .clicked()
            {
                self.organize.selected = (0..count).collect();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(6.0);
                // Aide tronquée (« … ») quand la place manque ; texte complet en info-bulle.
                ui.add(
                    egui::Label::new(
                        RichText::new(tr(lang, "organize.hint"))
                            .size(12.0)
                            .color(p.text_muted),
                    )
                    .truncate(),
                );
            });
        });
    }

    pub fn organize_grid(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let Some(doc) = &self.document else { return };
        let count = doc.info.page_count();
        let current = doc.page;
        let sizes = doc.info.page_sizes.clone();
        let px = ((CELL_W * ctx.pixels_per_point() / 16.0).round() * 16.0).max(16.0) as u32;
        if px != self.thumb_px {
            self.thumb_px = px;
            self.thumbnails.clear();
            self.thumbs_pending.clear();
        }
        let max_ratio = sizes
            .iter()
            .map(|s| s.height_pt / s.width_pt)
            .fold(0.0_f32, f32::max)
            .max(0.1);
        let cell_h = CELL_W * max_ratio + 40.0;
        let gap = 18.0;
        let columns = ((ui.available_width() - gap) / (CELL_W + gap))
            .floor()
            .max(1.0) as usize;
        let rows = count.div_ceil(columns);
        let mut clicked: Option<(usize, egui::Modifiers)> = None;
        let mut open_page = None;
        let mut dropped: Option<(usize, usize)> = None;
        let shift_held = ui.input(|i| i.modifiers.shift);
        let ctrl_held = ui.input(|i| i.modifiers.command);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, cell_h + gap, rows, |ui, row_range| {
                for row in row_range {
                    ui.horizontal(|ui| {
                        ui.add_space(gap);
                        for col in 0..columns {
                            let page = row * columns + col;
                            if page >= count {
                                break;
                            }
                            let ratio = sizes[page].height_pt / sizes[page].width_pt;
                            let img_size = egui::vec2(CELL_W, CELL_W * ratio);
                            if !self.thumbnails.contains_key(&page)
                                && self.thumbs_pending.insert(page)
                            {
                                self.worker.send(Request::Thumbnail { page, width_px: px });
                            }
                            let selected = self.organize.selected.contains(&page);
                            let (cell_rect, resp) = ui.allocate_exact_size(
                                egui::vec2(CELL_W + 16.0, cell_h),
                                Sense::click_and_drag(),
                            );
                            resp.dnd_set_drag_payload(page);
                            let fill = if selected {
                                p.accent.gamma_multiply(0.15)
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            let stroke = if selected {
                                Stroke::new(2.0, p.accent)
                            } else if page == current {
                                Stroke::new(1.0, p.accent.gamma_multiply(0.6))
                            } else {
                                Stroke::NONE
                            };
                            ui.painter().rect(
                                cell_rect,
                                8.0,
                                fill,
                                stroke,
                                egui::StrokeKind::Inside,
                            );
                            let img_rect = egui::Rect::from_min_size(
                                cell_rect.min
                                    + egui::vec2(
                                        8.0,
                                        8.0 + (cell_h - 40.0 - img_size.y).max(0.0) / 2.0,
                                    ),
                                img_size,
                            );
                            ui.painter().rect_filled(
                                img_rect.translate(egui::vec2(2.0, 2.0)),
                                2.0,
                                egui::Color32::from_black_alpha(50),
                            );
                            ui.painter()
                                .rect_filled(img_rect, 0.0, egui::Color32::WHITE);
                            if let Some(tex) = self.thumbnails.get(&page) {
                                ui.put(
                                    img_rect,
                                    egui::Image::new(egui::load::SizedTexture::new(
                                        tex.id(),
                                        img_size,
                                    ))
                                    .alt_text(tr_args(
                                        lang,
                                        "a11y.thumbnail",
                                        &[("page", &(page + 1).to_string())],
                                    )),
                                );
                            }
                            ui.painter().rect_stroke(
                                img_rect,
                                0.0,
                                Stroke::new(1.0, p.border),
                                egui::StrokeKind::Outside,
                            );
                            ui.painter().text(
                                egui::pos2(cell_rect.center().x, cell_rect.bottom() - 14.0),
                                egui::Align2::CENTER_CENTER,
                                (page + 1).to_string(),
                                egui::FontId::proportional(13.0),
                                p.text_muted,
                            );
                            // Cible de dépôt : barre d'insertion à gauche ou à droite selon la moitié survolée.
                            if let Some(payload) = resp.dnd_hover_payload::<usize>() {
                                if *payload != page {
                                    let left = ui
                                        .input(|i| i.pointer.hover_pos())
                                        .is_some_and(|pos| pos.x < cell_rect.center().x);
                                    let x = if left {
                                        cell_rect.left() - gap / 2.0
                                    } else {
                                        cell_rect.right() + gap / 2.0
                                    };
                                    ui.painter().vline(
                                        x,
                                        cell_rect.y_range(),
                                        Stroke::new(3.0, p.accent),
                                    );
                                }
                            }
                            if let Some(payload) = resp.dnd_release_payload::<usize>() {
                                let from = *payload;
                                if from != page {
                                    let left = ui
                                        .input(|i| i.pointer.latest_pos())
                                        .is_some_and(|pos| pos.x < cell_rect.center().x);
                                    let mut to = if left { page } else { page + 1 };
                                    if from < to {
                                        to -= 1;
                                    }
                                    // Déposée à sa propre place : rien à déplacer (ni « modifié »,
                                    // ni entrée d'annulation).
                                    if from != to {
                                        dropped = Some((from, to));
                                    }
                                }
                            }
                            if resp.double_clicked() {
                                open_page = Some(page);
                            } else if resp.clicked() {
                                clicked = Some((page, ui.input(|i| i.modifiers)));
                            }
                            ui.add_space(gap);
                        }
                    });
                    ui.add_space(gap);
                }
            });
        if let Some((page, modifiers)) = clicked {
            if modifiers.command || ctrl_held {
                if !self.organize.selected.remove(&page) {
                    self.organize.selected.insert(page);
                }
                self.organize.anchor = Some(page);
            } else if (modifiers.shift || shift_held) && self.organize.anchor.is_some() {
                let a = self.organize.anchor.unwrap_or(page);
                let (lo, hi) = if a <= page { (a, page) } else { (page, a) };
                self.organize.selected = (lo..=hi).collect();
            } else {
                self.organize.selected.clear();
                self.organize.selected.insert(page);
                self.organize.anchor = Some(page);
            }
            self.go_to_page(page);
        }
        if let Some((from, to)) = dropped {
            self.worker.send(Request::Edit(EditOp::Move { from, to }));
            self.organize.selected.clear();
            self.organize.selected.insert(to);
            self.organize.anchor = Some(to);
        }
        if let Some(page) = open_page {
            self.go_to_page(page);
            self.organize.active = false;
        }
    }
}
