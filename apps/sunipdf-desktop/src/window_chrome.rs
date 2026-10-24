//! Linux client-side window controls with native move and resize gestures.

use egui::{pos2, vec2, Color32, Pos2, Rect, ResizeDirection, Sense, Stroke, ViewportCommand};
use sunipdf_i18n::{tr, Language};

const TITLE_HEIGHT: f32 = 40.0;
const BUTTON_SIZE: f32 = 40.0;
const BUTTON_RADIUS: f32 = 15.0;
const EDGE: f32 = 6.0;

/// Reserve resize gutters so window gestures cannot also activate document tools.
pub(crate) fn frame<R>(
    ui: &mut egui::Ui,
    reading_full_screen: bool,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let fixed = reading_full_screen
        || ui.input(|i| {
            i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)
        });
    if fixed {
        return contents(ui);
    }
    let outer = ui.available_rect_before_wrap();
    let result = egui::Frame::new()
        .fill(ui.visuals().window_fill())
        .inner_margin(EDGE as i8)
        .show(ui, |ui| {
            ui.set_min_size((outer.size() - vec2(EDGE * 2.0, EDGE * 2.0)).max(vec2(0.0, 0.0)));
            contents(ui)
        })
        .inner;
    resize_handles(ui.ctx(), outer);
    result
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Control {
    Minimize,
    Maximize,
    Restore,
    Close,
}

impl Control {
    fn label(self) -> &'static str {
        match self {
            Self::Minimize => "window.minimize",
            Self::Maximize => "window.maximize",
            Self::Restore => "window.restore",
            Self::Close => "window.close",
        }
    }
}

#[derive(Clone, Copy)]
struct Colors {
    bar: Color32,
    button: Color32,
    hover: Color32,
    text: Color32,
}

fn colors(dark: bool) -> Colors {
    if dark {
        Colors {
            bar: Color32::from_gray(36),
            button: Color32::from_gray(54),
            hover: Color32::from_gray(74),
            text: Color32::from_gray(242),
        }
    } else {
        Colors {
            bar: Color32::from_rgb(246, 245, 244),
            button: Color32::from_rgb(222, 221, 218),
            hover: Color32::from_rgb(204, 203, 200),
            text: Color32::from_gray(42),
        }
    }
}

/// Return a close request to the application, which owns unsaved-work protection.
/// Do not emit `Close` here: root viewport commands can bypass a later input frame.
pub(crate) fn show(ui: &mut egui::Ui, language: Language, dark: bool, title: &str) -> bool {
    let palette = colors(dark);
    let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    let mut close = false;
    egui::Panel::top("linux-window-title")
        .exact_size(TITLE_HEIGHT)
        .frame(egui::Frame::new().fill(palette.bar))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            let controls_left = rect.right() - BUTTON_SIZE * 3.0 - 4.0;
            let drag_rect = Rect::from_min_max(rect.min, pos2(controls_left, rect.bottom()));
            let drag = ui.interact(
                drag_rect,
                ui.scope_id().with("window-drag"),
                Sense::click_and_drag(),
            );
            if title_double_clicked(&drag) {
                ui.ctx()
                    .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
            } else if drag.drag_started_by(egui::PointerButton::Primary) {
                ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
            }

            let title_rect = drag_rect.shrink2(vec2(12.0, 0.0));
            let galley = ui.painter().layout(
                title.to_owned(),
                egui::FontId::proportional(14.0),
                palette.text,
                f32::INFINITY,
            );
            let title_pos = pos2(
                title_rect
                    .left()
                    .max(rect.center().x - galley.size().x / 2.0),
                rect.center().y - galley.size().y / 2.0,
            );
            ui.painter()
                .with_clip_rect(title_rect)
                .galley(title_pos, galley, palette.text);

            let controls = [
                Control::Minimize,
                if maximized {
                    Control::Restore
                } else {
                    Control::Maximize
                },
                Control::Close,
            ];
            for (index, control) in controls.into_iter().enumerate() {
                let target = Rect::from_min_size(
                    pos2(controls_left + index as f32 * BUTTON_SIZE, rect.top()),
                    vec2(BUTTON_SIZE, TITLE_HEIGHT),
                );
                let response = control_button(ui, target, control, language, palette);
                if response.clicked() {
                    match control {
                        Control::Minimize => {
                            ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true))
                        }
                        Control::Maximize | Control::Restore => ui
                            .ctx()
                            .send_viewport_cmd(ViewportCommand::Maximized(!maximized)),
                        Control::Close => close = true,
                    }
                }
            }
        });
    close
}

fn title_double_clicked(response: &egui::Response) -> bool {
    let ctx = &response.ctx;
    let id = egui::Id::unique(("linux-title-click-pair", ctx.viewport_id()));
    let clear = || ctx.data_mut(|data| data.remove::<(f64, Pos2)>(id));
    if response.drag_started() {
        clear();
        return false;
    }
    if !response.clicked_by(egui::PointerButton::Primary) {
        if ctx.input(|i| i.pointer.any_click()) {
            clear();
        }
        return false;
    }
    let Some(position) = response.interact_pointer_pos() else {
        clear();
        return false;
    };
    let time = ctx.input(|i| i.time);
    let (delay, distance) = ctx.options(|o| {
        (
            o.input_options.max_double_click_delay,
            o.input_options.max_click_dist,
        )
    });
    // Count consecutive title clicks independently of clicks on other window controls.
    ctx.data_mut(|data| {
        let paired = data
            .get_temp::<(f64, Pos2)>(id)
            .is_some_and(|(previous, point)| {
                time >= previous
                    && time - previous < delay
                    && point.distance_sq(position) < distance * distance
            });
        if paired {
            data.remove::<(f64, Pos2)>(id);
        } else {
            data.insert_temp(id, (time, position));
        }
        paired
    })
}

fn control_button(
    ui: &mut egui::Ui,
    target: Rect,
    control: Control,
    language: Language,
    palette: Colors,
) -> egui::Response {
    let label = tr(language, control.label());
    let response = ui.interact(target, ui.scope_id().with(control.label()), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    let active = response.hovered() || response.is_pointer_button_down_on();
    let closing = active && control == Control::Close;
    let background = if closing {
        Color32::from_rgb(192, 28, 40)
    } else if active {
        palette.hover
    } else {
        palette.button
    };
    let foreground = if closing {
        Color32::WHITE
    } else {
        palette.text
    };
    let center = target.center();
    ui.painter()
        .circle_filled(center, BUTTON_RADIUS, background);
    if response.has_focus() {
        ui.painter().circle_stroke(
            center,
            BUTTON_RADIUS + 2.0,
            Stroke::new(2.0, crate::theme::palette(ui.visuals().dark_mode).focus),
        );
    }
    paint_symbol(ui.painter(), center, control, foreground);
    response.on_hover_text(label)
}

fn paint_symbol(painter: &egui::Painter, center: Pos2, control: Control, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    let line = |a: [f32; 2], b: [f32; 2]| {
        painter.line_segment(
            [center + vec2(a[0], a[1]), center + vec2(b[0], b[1])],
            stroke,
        );
    };
    match control {
        Control::Minimize => line([-5.0, 2.5], [5.0, 2.5]),
        Control::Maximize => {
            painter.rect_stroke(
                Rect::from_center_size(center, vec2(9.0, 9.0)),
                0,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        Control::Restore => {
            line([-2.0, -5.0], [5.0, -5.0]);
            line([5.0, -5.0], [5.0, 2.0]);
            painter.rect_stroke(
                Rect::from_min_max(center + vec2(-5.0, -2.0), center + vec2(2.0, 5.0)),
                0,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        Control::Close => {
            line([-4.5, -4.5], [4.5, 4.5]);
            line([4.5, -4.5], [-4.5, 4.5]);
        }
    }
}

fn resize_direction(rect: Rect, pointer: Pos2) -> Option<ResizeDirection> {
    if !rect.contains(pointer) || rect.shrink(EDGE).contains(pointer) {
        return None;
    }
    let corner = 24.0;
    let left = pointer.x < rect.left() + corner;
    let right = pointer.x > rect.right() - corner;
    let top = pointer.y < rect.top() + corner;
    let bottom = pointer.y > rect.bottom() - corner;
    Some(match (left, right, top, bottom) {
        (true, _, true, _) => ResizeDirection::NorthWest,
        (_, true, true, _) => ResizeDirection::NorthEast,
        (true, _, _, true) => ResizeDirection::SouthWest,
        (_, true, _, true) => ResizeDirection::SouthEast,
        (true, _, _, _) => ResizeDirection::West,
        (_, true, _, _) => ResizeDirection::East,
        (_, _, true, _) => ResizeDirection::North,
        _ => ResizeDirection::South,
    })
}

fn resize_cursor(direction: ResizeDirection) -> egui::CursorIcon {
    match direction {
        ResizeDirection::North | ResizeDirection::South => egui::CursorIcon::ResizeVertical,
        ResizeDirection::East | ResizeDirection::West => egui::CursorIcon::ResizeHorizontal,
        ResizeDirection::NorthEast | ResizeDirection::SouthWest => egui::CursorIcon::ResizeNeSw,
        ResizeDirection::NorthWest | ResizeDirection::SouthEast => egui::CursorIcon::ResizeNwSe,
    }
}

fn resize_handles(ctx: &egui::Context, rect: Rect) {
    let regions = [
        Rect::from_min_max(rect.min, pos2(rect.right(), rect.top() + EDGE)),
        Rect::from_min_max(pos2(rect.left(), rect.bottom() - EDGE), rect.max),
        Rect::from_min_max(
            pos2(rect.left(), rect.top() + EDGE),
            pos2(rect.left() + EDGE, rect.bottom() - EDGE),
        ),
        Rect::from_min_max(
            pos2(rect.right() - EDGE, rect.top() + EDGE),
            pos2(rect.right(), rect.bottom() - EDGE),
        ),
    ];
    for (index, region) in regions.into_iter().enumerate() {
        egui::Area::new(egui::Id::unique(("linux-window-resize", index)))
            .fixed_pos(region.min)
            .movable(false)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let (target, response) = ui.allocate_exact_size(region.size(), Sense::drag());
                if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                    if target.contains(pointer) {
                        if let Some(direction) = resize_direction(rect, pointer) {
                            ui.ctx().set_cursor_icon(resize_cursor(direction));
                            if response.is_pointer_button_down_on()
                                && ui.input(|i| {
                                    i.pointer.button_pressed(egui::PointerButton::Primary)
                                })
                            {
                                ui.ctx()
                                    .send_viewport_cmd(ViewportCommand::BeginResize(direction));
                            }
                        }
                    }
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_frame(
        ctx: &egui::Context,
        input: egui::RawInput,
        draw: impl FnMut(&mut egui::Ui),
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(input, draw);
        output.textures_delta.clear();
        output
    }

    fn input(events: Vec<egui::Event>, maximized: bool) -> egui::RawInput {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(720.0, 480.0))),
            events,
            ..Default::default()
        };
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .maximized = Some(maximized);
        input
    }

    fn pointer(pos: Pos2, pressed: bool) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }

    #[test]
    fn window_buttons_send_only_their_command_and_close_is_guarded_by_the_caller() {
        for (x, maximized, expected, closes) in [
            (616.0, false, Some(ViewportCommand::Minimized(true)), false),
            (656.0, false, Some(ViewportCommand::Maximized(true)), false),
            (656.0, true, Some(ViewportCommand::Maximized(false)), false),
            (696.0, false, None, true),
        ] {
            let ctx = egui::Context::default();
            let mut close = false;
            let draw = |ui: &mut egui::Ui| {
                close = show(ui, Language::English, true, "SuniPDF");
            };
            let _ = run_frame(&ctx, input(vec![], maximized), draw);
            let down = run_frame(&ctx, input(pointer(pos2(x, 20.0), true), maximized), |ui| {
                close = show(ui, Language::English, true, "SuniPDF");
            });
            assert!(!close);
            assert!(down
                .viewport_output
                .values()
                .all(|output| output.commands.is_empty()));
            let up = run_frame(
                &ctx,
                input(pointer(pos2(x, 20.0), false), maximized),
                |ui| {
                    close = show(ui, Language::English, true, "SuniPDF");
                },
            );
            let commands: Vec<_> = up
                .viewport_output
                .values()
                .flat_map(|output| output.commands.clone())
                .collect();
            assert_eq!(commands, expected.into_iter().collect::<Vec<_>>());
            assert_eq!(close, closes);
            assert!(!commands.contains(&ViewportCommand::StartDrag));
            assert!(!commands.contains(&ViewportCommand::Close));
        }
    }

    #[test]
    fn resize_corners_and_edges_never_overlap_content() {
        let rect = Rect::from_min_size(Pos2::ZERO, vec2(720.0, 480.0));
        for (point, expected) in [
            (pos2(2.0, 2.0), ResizeDirection::NorthWest),
            (pos2(718.0, 2.0), ResizeDirection::NorthEast),
            (pos2(2.0, 478.0), ResizeDirection::SouthWest),
            (pos2(718.0, 478.0), ResizeDirection::SouthEast),
            (pos2(360.0, 2.0), ResizeDirection::North),
            (pos2(360.0, 478.0), ResizeDirection::South),
            (pos2(2.0, 240.0), ResizeDirection::West),
            (pos2(718.0, 240.0), ResizeDirection::East),
        ] {
            assert_eq!(resize_direction(rect, point), Some(expected));
        }
        for point in [
            pos2(6.0, 6.0),
            pos2(700.0, 20.0),
            pos2(360.0, 240.0),
            pos2(-1.0, 20.0),
        ] {
            assert_eq!(resize_direction(rect, point), None);
        }
    }

    #[test]
    fn frame_preserves_content_size_and_full_screen_has_no_resize_gutter() {
        for maximized in [false, true] {
            let ctx = egui::Context::default();
            let _ = run_frame(&ctx, input(vec![], maximized), |ui| {
                frame(ui, false, |ui| {
                    assert_eq!(
                        ui.available_rect_before_wrap(),
                        if maximized {
                            Rect::from_min_max(pos2(0.0, 0.0), pos2(720.0, 480.0))
                        } else {
                            Rect::from_min_max(pos2(6.0, 6.0), pos2(714.0, 474.0))
                        }
                    );
                });
            });
        }
        let ctx = egui::Context::default();
        let _ = run_frame(&ctx, input(vec![], false), |ui| {
            frame(ui, true, |ui| {
                assert_eq!(ui.available_rect_before_wrap().min, Pos2::ZERO);
            })
        });
    }

    #[test]
    fn title_double_click_remains_a_pair_after_another_window_button() {
        for (maximized, prior_restore) in [(false, false), (true, false), (false, true)] {
            let ctx = egui::Context::default();
            let draw = |ui: &mut egui::Ui| {
                frame(ui, false, |ui| {
                    assert!(!show(ui, Language::English, true, "Document.pdf"));
                });
            };
            let timed = |time, events, maximized| {
                let mut input = input(events, maximized);
                input.time = Some(time);
                input
            };
            let _ = run_frame(&ctx, timed(0.0, vec![], maximized || prior_restore), draw);
            if prior_restore {
                let _ = run_frame(
                    &ctx,
                    timed(0.10, pointer(pos2(656.0, 20.0), true), true),
                    draw,
                );
                let restored = run_frame(
                    &ctx,
                    timed(0.12, pointer(pos2(656.0, 20.0), false), true),
                    draw,
                );
                assert!(restored
                    .viewport_output
                    .values()
                    .any(|v| v.commands.contains(&ViewportCommand::Maximized(false))));
            }
            let _ = run_frame(&ctx, timed(0.14, vec![], maximized), draw);
            let _ = run_frame(
                &ctx,
                timed(0.16, pointer(pos2(220.0, 26.0), true), maximized),
                draw,
            );
            let first = run_frame(
                &ctx,
                timed(0.17, pointer(pos2(220.0, 26.0), false), maximized),
                draw,
            );
            assert!(first
                .viewport_output
                .values()
                .all(|v| v.commands.is_empty()));
            let _ = run_frame(
                &ctx,
                timed(0.24, pointer(pos2(220.0, 26.0), true), maximized),
                draw,
            );
            let second = run_frame(
                &ctx,
                timed(0.25, pointer(pos2(220.0, 26.0), false), maximized),
                draw,
            );
            let commands: Vec<_> = second
                .viewport_output
                .values()
                .flat_map(|v| v.commands.clone())
                .collect();
            assert_eq!(
                commands,
                vec![ViewportCommand::Maximized(!maximized)],
                "prior restore: {prior_restore}"
            );
        }
    }

    #[test]
    fn title_drag_is_distinct_from_buttons_and_all_controls_are_named() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let draw = |ui: &mut egui::Ui| {
            assert!(!show(ui, Language::English, true, "Document.pdf — SuniPDF"));
        };
        let first = run_frame(&ctx, input(vec![], false), draw);
        let nodes = &first.platform_output.accesskit_update.unwrap().nodes;
        for key in ["window.minimize", "window.maximize", "window.close"] {
            let label = tr(Language::English, key);
            assert_ne!(label, key);
            assert!(nodes
                .iter()
                .any(|(_, node)| node.role() == egui::accesskit::Role::Button
                    && node.label() == Some(label)));
        }
        let _ = run_frame(&ctx, input(pointer(pos2(200.0, 20.0), true), false), draw);
        let moved = run_frame(
            &ctx,
            input(vec![egui::Event::PointerMoved(pos2(240.0, 24.0))], false),
            draw,
        );
        let commands: Vec<_> = moved
            .viewport_output
            .values()
            .flat_map(|output| output.commands.clone())
            .collect();
        assert_eq!(commands, vec![ViewportCommand::StartDrag]);
    }
}
