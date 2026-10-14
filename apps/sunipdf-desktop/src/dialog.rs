//! Cadre commun des boîtes de dialogue : en-tête (icône, titre, sous-titre), corps, pied à boutons.
//! Toutes les boîtes modales de l'application passent par `show` pour un rendu homogène.

use crate::icons::{self, glyph};
use crate::theme::{self, Palette};
use egui::{Align, Button, Color32, CornerRadius, Frame, Layout, Margin, RichText, Stroke};

pub struct Spec<'a> {
    pub id: &'a str,
    pub title: String,
    pub subtitle: Option<String>,
    /// Glyphe Segoe Fluent + repli texte.
    pub icon: (&'a str, &'a str),
    pub width: f32,
    pub dark: bool,
    pub has_icons: bool,
    /// Teinte de l'icône (accent par défaut, rouge pour les avertissements).
    pub tint: Option<Color32>,
}

impl<'a> Spec<'a> {
    pub fn new(id: &'a str, title: impl Into<String>, dark: bool, has_icons: bool) -> Self {
        Self {
            id,
            title: title.into(),
            subtitle: None,
            icon: (glyph::INFO, "i"),
            width: 420.0,
            dark,
            has_icons,
            tint: None,
        }
    }

    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = Some(s.into());
        self
    }

    pub fn icon(mut self, g: &'a str, fallback: &'a str) -> Self {
        self.icon = (g, fallback);
        self
    }

    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }

    pub fn warning(mut self) -> Self {
        self.tint = Some(theme::ERROR);
        self
    }
}

fn frame(p: &Palette, dark: bool) -> Frame {
    Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(14)
        .inner_margin(Margin::ZERO)
        .shadow(egui::epaint::Shadow {
            offset: [0, 10],
            blur: 32,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 160 } else { 60 }),
        })
}

/// Affiche une boîte modale. `body` dessine le contenu, `footer` les boutons (rangée alignée à
/// droite : le premier bouton ajouté est le plus à droite, donc l'action principale).
pub fn show<R>(
    ctx: &egui::Context,
    spec: Spec<'_>,
    body: impl FnOnce(&mut egui::Ui) -> R,
    footer: impl FnOnce(&mut egui::Ui),
) -> R {
    let p = theme::palette(spec.dark);
    let tint = spec.tint.unwrap_or(p.accent);
    egui::Modal::new(egui::Id::new(spec.id))
        .backdrop_color(Color32::from_black_alpha(if spec.dark { 150 } else { 110 }))
        .frame(frame(&p, spec.dark))
        .show(ctx, |ui| {
            ui.set_width(spec.width);
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
            // En-tête.
            Frame::new()
                .inner_margin(Margin {
                    left: 22,
                    right: 22,
                    top: 20,
                    bottom: 12,
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(40.0, 40.0), egui::Sense::hover());
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(10),
                            tint.gamma_multiply(if spec.dark { 0.35 } else { 0.14 }),
                        );
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            if spec.has_icons {
                                spec.icon.0
                            } else {
                                spec.icon.1
                            },
                            if spec.has_icons {
                                egui::FontId::new(18.0, egui::FontFamily::Name(icons::ICONS.into()))
                            } else {
                                egui::FontId::proportional(18.0)
                            },
                            tint,
                        );
                        ui.add_space(4.0);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(icons::semibold(spec.title.clone(), 18.0).color(p.text));
                            if let Some(s) = &spec.subtitle {
                                ui.label(RichText::new(s).size(13.0).color(p.text_muted));
                            }
                        });
                    });
                });
            // Corps.
            let inner = Frame::new()
                .inner_margin(Margin {
                    left: 22,
                    right: 22,
                    top: 4,
                    bottom: 18,
                })
                .show(ui, |ui| {
                    ui.set_width(spec.width - 44.0);
                    body(ui)
                })
                .inner;
            // Pied.
            Frame::new()
                .fill(p.chrome)
                .corner_radius(CornerRadius {
                    nw: 0,
                    ne: 0,
                    sw: 14,
                    se: 14,
                })
                .inner_margin(Margin {
                    left: 22,
                    right: 22,
                    top: 12,
                    bottom: 12,
                })
                .show(ui, |ui| {
                    ui.set_width(spec.width - 44.0);
                    let line = ui.max_rect();
                    ui.painter().hline(
                        line.x_range(),
                        line.top() - 12.0,
                        Stroke::new(1.0, p.border),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), footer);
                });
            inner
        })
        .inner
}

/// Bouton d'action principale (fond accent, texte blanc).
pub fn primary(ui: &mut egui::Ui, text: &str) -> bool {
    let p = theme::palette(ui.visuals().dark_mode);
    ui.add(
        Button::new(RichText::new(text).color(Color32::WHITE).size(14.0))
            .fill(p.accent)
            .stroke(Stroke::NONE)
            .corner_radius(8)
            .min_size(egui::vec2(96.0, 32.0)),
    )
    .clicked()
}

/// Bouton d'action destructrice (fond rouge).
pub fn danger(ui: &mut egui::Ui, text: &str) -> bool {
    ui.add(
        Button::new(RichText::new(text).color(Color32::WHITE).size(14.0))
            .fill(theme::ERROR)
            .stroke(Stroke::NONE)
            .corner_radius(8)
            .min_size(egui::vec2(96.0, 32.0)),
    )
    .clicked()
}

/// Bouton secondaire (bordure).
pub fn secondary(ui: &mut egui::Ui, text: &str) -> bool {
    let p = theme::palette(ui.visuals().dark_mode);
    ui.add(
        Button::new(RichText::new(text).size(14.0).color(p.text))
            .fill(p.surface)
            .stroke(Stroke::new(1.0, p.border))
            .corner_radius(8)
            .min_size(egui::vec2(96.0, 32.0)),
    )
    .clicked()
}

/// Libellé de champ (texte atténué, petit), au-dessus d'un contrôle.
pub fn field_label(ui: &mut egui::Ui, text: &str) {
    let p = theme::palette(ui.visuals().dark_mode);
    ui.label(RichText::new(text).size(12.5).color(p.text_muted));
}
