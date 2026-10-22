use crate::{annotate::Tool, app::SuniApp, icons::glyph, theme};
use sunipdf_core::Command;
use sunipdf_i18n::tr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NavigationPanel {
    Tools,
    Pages,
    Bookmarks,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContextPanel {
    Measurements,
    Comments,
    Objects,
    Forms,
}

impl ContextPanel {
    fn from_toggle(command: Command) -> Option<Self> {
        match command {
            Command::ToggleComments => Some(Self::Comments),
            Command::ToggleObjects => Some(Self::Objects),
            Command::ToggleForms => Some(Self::Forms),
            _ => None,
        }
    }
}

impl NavigationPanel {
    fn command(self) -> Command {
        match self {
            Self::Tools => Command::ToggleTools,
            Self::Pages => Command::ToggleThumbnails,
            Self::Bookmarks => Command::ToggleBookmarks,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Tools => "reading.tools",
            Self::Pages => "menu.pages",
            Self::Bookmarks => "bookmarks.title",
        }
    }
}

impl SuniApp {
    pub(crate) fn context_panel(&self) -> Option<ContextPanel> {
        if self.document.is_none() || self.organize.active {
            None
        } else if self.annotate.tool == Tool::Measure {
            Some(ContextPanel::Measurements)
        } else if self.annotate.comments_open {
            Some(ContextPanel::Comments)
        } else if self.edit.objects_open {
            Some(ContextPanel::Objects)
        } else if self.forms.panel_open {
            Some(ContextPanel::Forms)
        } else {
            None
        }
    }

    pub(crate) fn measurement_blocks_panel_command(&self, command: Command) -> bool {
        ContextPanel::from_toggle(command).is_some() && self.annotate.measure.is_active_draft()
    }

    pub(crate) fn toggle_context_panel(&mut self, command: Command) {
        let Some(panel) = ContextPanel::from_toggle(command) else {
            return;
        };
        self.set_context_panel(command, self.context_panel() != Some(panel));
    }

    pub(crate) fn set_context_panel(&mut self, command: Command, open: bool) {
        let Some(panel) = ContextPanel::from_toggle(command) else {
            return;
        };
        if self.measurement_blocks_panel_command(command) {
            self.status_msg = Some(self.t("measure.finish_draft_first").to_owned());
            return;
        }
        let next = open.then_some(panel);
        if next.is_some() && self.annotate.workspace == Some(crate::workspace::Workspace::Measure) {
            self.leave_workspace();
        }
        self.annotate.comments_open = next == Some(ContextPanel::Comments);
        self.edit.objects_open = next == Some(ContextPanel::Objects);
        self.forms.panel_open = next == Some(ContextPanel::Forms);
        if command == Command::ToggleComments {
            self.annotate.annotations_page = None;
        }
        if command == Command::ToggleObjects {
            self.edit.objects_page = None;
        }
        if self.forms.panel_open {
            self.forms.stale = true;
        }
        if self.status_msg.as_deref() == Some(self.t("measure.finish_draft_first")) {
            self.status_msg = None;
        }
    }

    pub(crate) fn navigation_panel(&self) -> Option<NavigationPanel> {
        // Older preferences allowed several left panels; display one without rewriting them.
        if self.document.is_some() && self.docinfo.bookmarks_open && !self.organize.active {
            Some(NavigationPanel::Bookmarks)
        } else if self.show_tools {
            Some(NavigationPanel::Tools)
        } else if self.document.is_some() && self.settings.show_thumbnails && !self.organize.active
        {
            Some(NavigationPanel::Pages)
        } else {
            None
        }
    }

    pub(crate) fn toggle_navigation_panel(&mut self, panel: NavigationPanel) {
        let next = (self.navigation_panel() != Some(panel)).then_some(panel);
        self.set_navigation_panel(next);
    }

    pub(crate) fn set_navigation_panel(&mut self, next: Option<NavigationPanel>) {
        self.show_tools = next == Some(NavigationPanel::Tools);
        self.settings.show_tools = self.show_tools;
        self.settings.show_thumbnails = next == Some(NavigationPanel::Pages);
        self.docinfo.bookmarks_open = next == Some(NavigationPanel::Bookmarks);
        self.persist_settings();
    }

    pub(crate) fn navigation_dock(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let Some(selected) = self.navigation_panel() else {
            return;
        };
        let p = theme::palette(self.settings.dark_theme);
        let max_width = (ui.available_width() - 220.0).clamp(160.0, 320.0);
        let min_width = 220.0_f32.min(max_width);
        egui::Panel::left("navigation-dock")
            .resizable(true)
            .default_size(240.0_f32.min(max_width))
            .size_range(min_width..=max_width)
            .frame(egui::Frame::new().fill(p.surface).inner_margin(8))
            .show(ui, |ui| {
                let lang = self.lang();
                let mut run = None;
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    for panel in [
                        NavigationPanel::Tools,
                        NavigationPanel::Pages,
                        NavigationPanel::Bookmarks,
                    ] {
                        let enabled = panel == NavigationPanel::Tools
                            || (self.document.is_some() && !self.organize.active);
                        let response = ui.add_enabled(
                            enabled,
                            egui::Button::selectable(selected == panel, tr(lang, panel.label())),
                        );
                        if response.clicked() && selected != panel {
                            run = Some(panel.command());
                        }
                    }
                    if self.icon_button(ui, glyph::CLOSE, "×", tr(lang, "tools.close")) {
                        run = Some(selected.command());
                    }
                });
                ui.separator();
                match selected {
                    NavigationPanel::Tools => self.tools_panel(ctx, ui),
                    NavigationPanel::Pages | NavigationPanel::Bookmarks => {
                        ui.add_enabled_ui(
                            !self.ink_draft_active()
                                && !self.annotate.measure.is_active_draft()
                                && self.annotate.adv.free_text.is_none(),
                            |ui| match selected {
                                NavigationPanel::Pages => self.thumbnails_panel(ctx, ui),
                                NavigationPanel::Bookmarks => self.bookmarks_panel(ctx, ui),
                                NavigationPanel::Tools => unreachable!(),
                            },
                        );
                    }
                }
                if let Some(command) = run {
                    self.execute(ctx, command);
                }
            });
    }

    pub(crate) fn measurement_dock(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if self.context_panel() != Some(ContextPanel::Measurements) {
            return;
        }
        let p = theme::palette(self.settings.dark_theme);
        let max_width = (ui.available_width() * 0.42).clamp(260.0, 380.0);
        egui::Panel::right("measurement-properties")
            .resizable(true)
            .default_size(300.0_f32.min(max_width))
            .size_range(260.0..=max_width)
            .frame(egui::Frame::new().fill(p.surface).inner_margin(10))
            .show(ui, |ui| {
                let mut leave = false;
                ui.horizontal(|ui| {
                    ui.strong(self.t("measure.tool"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        leave = self.icon_button(ui, glyph::CLOSE, "×", self.t("hub.leave"));
                    });
                });
                ui.separator();
                if leave {
                    self.execute(ctx, Command::HubLeave);
                }
                egui::ScrollArea::vertical()
                    .id_salt("measurement-properties-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.measurement_bar(ctx, ui));
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker::Request;
    use sunipdf_core::measurement::{
        Measurement, MeasurementKind, MeasurementRecord, MeasurementUnit, PageCalibration,
    };

    #[test]
    fn context_commands_switch_the_visible_panel_and_leave_idle_measurements() {
        let (mut app, _, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.annotate.comments_open = true;
        app.edit.objects_open = true;
        app.forms.panel_open = true;
        assert_eq!(app.context_panel(), Some(ContextPanel::Comments));
        for (command, panel) in [
            (Command::ToggleObjects, ContextPanel::Objects),
            (Command::ToggleForms, ContextPanel::Forms),
            (Command::ToggleComments, ContextPanel::Comments),
        ] {
            app.execute(&ctx, command);
            assert_eq!(app.context_panel(), Some(panel));
            assert_eq!(app.annotate.comments_open, panel == ContextPanel::Comments);
            assert_eq!(app.edit.objects_open, panel == ContextPanel::Objects);
            assert_eq!(app.forms.panel_open, panel == ContextPanel::Forms);
            app.execute(&ctx, command);
            assert_eq!(app.context_panel(), None);
            app.execute(&ctx, Command::ToolMeasure);
            assert_eq!(app.context_panel(), Some(ContextPanel::Measurements));
            app.execute(&ctx, command);
            assert_eq!(app.context_panel(), Some(panel));
            assert_ne!(
                app.annotate.workspace,
                Some(crate::workspace::Workspace::Measure)
            );
            assert_ne!(app.annotate.tool, Tool::Measure);
            assert!(!app.has_unsaved_changes());
        }
        app.annotate.comments_open = true;
        app.edit.objects_open = true;
        app.execute(&ctx, Command::ToolFillForm);
        assert_eq!(app.context_panel(), Some(ContextPanel::Forms));
        assert!(!app.annotate.comments_open && !app.edit.objects_open);
        assert!(app.forms.panel_open && app.forms.stale);
    }

    #[test]
    fn leaving_measurements_cancels_drafts_and_keeps_saved_data_scale_and_history() {
        for geometry in [false, true] {
            for blocked in [
                Command::ToggleComments,
                Command::ToggleObjects,
                Command::ToggleForms,
            ] {
                for leave in [Command::HubLeave, Command::ToolNote, Command::HubEdit] {
                    let (mut app, requests, _) = crate::app::tests::test_app();
                    let ctx = egui::Context::default();
                    let doc = app.document.as_mut().unwrap();
                    doc.info.page_sizes = vec![doc.info.page_sizes[0]; 3];
                    app.execute(&ctx, Command::ToolMeasure);
                    let scale = PageCalibration {
                        points: [(20.0, 20.0), (120.0, 20.0)],
                        known_distance: 5.0,
                        unit: MeasurementUnit::Metres,
                    };
                    assert!(app.annotate.measure.use_calibration(0, &scale));
                    let record = MeasurementRecord {
                        index: 0,
                        annotation_id: Some("saved-measurement".into()),
                        measurement: Measurement {
                            kind: MeasurementKind::Length,
                            points: vec![(20.0, 20.0), (220.0, 20.0)],
                            calibration: scale.points,
                            known_distance: scale.known_distance,
                            unit: scale.unit,
                            precision: 2,
                        },
                    };
                    app.request_measurement_page(0);
                    let (epoch, generation) = requests
                        .try_iter()
                        .find_map(|request| {
                            if let Request::MeasurementPage {
                                epoch, generation, ..
                            } = request
                            {
                                Some((epoch, generation))
                            } else {
                                None
                            }
                        })
                        .unwrap();
                    app.on_measurement_page(
                        0,
                        epoch,
                        generation,
                        Ok(crate::measurement_saved::MeasurementPageData::new(
                            Ok(Some(scale.clone())),
                            Ok(vec![record.clone()]),
                            true,
                        )),
                    );
                    app.view.reading.history.jump(0, None, 1, 1.0);
                    app.can_undo = true;
                    app.can_redo = true;
                    if geometry {
                        assert!(app.start_measurement_geometry(0, record.clone()));
                    } else {
                        app.annotate.measure.start_calibration();
                    }
                    assert!(app.annotate.measure.is_active_draft());
                    assert!(app.command_enabled(leave));
                    for mutation in [Command::Save, Command::RotateClockwise] {
                        assert!(!app.command_enabled(mutation));
                        app.execute(&ctx, mutation);
                        assert!(app.annotate.measure.is_active_draft());
                    }
                    assert!(!app.command_enabled(blocked));
                    app.execute(&ctx, blocked);
                    assert_eq!(app.context_panel(), Some(ContextPanel::Measurements));
                    assert!(app.annotate.measure.is_active_draft());
                    assert_eq!(
                        app.status_msg.as_deref(),
                        Some(app.t("measure.finish_draft_first"))
                    );
                    app.execute(&ctx, leave);
                    assert!(!app.annotate.measure.is_active_draft());
                    assert!(!app.navigation_blocked());
                    assert_ne!(app.context_panel(), Some(ContextPanel::Measurements));
                    assert_eq!(app.annotate.measure.local_calibration(0), Some(scale));
                    assert!(app.annotate.measure.saved.contains(0, &record));
                    assert!(app.view.reading.history.can_back());
                    assert!(app.can_undo && app.can_redo);
                    assert!(!app.dirty);
                    app.execute(&ctx, Command::NextPage);
                    assert_eq!(app.document.as_ref().unwrap().page, 1);
                    assert!(requests
                        .try_iter()
                        .all(|request| !matches!(request, Request::Edit(_))));
                }
            }
        }
    }

    #[test]
    fn actual_app_renders_one_context_before_navigation_and_keeps_a_readable_canvas() {
        for lang in sunipdf_i18n::Language::ALL {
            for selected in [
                ContextPanel::Measurements,
                ContextPanel::Comments,
                ContextPanel::Objects,
                ContextPanel::Forms,
            ] {
                let (mut app, _, _) = crate::app::tests::test_app();
                app.settings.set_language(lang);
                app.show_tools = true;
                app.settings.show_thumbnails = true;
                app.annotate.comments_open = true;
                app.edit.objects_open = true;
                app.forms.panel_open = true;
                let ctx = egui::Context::default();
                app.has_icons = crate::icons::install(&ctx);
                theme::apply(&ctx, app.settings.dark_theme);
                match selected {
                    ContextPanel::Measurements => app.execute(&ctx, Command::ToolMeasure),
                    ContextPanel::Comments => {}
                    ContextPanel::Objects => app.execute(&ctx, Command::ToggleObjects),
                    ContextPanel::Forms => app.execute(&ctx, Command::ToggleForms),
                }
                let mut root = egui::Id::NULL;
                for _ in 0..3 {
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(720.0, 480.0),
                            )),
                            ..Default::default()
                        },
                        |ui| {
                            root = ui.scope_id();
                            app.render_ui(ui);
                        },
                    );
                    output.textures_delta.clear();
                }
                let navigation = egui::PanelState::load(&ctx, root.with("navigation-dock"))
                    .unwrap()
                    .outer_rect;
                let mut context = None;
                for (panel, salt) in [
                    (ContextPanel::Measurements, "measurement-properties"),
                    (ContextPanel::Comments, "comments"),
                    (ContextPanel::Objects, "objects"),
                    (ContextPanel::Forms, "forms"),
                ] {
                    let state = egui::PanelState::load(&ctx, root.with(salt));
                    assert_eq!(
                        state.is_some(),
                        panel == selected,
                        "only selected context is rendered"
                    );
                    if let Some(state) = state {
                        context = Some(state.outer_rect);
                    }
                }
                let context = context.unwrap();
                assert!(
                    context.left() - navigation.right() >= 218.0,
                    "{lang:?} {selected:?}: left {navigation:?}, right {context:?}"
                );
                assert!(context.right() <= 720.0 && navigation.left() >= 0.0);
            }
        }
    }

    #[test]
    fn navigation_commands_switch_one_panel_and_close_it_without_changing_the_document() {
        let (mut app, _, _) = crate::app::tests::test_app();
        let ctx = egui::Context::default();
        app.show_tools = true;
        app.settings.show_tools = true;
        app.settings.show_thumbnails = true;
        assert_eq!(app.navigation_panel(), Some(NavigationPanel::Tools));
        for panel in [
            NavigationPanel::Pages,
            NavigationPanel::Bookmarks,
            NavigationPanel::Tools,
        ] {
            app.execute(&ctx, panel.command());
            assert_eq!(app.navigation_panel(), Some(panel));
            assert_eq!(app.show_tools, panel == NavigationPanel::Tools);
            assert_eq!(
                app.settings.show_thumbnails,
                panel == NavigationPanel::Pages
            );
            assert_eq!(
                app.docinfo.bookmarks_open,
                panel == NavigationPanel::Bookmarks
            );
            app.execute(&ctx, panel.command());
            assert_eq!(app.navigation_panel(), None);
            assert!(!app.has_unsaved_changes());
            assert_eq!(app.document.as_ref().unwrap().page, 0);
        }
    }
}
