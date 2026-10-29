//! About SuniPDF: product identity, privacy, live update controls and local recovery.

use crate::icons::{self, glyph};
use crate::theme;
use egui::{Align, Color32, Frame, Layout, Margin, RichText, Stroke};
use sunipdf_i18n::{tr, Language};

pub fn show(
    ctx: &egui::Context,
    lang: Language,
    dark: bool,
    has_icons: bool,
    logo: Option<egui::Image<'static>>,
    updates: impl FnOnce(&mut egui::Ui),
) -> bool {
    let p = theme::palette_for(ctx);
    let width = (ctx.content_rect().width() - 40.0).clamp(280.0, 640.0);
    let body_height = (ctx.content_rect().height() - 148.0).max(120.0);
    let mut close = false;
    egui::Modal::new(egui::Id::unique("about"))
        .backdrop_color(Color32::from_black_alpha(if dark { 150 } else { 80 }))
        .frame(
            Frame::new()
                .fill(p.surface)
                .stroke(Stroke::new(1.0, p.border))
                .corner_radius(12)
                .shadow(egui::epaint::Shadow {
                    offset: [0, 10],
                    blur: 32,
                    spread: 0,
                    color: Color32::from_black_alpha(if dark { 160 } else { 50 }),
                }),
        )
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);
            Frame::new()
                .fill(p.chrome)
                .corner_radius(egui::CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 })
                .inner_margin(Margin::symmetric(18, 10))
                .show(ui, |ui| {
                    ui.set_width(width - 36.0);
                    ui.horizontal(|ui| {
                        Frame::new()
                            .fill(p.accent_soft)
                            .corner_radius(8)
                            .inner_margin(8)
                            .show(ui, |ui| {
                                ui.label(icons::icon(has_icons, glyph::DOCUMENT, "S", 18.0).color(p.accent));
                            });
                        ui.add_space(4.0);
                        ui.label(icons::semibold(tr(lang, "about.title"), 19.0).color(p.text));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let response = ui.add(
                                egui::Button::new(icons::icon(has_icons, glyph::CLOSE, "×", 17.0))
                                    .frame(false)
                                    .min_size(egui::vec2(30.0, 30.0)),
                            ).on_hover_text(tr(lang, "about.close"));
                            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tr(lang, "about.close")));
                            close |= response.clicked();
                        });
                    });
                });
            separator(ui);
            Frame::new().inner_margin(Margin::symmetric(26, 0)).show(ui, |ui| {
                ui.set_width(width - 52.0);
                ui.spacing_mut().scroll.floating_allocated_width = 10.0;
                egui::ScrollArea::vertical()
                    .id_salt("about-body")
                    .max_height(body_height)
                    .min_scrolled_height(body_height)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().item_spacing.y = 6.0;
                        ui.add_space(14.0);
                        ui.vertical_centered(|ui| {
                            if let Some(logo) = logo {
                                ui.add(logo);
                            } else {
                                ui.label(icons::semibold("SuniPDF", 30.0).color(p.text));
                            }
                            let version = sunipdf_i18n::tr_args(lang, "about.version", &[("version", env!("CARGO_PKG_VERSION"))]);
                            let version_width = ui.painter().layout_no_wrap(version.clone(), egui::FontId::proportional(14.0), p.text).size().x;
                            ui.horizontal(|ui| {
                                ui.add_space(((ui.available_width() - version_width - 28.0) / 2.0).max(0.0));
                                Frame::new().fill(p.accent_soft).corner_radius(9)
                                .inner_margin(Margin::symmetric(14, 4)).show(ui, |ui| {
                                    ui.label(RichText::new(version).size(14.0).color(p.text));
                                });
                            });
                            ui.add_space(6.0);
                            ui.label(icons::semibold(tr(lang, "about.tagline"), 22.0).color(p.text));
                            ui.label(RichText::new(tr(lang, "about.subtitle")).size(15.0).color(p.text));
                            ui.add_space(6.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(10.0, 8.0);
                                let total_width = ["about.offline", "about.no_telemetry", "about.no_account"]
                                    .iter().map(|key| ui.painter().layout_no_wrap(
                                        tr(lang, key).to_owned(), egui::FontId::proportional(14.0), p.text,
                                    ).size().x + 53.0).sum::<f32>() + 20.0;
                                if ui.available_width() > total_width {
                                    ui.add_space((ui.available_width() - total_width) / 2.0);
                                }
                                badge(ui, lang, Feature::Offline, "about.offline");
                                badge(ui, lang, Feature::Privacy, "about.no_telemetry");
                                badge(ui, lang, Feature::Account, "about.no_account");
                            });
                        });
                        ui.add_space(6.0);
                        Frame::new().fill(p.accent_soft)
                            .stroke(Stroke::new(1.0, p.accent.gamma_multiply(0.2)))
                            .corner_radius(10).inner_margin(16).show(ui, |ui| {
                                ui.horizontal_top(|ui| {
                                    let (rect, _) = ui.allocate_exact_size(egui::vec2(56.0, 64.0), egui::Sense::hover());
                                    let center = rect.center();
                                    ui.painter().circle_filled(center, 27.0, p.accent.gamma_multiply(0.1));
                                    ui.painter().add(egui::Shape::convex_polygon(
                                        [(0.0,-19.0),(15.0,-13.0),(14.0,5.0),(8.0,15.0),(0.0,20.0),(-8.0,15.0),(-14.0,5.0),(-15.0,-13.0)]
                                            .map(|(x,y)| center + egui::vec2(x,y)).to_vec(),
                                        p.accent_fill, Stroke::NONE,
                                    ));
                                    ui.painter().add(egui::Shape::line(
                                        [(-7.0,0.0),(-2.0,5.0),(8.0,-6.0)].map(|(x,y)| center + egui::vec2(x,y)).to_vec(),
                                        Stroke::new(2.7, Color32::WHITE),
                                    ));
                                    ui.add_space(8.0);
                                    ui.vertical(|ui| {
                                        ui.set_width(ui.available_width());
                                        ui.label(icons::semibold(tr(lang, "about.privacy_title"), 17.0).color(p.text));
                                        ui.label(RichText::new(tr(lang, "about.privacy")).size(14.0).color(p.text));
                                        ui.label(RichText::new(tr(lang, "about.privacy_updates")).size(13.0).color(p.text_muted));
                                    });
                                });
                            });
                        section_break(ui);
                        ui.label(icons::semibold(tr(lang, "about.updates_recovery"), 19.0).color(p.text));
                        updates(ui);
                        section_break(ui);
                        ui.label(icons::semibold(tr(lang, "about.licenses"), 16.0).color(p.text));
                        ui.columns(2, |columns| {
                            columns[0].label(RichText::new(tr(lang, "about.engine")).size(13.0).color(p.text));
                            columns[0].label(RichText::new(tr(lang, "about.license")).size(13.0).color(p.text));
                            columns[1].add_space(8.0);
                            let link_width = columns[1].available_width();
                            columns[1].allocate_ui_with_layout(egui::vec2(link_width, 22.0), Layout::right_to_left(Align::Center), |ui| {
                                ui.hyperlink_to(RichText::new(tr(lang, "about.third_party")).size(13.0).color(p.accent),
                                    "https://github.com/lmdanavanem/SuniPDF/blob/main/docs/THIRD_PARTY_NOTICES.md");
                            });
                        });
                        section_break(ui);
                        ui.label(icons::semibold(tr(lang, "about.credit"), 17.0).color(p.text));
                        ui.label(RichText::new(tr(lang, "about.author")).size(14.0).color(p.text));
                        ui.hyperlink_to(tr(lang, "about.website"), "https://sunitech.ch/");
                        ui.add_space(14.0);
                    });
            });
            separator(ui);
            Frame::new().fill(p.chrome)
                .corner_radius(egui::CornerRadius { nw: 0, ne: 0, sw: 12, se: 12 })
                .inner_margin(Margin::symmetric(18, 9)).show(ui, |ui| {
                    ui.set_width(width - 36.0);
                    ui.allocate_ui_with_layout(egui::vec2(width - 36.0, 34.0), Layout::right_to_left(Align::Center), |ui| {
                        close |= crate::dialog::primary(ui, tr(lang, "about.close"));
                    });
                });
            close |= ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        });
    close
}

fn separator(ui: &mut egui::Ui) {
    let p = theme::palette_for(ui.ctx());
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter()
        .hline(rect.x_range(), rect.center().y, Stroke::new(1.0, p.border));
}

fn section_break(ui: &mut egui::Ui) {
    ui.add_space(8.0);
    separator(ui);
    ui.add_space(6.0);
}

pub fn update_button(ui: &mut egui::Ui, enabled: bool, label: &str) -> bool {
    let p = theme::palette_for(ui.ctx());
    let response = ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(format!("     {label}")).color(p.accent))
            .fill(p.surface)
            .stroke(Stroke::new(1.0, p.accent))
            .corner_radius(8)
            .min_size(egui::vec2(0.0, 34.0)),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, label));
    let center = egui::pos2(response.rect.left() + 14.0, response.rect.center().y);
    let stroke = Stroke::new(
        1.5,
        if enabled {
            p.accent
        } else {
            p.accent.gamma_multiply(0.4)
        },
    );
    ui.painter().add(egui::Shape::line(
        (0..=24)
            .map(|index| {
                let angle = std::f32::consts::PI * (0.1 + index as f32 * 1.7 / 24.0);
                center + egui::vec2(angle.cos(), angle.sin()) * 7.0
            })
            .collect(),
        stroke,
    ));
    ui.painter().add(egui::Shape::line(
        [(2.0, -6.0), (6.0, -4.0), (7.0, -8.0)]
            .map(|(x, y)| center + egui::vec2(x, y))
            .to_vec(),
        stroke,
    ));
    response.clicked()
}

enum Feature {
    Offline,
    Privacy,
    Account,
}

fn badge(ui: &mut egui::Ui, lang: Language, feature: Feature, key: &str) {
    let p = theme::palette_for(ui.ctx());
    Frame::new()
        .fill(p.accent_soft)
        .stroke(Stroke::new(1.0, p.accent.gamma_multiply(0.2)))
        .corner_radius(20)
        .inner_margin(Margin::symmetric(12, 7))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(19.0, 19.0), egui::Sense::hover());
                let painter = ui.painter();
                let c = rect.center();
                let stroke = Stroke::new(1.6, p.accent);
                let line = |points: &[(f32, f32)]| {
                    painter.add(egui::Shape::line(
                        points.iter().map(|&(x, y)| c + egui::vec2(x, y)).collect(),
                        stroke,
                    ));
                };
                match feature {
                    Feature::Offline => {
                        for radius in [5.0, 9.0] {
                            painter.add(egui::Shape::line(
                                (0..=16)
                                    .map(|i| {
                                        let angle =
                                            std::f32::consts::PI * (0.2 + i as f32 * 0.6 / 16.0);
                                        c + egui::vec2(
                                            angle.cos() * radius,
                                            4.0 - angle.sin() * radius,
                                        )
                                    })
                                    .collect(),
                                stroke,
                            ));
                        }
                        painter.circle_filled(c + egui::vec2(0.0, 5.0), 1.5, p.accent);
                        line(&[(-8.0, -8.0), (8.0, 8.0)]);
                    }
                    Feature::Privacy => {
                        line(&[
                            (-9.0, 0.0),
                            (-5.0, -5.0),
                            (0.0, -7.0),
                            (5.0, -5.0),
                            (9.0, 0.0),
                            (5.0, 5.0),
                            (0.0, 7.0),
                            (-5.0, 5.0),
                            (-9.0, 0.0),
                        ]);
                        painter.circle_stroke(c, 3.0, stroke);
                        line(&[(-8.0, -8.0), (8.0, 8.0)]);
                    }
                    Feature::Account => {
                        painter.circle_stroke(c + egui::vec2(0.0, -5.0), 3.3, stroke);
                        painter.rect_stroke(
                            egui::Rect::from_center_size(
                                c + egui::vec2(0.0, 5.0),
                                egui::vec2(13.0, 7.0),
                            ),
                            3,
                            stroke,
                            egui::StrokeKind::Inside,
                        );
                    }
                }
                ui.label(RichText::new(tr(lang, key)).size(14.0).color(p.text));
            });
        });
}

/// A full-width, keyboard-accessible preference row with a native-style switch.
pub fn preference(ui: &mut egui::Ui, value: &mut bool, label: &str, enabled: bool) {
    let p = theme::palette_for(ui.ctx());
    ui.add_enabled_ui(enabled, |ui| {
        let height = 44.0;
        let (rect, mut response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), height),
            egui::Sense::click(),
        );
        if response.clicked() {
            *value = !*value;
            response.mark_changed();
        }
        response
            .widget_info(|| egui::WidgetInfo::selected(egui::Role::Switch, enabled, *value, label));
        if response.hovered() {
            ui.painter().rect_filled(rect, 6, p.hover);
        }
        let galley = ui.painter().layout(
            label.to_owned(),
            egui::FontId::proportional(14.0),
            p.text,
            (rect.width() - 80.0).max(1.0),
        );
        ui.painter().galley(
            egui::pos2(rect.left() + 14.0, rect.center().y - galley.size().y / 2.0),
            galley,
            p.text,
        );
        let switch = egui::Rect::from_center_size(
            egui::pos2(rect.right() - 36.0, rect.center().y),
            egui::vec2(44.0, 25.0),
        );
        ui.painter().rect_filled(
            switch,
            13,
            if *value { p.accent_fill } else { p.text_muted },
        );
        let x = if *value {
            switch.right() - 12.5
        } else {
            switch.left() + 12.5
        };
        ui.painter()
            .circle_filled(egui::pos2(x, switch.center().y), 9.0, Color32::WHITE);
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect.shrink(1.0),
                6,
                Stroke::new(2.0, p.focus),
                egui::StrokeKind::Inside,
            );
        }
    });
}
