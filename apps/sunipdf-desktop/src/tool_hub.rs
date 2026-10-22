//! Searchable task families for the tools dock.

use crate::app::SuniApp;
use crate::icons::{self, glyph};
use crate::theme;
use crate::workspace::{bar_button, BarButton, Entry, Workspace, WsInfo};
use egui::RichText;
use sunipdf_i18n::{tr, Language};

struct ToolFamily {
    title_key: &'static str,
    tools: &'static [Workspace],
}

const FAMILIES: &[ToolFamily] = &[
    ToolFamily {
        title_key: "tools.group.review",
        tools: &[
            Workspace::Comment,
            Workspace::Measure,
            Workspace::Quantity,
            Workspace::Library,
            Workspace::RevisionOverlay,
            Workspace::Stamp,
        ],
    },
    ToolFamily {
        title_key: "tools.group.edit",
        tools: &[Workspace::Edit, Workspace::Organize, Workspace::Redact],
    },
    ToolFamily {
        title_key: "tools.group.forms",
        tools: &[
            Workspace::FillSign,
            Workspace::PrepareForm,
            Workspace::Certificate,
        ],
    },
    ToolFamily {
        title_key: "tools.group.convert",
        tools: &[
            Workspace::Create,
            Workspace::Combine,
            Workspace::Export,
            Workspace::ScanOcr,
            Workspace::Compress,
        ],
    },
    ToolFamily {
        title_key: "tools.group.security",
        tools: &[Workspace::Protect],
    },
];

fn query_id() -> egui::Id {
    egui::Id::new("tools-search-query")
}

fn matches_query(lang: Language, family: &ToolFamily, ws: Workspace, query: &str) -> bool {
    let info = ws.info();
    if info.entry.is_none() {
        return false;
    }
    if query.trim().is_empty() {
        return true;
    }
    let searchable = format!(
        "{} {} {}",
        tr(lang, family.title_key),
        tr(lang, info.name_key),
        tr(lang, info.desc_key)
    )
    .to_lowercase();
    query
        .split_whitespace()
        .all(|word| searchable.contains(&word.to_lowercase()))
}

/// Names keep their full accessible label and tooltip even when a narrow dock wraps them.
fn compact_tool_row(
    ui: &mut egui::Ui,
    has_icons: bool,
    info: &WsInfo,
    label: &str,
    description: &str,
    active: bool,
) -> egui::Response {
    let dark = ui.visuals().dark_mode;
    let p = theme::palette(dark);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::click());
    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, active, label));
    let response = response
        .on_hover_text(format!("{label}\n{description}"))
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if ui.is_rect_visible(rect) {
        let hover = theme::hover_fade(
            ui,
            response.id.with("hover"),
            response.hovered() && !active,
            p.hover,
        );
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            theme::radius::ROW,
            if active { p.accent_soft } else { hover },
        );
        if active {
            painter.rect_filled(
                egui::Rect::from_min_size(
                    rect.min + egui::vec2(0.0, 8.0),
                    egui::vec2(3.0, rect.height() - 16.0),
                ),
                2.0,
                info.color,
            );
        }
        let badge =
            egui::Rect::from_min_size(rect.min + egui::vec2(8.0, 8.0), egui::vec2(24.0, 24.0));
        crate::shell::paint_badge(painter, badge, has_icons, info.icon, info.color, dark);
        let text_left = badge.right() + 10.0;
        let text_right = rect.right() - 24.0;
        let mut job = egui::text::LayoutJob::simple(
            label.to_string(),
            egui::FontId::proportional(13.0),
            p.text,
            (text_right - text_left).max(1.0),
        );
        job.wrap.max_rows = 2;
        job.wrap.break_anywhere = text_right - text_left < 110.0;
        let galley = painter.layout_job(job);
        let text_rect = egui::Rect::from_min_max(
            egui::pos2(text_left, rect.top() + 2.0),
            egui::pos2(text_right, rect.bottom() - 2.0),
        );
        painter
            .with_clip_rect(text_rect.intersect(ui.clip_rect()))
            .galley(
                egui::pos2(text_left, rect.center().y - galley.size().y / 2.0),
                galley,
                p.text,
            );
        if matches!(info.entry, Some(Entry::Mode(_))) {
            painter.text(
                egui::pos2(rect.right() - 12.0, rect.center().y),
                egui::Align2::CENTER_CENTER,
                if has_icons {
                    glyph::CHEVRON_RIGHT
                } else {
                    "›"
                },
                if has_icons {
                    egui::FontId::new(11.0, egui::FontFamily::Name(icons::ICONS.into()))
                } else {
                    egui::FontId::proportional(14.0)
                },
                p.text_muted,
            );
        }
        if response.has_focus() {
            theme::focus_ring(painter, rect, &p, theme::radius::ROW);
        }
    }
    response
}

impl SuniApp {
    pub fn tools_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.lang();
        let p = theme::palette(self.settings.dark_theme);
        let mut query =
            ctx.data_mut(|data| data.get_temp::<String>(query_id()).unwrap_or_default());
        let previous_query = query.clone();
        ui.horizontal(|ui| {
            let width = (ui.available_width() - 34.0 - ui.spacing().item_spacing.x).max(1.0);
            ui.add_sized(
                [width, 30.0],
                egui::TextEdit::singleline(&mut query)
                    .id_salt("tools-search")
                    .hint_text(tr(lang, "tools.search")),
            );
            if bar_button(
                ui,
                self.has_icons,
                BarButton {
                    icon: (glyph::CLOSE, "×"),
                    tint: None,
                    label: tr(lang, "tools.clear_search"),
                    show_label: false,
                    tooltip: tr(lang, "tools.clear_search"),
                    selected: false,
                    enabled: !query.is_empty(),
                },
            )
            .clicked()
            {
                query.clear();
            }
        });
        ctx.data_mut(|data| data.insert_temp(query_id(), query.clone()));
        ui.add_space(8.0);
        let active = self.annotate.workspace;
        let mut run = None;
        let mut count = 0;
        let mut scroll = egui::ScrollArea::vertical()
            .id_salt("tools-by-task")
            .auto_shrink([false, false]);
        if previous_query != query {
            scroll = scroll.vertical_scroll_offset(0.0);
        }
        scroll.show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for family in FAMILIES {
                let tools: Vec<_> = family
                    .tools
                    .iter()
                    .copied()
                    .filter(|ws| matches_query(lang, family, *ws, &query))
                    .collect();
                if tools.is_empty() {
                    continue;
                }
                if count > 0 {
                    ui.add_space(12.0);
                }
                ui.add(
                    egui::Label::new(
                        icons::semibold(tr(lang, family.title_key), 12.0).color(p.text_muted),
                    )
                    .wrap(),
                );
                ui.add_space(4.0);
                for ws in tools {
                    count += 1;
                    let info = ws.info();
                    let response = ui
                        .push_id(("tool", ws as usize), |ui| {
                            compact_tool_row(
                                ui,
                                self.has_icons,
                                info,
                                tr(lang, info.name_key),
                                tr(lang, info.desc_key),
                                active == Some(ws),
                            )
                        })
                        .inner;
                    if response.clicked() {
                        response.surrender_focus();
                        run = ws.activate();
                    }
                }
            }
            if count == 0 {
                ui.add(
                    egui::Label::new(
                        RichText::new(tr(lang, "tools.no_results")).color(p.text_muted),
                    )
                    .wrap(),
                );
            }
            ui.add_space(12.0);
        });
        if let Some(command) = run {
            self.execute(ctx, command);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families_cover_each_available_tool_once_and_measurements_are_easy_to_find() {
        let tools: Vec<_> = FAMILIES
            .iter()
            .flat_map(|family| family.tools)
            .copied()
            .collect();
        assert_eq!(tools.len(), Workspace::available().count());
        for ws in Workspace::available() {
            assert_eq!(
                tools.iter().filter(|tool| **tool == ws).count(),
                1,
                "{ws:?}"
            );
        }
        assert_eq!(FAMILIES[0].tools[1], Workspace::Measure);
        for lang in Language::ALL {
            for family in FAMILIES {
                assert_ne!(tr(lang, family.title_key), family.title_key);
            }
            let title = tr(lang, Workspace::Measure.info().name_key);
            assert!(matches_query(lang, &FAMILIES[0], Workspace::Measure, title));
            assert!(matches_query(
                lang,
                &FAMILIES[0],
                Workspace::Measure,
                &title.to_uppercase()
            ));
            assert!(matches_query(
                lang,
                &FAMILIES[0],
                Workspace::Measure,
                "  \t"
            ));
            assert!(!matches_query(
                lang,
                &FAMILIES[0],
                Workspace::Measure,
                "no-such-tool-781"
            ));
            let description = tr(lang, Workspace::Measure.info().desc_key);
            assert!(matches_query(
                lang,
                &FAMILIES[0],
                Workspace::Measure,
                description
            ));
        }
    }

    fn raw_input(width: f32) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 720.0),
            )),
            ..Default::default()
        }
    }

    fn button_center(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
        let node = output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .map(|(_, node)| node)
            .find(|node| {
                node.label() == Some(label) && node.role() == egui::accesskit::Role::Button
            })
            .unwrap_or_else(|| panic!("Missing button: {label}"));
        let bounds = node.bounds().unwrap();
        egui::pos2(
            ((bounds.x0 + bounds.x1) / 2.0) as f32,
            ((bounds.y0 + bounds.y1) / 2.0) as f32,
        )
    }

    fn click(app: &mut SuniApp, ctx: &egui::Context, input: &egui::RawInput, pos: egui::Pos2) {
        for pressed in [true, false] {
            let mut input = input.clone();
            input.events = vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ];
            let mut output = ctx.run_ui(input, |ui| app.tools_panel(ctx, ui));
            output.textures_delta.clear();
        }
    }

    #[test]
    fn grouped_measurements_activate_by_mouse_and_keyboard_in_both_languages() {
        for lang in Language::ALL {
            for width in [160.0, 220.0, 260.0, 300.0] {
                let (mut app, _, _) = crate::app::tests::test_app();
                app.settings.set_language(lang);
                let ctx = egui::Context::default();
                app.has_icons = icons::install(&ctx);
                ctx.enable_accesskit();
                let input = raw_input(width);
                let mut output = ctx.run_ui(input.clone(), |ui| app.tools_panel(&ctx, ui));
                output.textures_delta.clear();
                let output = ctx.run_ui(input.clone(), |ui| app.tools_panel(&ctx, ui));
                let label = tr(lang, Workspace::Measure.info().name_key);
                let pos = button_center(&output, label);
                assert!(pos.y < 240.0 && pos.x > 0.0 && pos.x < width);
                click(&mut app, &ctx, &input, pos);
                assert_eq!(app.annotate.workspace, Some(Workspace::Measure));
                assert_eq!(app.annotate.tool, crate::annotate::Tool::Measure);

                // Opening an active tool keeps its context instead of hiding it.
                click(&mut app, &ctx, &input, pos);
                assert_eq!(app.annotate.workspace, Some(Workspace::Measure));
                app.leave_workspace();
                let mut focused = false;
                for _ in 0..8 {
                    let mut tab = input.clone();
                    tab.events.push(egui::Event::Key {
                        key: egui::Key::Tab,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    });
                    let mut output = ctx.run_ui(tab, |ui| app.tools_panel(&ctx, ui));
                    output.textures_delta.clear();
                    let focus_color = theme::palette(ctx.style().visuals.dark_mode).focus;
                    focused = output.shapes.iter().any(|shape| {
                        matches!(&shape.shape, egui::Shape::Rect(rect)
                            if rect.rect.contains(pos)
                                && rect.stroke.width == 2.0
                                && rect.stroke.color == focus_color)
                    });
                    let mut release = input.clone();
                    release.events.push(egui::Event::Key {
                        key: egui::Key::Tab,
                        physical_key: None,
                        pressed: false,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    });
                    let mut output = ctx.run_ui(release, |ui| app.tools_panel(&ctx, ui));
                    output.textures_delta.clear();
                    if focused {
                        break;
                    }
                }
                assert!(focused, "Tab must reach the measurement tool");
                let mut keyboard = input.clone();
                keyboard.events.push(egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                });
                let mut output = ctx.run_ui(keyboard, |ui| app.tools_panel(&ctx, ui));
                output.textures_delta.clear();
                assert_eq!(app.annotate.workspace, Some(Workspace::Measure));
                assert_eq!(app.annotate.tool, crate::annotate::Tool::Measure);
            }
        }
    }

    #[test]
    fn tool_labels_wrap_inside_narrow_rows_and_expose_complete_accessible_names() {
        for lang in Language::ALL {
            for dark in [false, true] {
                let ctx = egui::Context::default();
                let has_icons = icons::install(&ctx);
                ctx.enable_accesskit();
                theme::apply(&ctx, dark);
                for width in [160.0, 220.0, 260.0, 300.0] {
                    let mut rectangles = Vec::new();
                    let mut input = raw_input(width);
                    input.screen_rect.as_mut().unwrap().max.y = 1200.0;
                    let mut output = ctx.run_ui(input, |ui| {
                        for ws in Workspace::available() {
                            let info = ws.info();
                            let row = compact_tool_row(
                                ui,
                                has_icons,
                                info,
                                tr(lang, info.name_key),
                                tr(lang, info.desc_key),
                                ws == Workspace::Measure,
                            );
                            rectangles.push((ws, row.rect));
                        }
                    });
                    output.textures_delta.clear();
                    let tree = output.platform_output.accesskit_update.as_ref().unwrap();
                    for (ws, rect) in &rectangles {
                        let title = tr(lang, ws.info().name_key);
                        let label = output
                            .shapes
                            .iter()
                            .find_map(|shape| match &shape.shape {
                                egui::Shape::Text(text) if text.galley.text() == title => {
                                    Some((shape.clip_rect, text))
                                }
                                _ => None,
                            })
                            .unwrap_or_else(|| panic!("Missing title: {title}"));
                        assert!(label.0.min.x >= rect.min.x && label.0.max.x <= rect.max.x);
                        assert!(label.1.galley.rows.len() <= 2);
                        assert!(
                            label.1.pos.y >= rect.min.y
                                && label.1.pos.y + label.1.galley.size().y <= rect.max.y
                        );
                        assert!(label.1.pos.x + label.1.galley.size().x <= rect.max.x - 23.0);
                        assert!(tree
                            .nodes
                            .iter()
                            .any(|(_, node)| node.label() == Some(title)));
                    }
                }
            }
        }
    }

    #[test]
    fn search_filters_rows_and_clear_restores_the_grouped_tools() {
        for lang in Language::ALL {
            let (mut app, _, _) = crate::app::tests::test_app();
            app.settings.set_language(lang);
            let ctx = egui::Context::default();
            app.has_icons = icons::install(&ctx);
            ctx.enable_accesskit();
            let input = raw_input(240.0);
            ctx.data_mut(|data| data.insert_temp(query_id(), "no-such-tool-781".to_string()));
            let mut output = ctx.run_ui(input.clone(), |ui| app.tools_panel(&ctx, ui));
            output.textures_delta.clear();
            assert!(output.shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == tr(lang, "tools.no_results"))
            }));
            assert!(!output
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .any(
                    |(_, node)| node.label() == Some(tr(lang, Workspace::Measure.info().name_key))
                ));
            let pos = button_center(&output, tr(lang, "tools.clear_search"));
            click(&mut app, &ctx, &input, pos);
            assert_eq!(
                ctx.data_mut(|data| data.get_temp::<String>(query_id()))
                    .unwrap(),
                ""
            );
            let mut output = ctx.run_ui(input, |ui| app.tools_panel(&ctx, ui));
            output.textures_delta.clear();
            let _ = button_center(&output, tr(lang, Workspace::Measure.info().name_key));
        }
    }
}
