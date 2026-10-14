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

/// Hauteur réservée à l'en-tête, au pied et aux marges : le corps défile au-delà, pour que les
/// boutons restent visibles dans une fenêtre basse (minimum 720 × 480).
const CHROME_H: f32 = 170.0;

/// Affiche une boîte modale. `body` dessine le contenu, `footer` les boutons (rangée alignée à
/// droite : le premier bouton ajouté est le plus à droite, donc l'action principale).
/// Échap : si une liste déroulante ou un sélecteur de couleur du corps était ouvert, la touche
/// ne ferme que lui (consommée ici, le pied ne la voit pas).
pub fn show<R>(
    ctx: &egui::Context,
    spec: Spec<'_>,
    body: impl FnOnce(&mut egui::Ui) -> R,
    footer: impl FnOnce(&mut egui::Ui),
) -> R {
    let p = theme::palette(spec.dark);
    let tint = spec.tint.unwrap_or(p.accent);
    // État de l'image précédente : la liste était ouverte quand Échap a été pressé.
    let popup_open = egui::Popup::is_any_open(ctx);
    let body_max_h = (ctx.content_rect().height() - CHROME_H).max(120.0);
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
                    egui::ScrollArea::vertical()
                        .max_height(body_max_h)
                        .show(ui, |ui| {
                            ui.set_width(spec.width - 44.0);
                            body(ui)
                        })
                        .inner
                })
                .inner;
            if popup_open {
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            }
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
            .fill(p.accent_fill)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn esc() -> egui::Event {
        egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// Une image de la boîte : (liste ouverte vue par le corps, Échap vu par le pied).
    fn frame(ctx: &egui::Context, events: Vec<egui::Event>, open_list: bool) -> (bool, bool) {
        let mut seen = (false, false);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            show(
                ui.ctx(),
                Spec::new("t", "Titre", false, false),
                |ui| {
                    // Même mécanique qu'une `ComboBox` ou un sélecteur de couleur (`Popup::menu`).
                    let menu = ui.menu_button("Liste", |ui| ui.label("a"));
                    seen.0 = menu.inner.is_some();
                    if open_list {
                        egui::Popup::open_id(
                            ui.ctx(),
                            egui::Popup::default_response_id(&menu.response),
                        );
                    }
                },
                |ui| {
                    seen.1 = ui.input(|i| i.key_pressed(egui::Key::Escape));
                },
            );
        });
        out.textures_delta.clear();
        seen
    }

    /// Scénario de la revue : Échap sur une liste ouverte ferme la liste, pas la boîte.
    #[test]
    fn escape_closes_an_open_list_without_reaching_the_footer() {
        let ctx = egui::Context::default();
        // Famille « semibold » de l'en-tête (polices système si présentes, sinon repli).
        let _ = icons::install(&ctx);
        let _ = frame(&ctx, vec![], true);
        assert_eq!(frame(&ctx, vec![], false), (true, false), "liste ouverte");
        // egui ferme la liste à la fin de l'image où Échap arrive ; le pied ne voit pas la touche.
        assert_eq!(
            frame(&ctx, vec![esc()], false),
            (true, false),
            "liste fermée seule"
        );
        assert_eq!(
            frame(&ctx, vec![esc()], false),
            (false, true),
            "le pied voit l'Échap suivant"
        );
    }

    #[test]
    fn primary_and_error_text_contrast_is_at_least_4_5_in_both_themes() {
        fn lum(c: Color32) -> f64 {
            let f = |v: u8| {
                let v = v as f64 / 255.0;
                if v <= 0.03928 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * f(c.r()) + 0.7152 * f(c.g()) + 0.0722 * f(c.b())
        }
        let contrast = |a: Color32, b: Color32| {
            let (x, y) = (lum(a) + 0.05, lum(b) + 0.05);
            x.max(y) / x.min(y)
        };
        for dark in [false, true] {
            let p = theme::palette(dark);
            assert!(
                contrast(Color32::WHITE, p.accent_fill) >= 4.5,
                "bouton (dark={dark})"
            );
            for surface in [p.surface, p.chrome] {
                assert!(contrast(p.error, surface) >= 4.5, "erreur (dark={dark})");
            }
        }
    }
}
