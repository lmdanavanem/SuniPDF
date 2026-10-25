use crate::app::{SuniApp, Zoom, SCREEN_PER_PDF_POINT};
use crate::worker::{EditOp, Request};
use egui::{vec2, Event, FullOutput, Key, Modifiers, Pos2, Rect};
use std::sync::mpsc::Receiver;
use sunipdf_core::annot_rich::{RichAnnotation, RichAnnotationPreview};
use sunipdf_core::measurement::{Measurement, MeasurementKind, MeasurementRecord, MeasurementUnit};
use sunipdf_core::Command;
use sunipdf_i18n::tr;

struct Canvas {
    ctx: egui::Context,
    time: f64,
    screen_size: egui::Vec2,
    panel_rect: Rect,
    panel_content_size: egui::Vec2,
    panel_scroll: f32,
}

impl Canvas {
    fn new(app: &mut SuniApp) -> Self {
        let ctx = egui::Context::default();
        crate::icons::install(&ctx);
        crate::theme::apply(&ctx, app.settings.dark_theme);
        app.document.as_mut().unwrap().info.form = sunipdf_core::FormKind::None;
        app.zoom = Zoom::Fixed(0.8);
        app.view.mode = crate::view::ViewMode::Single;
        app.execute(&ctx, Command::ToolMeasure);
        let mut canvas = Self {
            ctx,
            time: 1.0,
            screen_size: vec2(1600.0, 1100.0),
            panel_rect: Rect::NOTHING,
            panel_content_size: egui::Vec2::ZERO,
            panel_scroll: 0.0,
        };
        for _ in 0..3 {
            canvas.frame(app, Vec::new());
        }
        canvas
    }

    fn frame(&mut self, app: &mut SuniApp, events: Vec<Event>) -> FullOutput {
        self.time += 0.025;
        let mut output = self.ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.screen_size)),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ui| {
                app.handle_input(&self.ctx);
                egui::Panel::right("measure-integration-controls")
                    .exact_size(280.0)
                    .frame(egui::Frame::new().inner_margin(8))
                    .show(ui, |ui| {
                        let scroll = egui::ScrollArea::vertical()
                            .id_salt("measure-integration-scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| app.measurement_bar(&self.ctx, ui));
                        self.panel_rect = scroll.inner_rect;
                        self.panel_content_size = scroll.content_size;
                        self.panel_scroll = scroll.state.clamped_offset().y;
                    });
                app.document_view(&self.ctx, ui);
                app.saved_measurements_dialog(&self.ctx);
                app.measurement_scale_dialogs(&self.ctx);
                app.rich_annotation_dialog(&self.ctx);
            },
        );
        output.textures_delta.clear();
        output
    }

    fn click(&mut self, app: &mut SuniApp, position: Pos2, rapid: bool) {
        if !rapid {
            self.time += 0.6;
        }
        let button = |pressed| Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.frame(app, vec![Event::PointerMoved(position), button(true)]);
        self.frame(app, vec![button(false)]);
    }

    fn control(&mut self, app: &mut SuniApp, key: &str) {
        let label = tr(app.lang(), key).to_owned();
        self.control_text(app, &label);
    }

    fn control_text(&mut self, app: &mut SuniApp, label: &str) {
        self.frame(app, Vec::new());
        let output = self.frame(app, Vec::new());
        let mut position = text_position(&output.shapes, label);
        if position.is_none() {
            self.scroll_panel(app, 10_000.0);
            for _ in 0..12 {
                let output = self.frame(app, Vec::new());
                position = text_position(&output.shapes, label);
                if position.is_some() {
                    break;
                }
                self.scroll_panel(app, -150.0);
            }
        }
        let position = position.unwrap_or_else(|| panic!("visible measurement control: {label}"));
        self.click(app, position, false);
        self.frame(app, Vec::new());
    }

    fn scroll_panel(&mut self, app: &mut SuniApp, delta: f32) {
        self.frame(
            app,
            vec![
                Event::PointerMoved(self.panel_rect.center()),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: vec2(0.0, delta),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
        for _ in 0..12 {
            self.frame(app, Vec::new());
        }
    }

    fn point(&mut self, app: &mut SuniApp, point: (f32, f32), rapid: bool) {
        self.frame(app, Vec::new());
        let (page, rect, height) = app.edit.page_view.expect("visible PDF page");
        assert_eq!(page, app.document.as_ref().unwrap().page);
        let k = app.effective_zoom * SCREEN_PER_PDF_POINT;
        let position = rect.min + vec2(point.0 * k, (height - point.1) * k);
        assert!(rect.contains(position));
        self.click(app, position, rapid);
    }

    fn key(&mut self, app: &mut SuniApp, key: Key) {
        self.frame(
            app,
            vec![Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        self.frame(
            app,
            vec![Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
    }

    fn rectangle_rotation(&mut self, app: &mut SuniApp, value: &str) {
        self.control(app, "measure.rectangle.rotation");
        self.ctx.memory_mut(|memory| {
            memory.request_focus(egui::Id::unique("dimensioned-rectangle-rotation"))
        });
        self.frame(
            app,
            vec![
                Event::Key {
                    key: Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::COMMAND,
                },
                Event::Text(value.into()),
            ],
        );
        self.key(app, Key::Tab);
    }

    fn calibrate(&mut self, app: &mut SuniApp, known_distance: &str) {
        self.control(app, "measure.calibrate");
        assert!(app.annotate.measure.is_active_draft());
        self.point(app, (60.0, 650.0), false);
        self.point(app, (160.0, 650.0), false);
        app.annotate.measure.input_distance = known_distance.into();
        self.control(app, "measure.confirm_scale");
        assert!(!app.annotate.measure.is_active_draft());
        assert!(app.annotate.adv.rich.is_none());
    }
}

fn text_position(shapes: &[egui::epaint::ClippedShape], label: &str) -> Option<Pos2> {
    fn find(shape: &egui::Shape, label: &str) -> Option<Pos2> {
        match shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, label)),
            _ => None,
        }
    }
    shapes.iter().rev().find_map(|shape| {
        find(&shape.shape, label).filter(|position| shape.clip_rect.contains(*position))
    })
}

fn take_preview(requests: &Receiver<Request>) -> (usize, Measurement, u64, u64) {
    let mut preview = None;
    for request in requests.try_iter() {
        match request {
            Request::PreviewRichAnnotation {
                page,
                annotation: RichAnnotation::Measurement(measurement),
                epoch,
                generation,
                ..
            } => {
                assert!(preview.is_none(), "one preview per completed drawing");
                preview = Some((page, measurement, epoch, generation));
            }
            Request::Edit(_) | Request::Save { .. } => {
                panic!("drawing and preview must not mutate or save the PDF")
            }
            _ => {}
        }
    }
    preview.expect("canvas drawing reached the PDF preview worker")
}

fn assert_no_edit_or_preview(requests: &Receiver<Request>) {
    assert!(requests.try_iter().all(|request| !matches!(
        request,
        Request::Edit(_)
            | Request::Save { .. }
            | Request::PreviewRichAnnotation { .. }
            | Request::PreviewDimensionedRectangle { .. }
            | Request::Suspend
    )));
}

fn preview_image(page: usize) -> RichAnnotationPreview {
    RichAnnotationPreview {
        image: sunipdf_core::RenderedPage {
            page_index: page,
            width: 4,
            height: 2,
            rgba: vec![255; 32],
        },
        scale: 1.0,
        region: sunipdf_core::PixelRect::new(0, 0, 4, 2),
        index: 3,
        prepared: None,
    }
}

#[test]
fn canvas_measurement_controls_calibrate_preview_and_apply_exactly_once() {
    for language in ["en-US", "fr-FR"] {
        let (mut app, requests, responses) = crate::app::tests::test_app();
        app.settings.language = language.into();
        let mut canvas = Canvas::new(&mut app);
        canvas.calibrate(&mut app, "10");
        assert_no_edit_or_preview(&requests);
        canvas.point(&mut app, (80.0, 550.0), false);
        assert!(app.has_unsaved_changes());
        assert_no_edit_or_preview(&requests);
        canvas.point(&mut app, (180.0, 550.0), false);
        let (page, measurement, epoch, generation) = take_preview(&requests);
        assert_eq!(page, 0);
        assert_eq!(measurement.kind, MeasurementKind::Length);
        assert_eq!(measurement.points.len(), 2);
        assert!((measurement.value().unwrap() - 10.0).abs() < 0.001);
        for (actual, expected) in measurement
            .points
            .iter()
            .zip([(80.0, 550.0), (180.0, 550.0)])
        {
            assert!((actual.0 - expected.0).abs() < 0.01);
            assert!((actual.1 - expected.1).abs() < 0.01);
        }
        assert!(app.annotate.adv.rich.is_some());
        app.on_rich_preview(&canvas.ctx, epoch, generation, Ok(preview_image(page)));
        canvas.control(&mut app, "bar.apply");
        canvas.frame(&mut app, Vec::new());
        let edits: Vec<_> = requests
            .try_iter()
            .filter_map(|request| {
                if let Request::Edit(edit) = request {
                    Some(edit)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(edits.len(), 1);
        assert!(matches!(edits[0], EditOp::CommitRichAnnotationPreview {
            epoch: sent_epoch, generation: sent_generation
        } if sent_epoch == epoch && sent_generation == generation));
        responses
            .send(crate::worker::Response::DocumentChanged(
                crate::worker::DocState {
                    info: app.document.as_ref().unwrap().info.clone(),
                    can_undo: true,
                    can_redo: false,
                    pages_unchanged: true,
                    measurement_scales_unchanged: true,
                    modified: true,
                    has_signatures: false,
                },
            ))
            .unwrap();
        app.handle_responses(&canvas.ctx);
        canvas.frame(&mut app, Vec::new());
        assert!(app.annotate.adv.rich.is_none());
        assert!(app.dirty && app.can_undo);
        responses
            .send(crate::worker::Response::MutationFinished)
            .unwrap();
        app.handle_responses(&canvas.ctx);
        assert!(!app.worker.has_pending_mutations());
        assert_no_edit_or_preview(&requests);
    }
}

#[test]
fn dimensioned_rectangle_apply_closes_the_modal_and_allows_next_placement() {
    for language in ["en-US", "fr-FR"] {
        let (mut app, requests, responses) = crate::app::tests::test_app();
        app.settings.language = language.into();
        let mut canvas = Canvas::new(&mut app);
        canvas.calibrate(&mut app, "10");
        canvas.control(&mut app, "measure.rectangle.automatic");
        canvas.control(&mut app, "measure.rectangle.local");
        canvas.rectangle_rotation(&mut app, "33.5");
        canvas.control(&mut app, "measure.rectangle.place");
        canvas.point(&mut app, (80.0, 450.0), false);
        let (page, epoch, generation) = requests
            .try_iter()
            .find_map(|request| match request {
                Request::PreviewDimensionedRectangle {
                    page,
                    rectangle,
                    epoch,
                    generation,
                    ..
                } => {
                    assert_eq!((rectangle.width, rectangle.height), (2.0, 3.0));
                    assert_eq!(rectangle.rotation_degrees, 33.5);
                    Some((page, epoch, generation))
                }
                _ => None,
            })
            .expect("rectangle placement requested its preview");
        responses
            .send(crate::worker::Response::RichAnnotationPreview {
                epoch,
                generation,
                result: Ok(preview_image(page)),
            })
            .unwrap();
        app.handle_responses(&canvas.ctx);
        canvas.control(&mut app, "bar.apply");
        let commits = requests
            .try_iter()
            .filter(|request| {
                matches!(
                    request,
                    Request::Edit(EditOp::CommitRichAnnotationPreview { .. })
                )
            })
            .count();
        assert_eq!(commits, 1);
        responses
            .send(crate::worker::Response::DocumentChanged(
                crate::worker::DocState {
                    info: app.document.as_ref().unwrap().info.clone(),
                    can_undo: true,
                    can_redo: false,
                    pages_unchanged: true,
                    measurement_scales_unchanged: true,
                    modified: true,
                    has_signatures: false,
                },
            ))
            .unwrap();
        app.handle_responses(&canvas.ctx);
        canvas.frame(&mut app, Vec::new());
        assert!(app.annotate.adv.rich.is_none());
        assert!(!app.annotate.adv.dialog_open());
        assert!(app.dirty && app.can_undo);
        responses
            .send(crate::worker::Response::MutationFinished)
            .unwrap();
        app.handle_responses(&canvas.ctx);
        canvas.control(&mut app, "measure.rectangle.place");
        assert!(app.annotate.measure.is_active_draft());
        canvas.key(&mut app, Key::Escape);
        assert!(!app.annotate.measure.is_active_draft());
        assert_no_edit_or_preview(&requests);
    }
}

#[test]
fn dimensioned_rectangle_controls_capture_reference_place_once_and_cancel_without_editing() {
    for language in ["en-US", "fr-FR"] {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.settings.language = language.into();
        let mut canvas = Canvas::new(&mut app);
        canvas.calibrate(&mut app, "10");
        let captures: Vec<_> = requests
            .try_iter()
            .filter_map(|request| match request {
                Request::CaptureRectangleCalibration { page, calibration } => {
                    Some((page, calibration))
                }
                Request::Edit(_) => panic!("calibration must not edit the document"),
                _ => None,
            })
            .collect();
        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].0, 0);
        assert_eq!(captures[0].1.known_distance, 10.0);
        canvas.control(&mut app, "measure.rectangle.automatic");
        canvas.control(&mut app, "measure.rectangle.local");
        canvas.rectangle_rotation(&mut app, "33.5");
        canvas.control(&mut app, "measure.rectangle.place");
        assert!(app.annotate.measure.is_active_draft());
        assert!(!app.suspend_active());
        canvas.point(&mut app, (80.0, 450.0), false);
        let previews: Vec<_> = requests
            .try_iter()
            .filter_map(|request| match request {
                Request::PreviewDimensionedRectangle {
                    page,
                    rectangle,
                    epoch,
                    generation,
                    ..
                } => Some((page, rectangle, epoch, generation)),
                Request::Edit(_) => panic!("placement must not apply the rectangle"),
                _ => None,
            })
            .collect();
        assert_eq!(previews.len(), 1);
        let (page, rectangle, epoch, generation) = &previews[0];
        assert_eq!(*page, 0);
        assert_eq!((rectangle.width, rectangle.height), (2.0, 3.0));
        assert_eq!(rectangle.rotation_degrees, 33.5);
        assert!((rectangle.anchor.0 - 80.0).abs() < 0.01);
        assert!((rectangle.anchor.1 - 450.0).abs() < 0.01);
        assert!(
            matches!(&rectangle.scale, crate::measurement_shapes_ui::RectangleScaleSelection::Local(calibration) if calibration == &captures[0].1)
        );
        app.apply_rich_draft();
        assert_no_edit_or_preview(&requests);
        app.on_rich_preview(&canvas.ctx, *epoch, *generation, Ok(preview_image(*page)));
        canvas.key(&mut app, Key::Escape);
        assert!(app.annotate.adv.rich.is_none());
        assert!(!app.has_unsaved_changes());
        assert_no_edit_or_preview(&requests);
        canvas.control(&mut app, "measure.rectangle.place");
        canvas.key(&mut app, Key::Escape);
        assert!(!app.annotate.measure.is_active_draft());
        assert_no_edit_or_preview(&requests);
    }
}

#[test]
fn canvas_area_double_click_finishes_without_a_duplicate_endpoint() {
    let (mut app, requests, _) = crate::app::tests::test_app();
    let mut canvas = Canvas::new(&mut app);
    canvas.calibrate(&mut app, "10");
    app.annotate.measure.kind = MeasurementKind::Area;
    canvas.point(&mut app, (80.0, 550.0), false);
    canvas.point(&mut app, (180.0, 550.0), false);
    canvas.point(&mut app, (180.0, 450.0), false);
    assert_no_edit_or_preview(&requests);
    canvas.point(&mut app, (180.0, 450.0), true);
    let (_, measurement, _, _) = take_preview(&requests);
    assert_eq!(measurement.kind, MeasurementKind::Area);
    assert_eq!(measurement.points.len(), 3);
    assert!((measurement.value().unwrap() - 50.0).abs() < 0.01);
}

#[test]
fn canvas_cancel_and_backspace_keep_the_document_unchanged() {
    let (mut app, requests, _) = crate::app::tests::test_app();
    let mut canvas = Canvas::new(&mut app);
    canvas.calibrate(&mut app, "10");
    app.annotate.measure.kind = MeasurementKind::Polyline;
    canvas.point(&mut app, (80.0, 550.0), false);
    canvas.point(&mut app, (180.0, 550.0), false);
    canvas.key(&mut app, Key::Backspace);
    canvas.point(&mut app, (80.0, 450.0), false);
    canvas.key(&mut app, Key::Enter);
    let (_, measurement, _, _) = take_preview(&requests);
    assert_eq!(measurement.points.len(), 2);
    assert!((measurement.points[1].0 - 80.0).abs() < 0.01);
    assert!((measurement.value().unwrap() - 10.0).abs() < 0.01);
    canvas.control(&mut app, "dialog.password.cancel");
    assert!(app.annotate.adv.rich.is_none());
    canvas.point(&mut app, (100.0, 500.0), false);
    canvas.key(&mut app, Key::Escape);
    assert!(!app.annotate.measure.is_active_draft());
    assert!(!app.has_unsaved_changes());
    canvas.point(&mut app, (100.0, 500.0), false);
    canvas.control(&mut app, "measure.cancel_draft");
    assert!(!app.annotate.measure.is_active_draft());
    assert!(!app.dirty);
    assert_no_edit_or_preview(&requests);
}

#[test]
fn canvas_draft_blocks_page_tab_undo_save_and_tool_changes() {
    let (mut app, requests, _) = crate::app::tests::test_app();
    let size = app.document.as_ref().unwrap().info.page_sizes[0];
    app.document.as_mut().unwrap().info.page_sizes.push(size);
    let mut canvas = Canvas::new(&mut app);
    canvas.calibrate(&mut app, "10");
    canvas.point(&mut app, (80.0, 550.0), false);
    app.go_to_page(1);
    assert_eq!(app.document.as_ref().unwrap().page, 0);
    assert!(!app.suspend_active());
    for command in [
        Command::Undo,
        Command::Save,
        Command::DeletePage,
        Command::ToolSelect,
    ] {
        app.execute(&canvas.ctx, command);
        assert!(app.annotate.measure.is_active_draft());
        assert_eq!(app.annotate.tool, crate::annotate::Tool::Measure);
    }
    assert_no_edit_or_preview(&requests);
    canvas.key(&mut app, Key::Escape);
    app.go_to_page(1);
    assert_eq!(app.document.as_ref().unwrap().page, 1);
    assert!(!app.dirty);
}

#[test]
fn canvas_uses_each_pages_confirmed_scale_and_discards_stale_calibration() {
    let (mut app, requests, _) = crate::app::tests::test_app();
    let size = app.document.as_ref().unwrap().info.page_sizes[0];
    app.document.as_mut().unwrap().info.page_sizes.push(size);
    let mut canvas = Canvas::new(&mut app);
    canvas.calibrate(&mut app, "10");
    app.go_to_page(1);
    canvas.frame(&mut app, Vec::new());
    canvas.calibrate(&mut app, "25");
    for (page, value) in [(0, 10.0), (1, 25.0)] {
        app.go_to_page(page);
        canvas.frame(&mut app, Vec::new());
        canvas.point(&mut app, (80.0, 550.0), false);
        canvas.point(&mut app, (180.0, 550.0), false);
        let (measured_page, measurement, _, _) = take_preview(&requests);
        assert_eq!(measured_page, page);
        assert!((measurement.value().unwrap() - value).abs() < 0.01);
        canvas.control(&mut app, "dialog.password.cancel");
    }
    app.view.bump_epoch();
    canvas.frame(&mut app, Vec::new());
    canvas.point(&mut app, (80.0, 550.0), false);
    canvas.point(&mut app, (180.0, 550.0), false);
    assert!(app.annotate.adv.rich.is_none());
    assert!(!app.annotate.measure.is_active_draft());
    assert_no_edit_or_preview(&requests);
}

#[test]
fn geometry_drag_survives_scroll_in_continuous_spread_and_book_views() {
    use crate::measurement_saved::MeasurementPageData;
    use sunipdf_core::measurement::{MeasurementRecord, MeasurementUnit};
    for mode in [
        crate::view::ViewMode::Continuous,
        crate::view::ViewMode::TwoPage,
        crate::view::ViewMode::Book,
    ] {
        let (mut app, requests, _) = crate::app::tests::test_app();
        let size = app.document.as_ref().unwrap().info.page_sizes[0];
        app.document.as_mut().unwrap().info.page_sizes = vec![size; 6];
        let mut canvas = Canvas::new(&mut app);
        app.view.mode = mode;
        let record = MeasurementRecord {
            validity: sunipdf_core::measurement::MeasurementValidity::Current,
            index: 0,
            annotation_id: Some("scroll-measurement".into()),
            measurement: Measurement {
                holes: Vec::new(),
                depth: None,
                kind: MeasurementKind::Length,
                points: vec![(80.0, 550.0), (180.0, 550.0)],
                calibration: [(30.0, 40.0), (130.0, 40.0)],
                known_distance: 10.0,
                unit: MeasurementUnit::Metres,
                precision: 2,
            },
        };
        let (epoch, generation) = requests
            .try_iter()
            .find_map(|r| match r {
                Request::MeasurementPage {
                    page: 0,
                    epoch,
                    generation,
                } => Some((epoch, generation)),
                _ => None,
            })
            .expect("measurement page read");
        app.on_measurement_page(
            0,
            epoch,
            generation,
            Ok(MeasurementPageData::new(
                Ok(None),
                Ok(vec![record.clone()]),
                true,
            )),
        );
        assert!(app.start_measurement_geometry(0, record));
        for _ in 0..3 {
            canvas.frame(&mut app, vec![]);
        }
        let (_, rect, height) = app.edit.page_view.expect("active geometry page");
        let k = app.effective_zoom * SCREEN_PER_PDF_POINT;
        let start = rect.min + vec2(180.0 * k, (height - 550.0) * k);
        let moved = start + vec2(30.0, -15.0);
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        canvas.frame(
            &mut app,
            vec![Event::PointerMoved(start), button(start, true)],
        );
        canvas.frame(&mut app, vec![Event::PointerMoved(moved)]);
        canvas.frame(&mut app, vec![Event::PointerMoved(moved + vec2(5.0, 0.0))]);
        let edited = app
            .annotate
            .measure
            .geometry
            .as_ref()
            .unwrap()
            .replacement
            .clone();
        assert_ne!(edited.points[1], (180.0, 550.0));
        app.view.scroll_by(2000.0);
        for _ in 0..3 {
            canvas.frame(&mut app, vec![]);
        }
        assert!(app.view.offset.y > 500.0, "the document really scrolled");
        assert_eq!(
            app.document.as_ref().unwrap().page,
            0,
            "scroll cannot change the draft's current page"
        );
        assert_eq!(
            app.edit.page_view.unwrap().0,
            0,
            "the captured page remains pinned offscreen"
        );
        assert!(app.annotate.measure.geometry.is_some());
        canvas.frame(
            &mut app,
            vec![Event::PointerMoved(moved), button(moved, false)],
        );
        assert!(
            app.annotate.measure.geometry.is_some(),
            "release outside the page retains the draft"
        );
        assert!(
            app.finish_measurement_geometry(),
            "release clears the handle even after scroll"
        );
        assert!(requests
            .try_iter()
            .any(|r| matches!(r, Request::PreviewMeasurementEdit { page: 0, .. })));
    }
}

#[test]
fn canvas_volume_deductions_reach_preview_and_cancel_without_editing_the_document() {
    for language in ["en-US", "fr-FR"] {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.settings.language = language.into();
        let mut canvas = Canvas::new(&mut app);
        canvas.calibrate(&mut app, "10");
        canvas.control(&mut app, "measure.length");
        canvas.control(&mut app, "measure.volume");
        assert_eq!(app.annotate.measure.kind, MeasurementKind::Volume);
        for point in [(80.0, 450.0), (280.0, 450.0), (280.0, 650.0), (80.0, 650.0)] {
            canvas.point(&mut app, point, false);
        }
        canvas.control(&mut app, "measure.deductions.add");
        for point in [
            (130.0, 500.0),
            (180.0, 500.0),
            (180.0, 550.0),
            (130.0, 550.0),
        ] {
            canvas.point(&mut app, point, false);
        }
        canvas.control(&mut app, "measure.undo_point");
        canvas.point(&mut app, (130.0, 550.0), false);
        canvas.control(&mut app, "measure.finish");
        let (_, measurement, _, _) = take_preview(&requests);
        assert_eq!(measurement.kind, MeasurementKind::Volume);
        assert_eq!(measurement.holes.len(), 1);
        assert_eq!(measurement.points.len(), 4);
        assert_eq!(measurement.holes[0].len(), 4);
        assert!((measurement.area_breakdown().unwrap().net - 375.0).abs() < 0.1);
        assert_eq!(measurement.depth.unwrap().unit, MeasurementUnit::Metres);
        canvas.control(&mut app, "dialog.password.cancel");
        assert!(app.annotate.adv.rich.is_none());
        assert_no_edit_or_preview(&requests);
    }
}

#[test]
fn canvas_deduction_geometry_buttons_preserve_outer_contour_and_can_remove_a_local_ring() {
    for language in ["en-US", "fr-FR"] {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.settings.language = language.into();
        let mut canvas = Canvas::new(&mut app);
        let mut original = vertex_record(vec![
            (80.0, 450.0),
            (280.0, 450.0),
            (280.0, 650.0),
            (80.0, 650.0),
        ]);
        original.measurement.kind = MeasurementKind::Area;
        begin_saved_geometry(&mut app, &requests, original.clone());
        canvas.control(&mut app, "measure.deductions.add");
        for point in [
            (130.0, 500.0),
            (180.0, 500.0),
            (180.0, 550.0),
            (130.0, 550.0),
        ] {
            canvas.point(&mut app, point, false);
        }
        canvas.control(&mut app, "measure.deductions.finish");
        canvas.control(&mut app, "measure.geometry.preview");
        let (replacement, epoch, generation) = requests
            .try_iter()
            .find_map(|request| match request {
                Request::PreviewMeasurementEdit {
                    record,
                    measurement,
                    epoch,
                    generation,
                    ..
                } => {
                    assert_eq!(record, original);
                    Some((measurement, epoch, generation))
                }
                _ => None,
            })
            .expect("hole-only edit reaches guarded preview");
        assert_eq!(replacement.points, original.measurement.points);
        assert_eq!(replacement.holes.len(), 1);
        app.on_rich_preview(&canvas.ctx, epoch, generation, Ok(preview_image(0)));
        canvas.control(&mut app, "measure.geometry.adjust");
        canvas.control(&mut app, "measure.deductions.outer");
        let ring = sunipdf_i18n::tr_args(app.lang(), "measure.deductions.ring", &[("number", "1")]);
        canvas.control_text(&mut app, &ring);
        canvas.control(&mut app, "measure.deductions.remove");
        assert_eq!(
            app.annotate.measure.geometry.as_ref().unwrap().replacement,
            original.measurement
        );
        canvas.key(&mut app, Key::Escape);
        assert!(app.annotate.measure.geometry.is_none());
        assert_no_edit_or_preview(&requests);
    }
}

fn begin_saved_geometry(
    app: &mut SuniApp,
    requests: &Receiver<Request>,
    record: MeasurementRecord,
) {
    let queued: Vec<_> = requests.try_iter().collect();
    let (epoch, generation) = queued
        .into_iter()
        .find_map(|r| match r {
            Request::MeasurementPage {
                page: 0,
                epoch,
                generation,
            } => Some((epoch, generation)),
            _ => None,
        })
        .expect("measurement page query");
    app.on_measurement_page(
        0,
        epoch,
        generation,
        Ok(crate::measurement_saved::MeasurementPageData::new(
            Ok(None),
            Ok(vec![record.clone()]),
            true,
        )),
    );
    assert!(app.start_measurement_geometry(0, record));
}

fn vertex_record(points: Vec<(f32, f32)>) -> MeasurementRecord {
    MeasurementRecord {
        validity: sunipdf_core::measurement::MeasurementValidity::Current,
        index: 3,
        annotation_id: Some("vertex-edit".into()),
        measurement: Measurement {
            holes: Vec::new(),
            depth: None,
            kind: MeasurementKind::Polyline,
            points,
            calibration: [(30.0, 40.0), (130.0, 40.0)],
            known_distance: 17.0,
            unit: MeasurementUnit::Feet,
            precision: 4,
        },
    }
}

#[test]
fn canvas_vertex_buttons_insert_remove_adjust_preview_and_apply_one_guarded_edit() {
    for language in ["en-US", "fr-FR"] {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.settings.language = language.into();
        let mut canvas = Canvas::new(&mut app);
        let original = vertex_record(vec![(80.0, 450.0), (280.0, 450.0), (280.0, 650.0)]);
        begin_saved_geometry(&mut app, &requests, original.clone());
        canvas.control(&mut app, "measure.geometry.add_vertex");
        canvas.point(&mut app, (180.0, 450.0), false);
        assert_eq!(
            app.annotate
                .measure
                .geometry
                .as_ref()
                .unwrap()
                .replacement
                .points
                .len(),
            4
        );
        // The successful insertion is one-shot; a rapid second click cannot add a duplicate.
        canvas.point(&mut app, (180.0, 450.0), true);
        assert_eq!(
            app.annotate
                .measure
                .geometry
                .as_ref()
                .unwrap()
                .replacement
                .points
                .len(),
            4
        );
        canvas.control(&mut app, "measure.geometry.remove_vertex");
        assert_eq!(
            app.annotate.measure.geometry.as_ref().unwrap().replacement,
            original.measurement
        );
        canvas.control(&mut app, "measure.geometry.add_vertex");
        canvas.point(&mut app, (180.0, 450.0), false);
        canvas.control(&mut app, "measure.geometry.preview");
        let (replacement, epoch, generation) = requests
            .try_iter()
            .find_map(|r| match r {
                Request::PreviewMeasurementEdit {
                    record,
                    measurement,
                    epoch,
                    generation,
                    ..
                } => {
                    assert_eq!(record, original);
                    Some((measurement, epoch, generation))
                }
                _ => None,
            })
            .expect("geometry preview");
        assert_eq!(replacement.points.len(), 4);
        assert_eq!(replacement.calibration, original.measurement.calibration);
        assert_eq!(
            replacement.known_distance,
            original.measurement.known_distance
        );
        assert_eq!(replacement.unit, original.measurement.unit);
        assert_eq!(replacement.precision, original.measurement.precision);
        app.on_rich_preview(
            &canvas.ctx,
            epoch,
            generation,
            Err(sunipdf_core::EngineError::Other("test refusal".into())),
        );
        canvas.control(&mut app, "measure.geometry.adjust");
        assert_eq!(
            app.annotate.measure.geometry.as_ref().unwrap().replacement,
            replacement
        );
        canvas.control(&mut app, "measure.geometry.select_vertex");
        canvas.control_text(&mut app, "2");
        canvas.control(&mut app, "measure.geometry.remove_vertex");
        assert_eq!(
            app.annotate.measure.geometry.as_ref().unwrap().replacement,
            original.measurement
        );
        canvas.control(&mut app, "measure.geometry.add_vertex");
        canvas.point(&mut app, (180.0, 450.0), false);
        canvas.key(&mut app, Key::Enter);
        let (epoch, generation) = requests
            .try_iter()
            .find_map(|r| match r {
                Request::PreviewMeasurementEdit {
                    epoch, generation, ..
                } => Some((epoch, generation)),
                _ => None,
            })
            .expect("Enter previews valid changed geometry");
        app.on_rich_preview(&canvas.ctx, epoch, generation, Ok(preview_image(0)));
        canvas.control(&mut app, "bar.apply");
        let edits: Vec<_> = requests
            .try_iter()
            .filter_map(|r| match r {
                Request::Edit(edit) => Some(edit),
                _ => None,
            })
            .collect();
        assert_eq!(edits.len(), 1);
        assert!(
            matches!(edits[0], EditOp::CommitRichAnnotationPreview { epoch: e, generation: g } if e == epoch && g == generation)
        );
    }
}

#[test]
fn canvas_numbered_selector_drags_the_chosen_coincident_vertex_and_escape_discards_changes() {
    let (mut app, requests, _) = crate::app::tests::test_app();
    let mut canvas = Canvas::new(&mut app);
    let original = vertex_record(vec![
        (80.0, 450.0),
        (280.0, 450.0),
        (80.0, 450.0),
        (80.0, 650.0),
    ]);
    begin_saved_geometry(&mut app, &requests, original.clone());
    canvas.control(&mut app, "measure.geometry.select_vertex");
    canvas.control_text(&mut app, "3");
    let (_, rect, height) = app.edit.page_view.unwrap();
    let k = app.effective_zoom * SCREEN_PER_PDF_POINT;
    let start = rect.min + vec2(80.0 * k, (height - 450.0) * k);
    let final_point = rect.min + vec2(120.0 * k, (height - 500.0) * k);
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    canvas.frame(
        &mut app,
        vec![Event::PointerMoved(start), button(start, true)],
    );
    canvas.frame(
        &mut app,
        vec![Event::PointerMoved(start + vec2(12.0, -12.0))],
    );
    canvas.frame(
        &mut app,
        vec![Event::PointerMoved(final_point), button(final_point, false)],
    );
    let replacement = &app.annotate.measure.geometry.as_ref().unwrap().replacement;
    assert_eq!(replacement.points[0], original.measurement.points[0]);
    assert!((replacement.points[2].0 - 120.0).abs() < 0.01);
    assert!((replacement.points[2].1 - 500.0).abs() < 0.01);
    canvas.key(&mut app, Key::Escape);
    assert!(!app.annotate.measure.is_active_draft());
    assert!(app.annotate.measure.saved.contains(0, &original));
    assert_no_edit_or_preview(&requests);
}

#[test]
fn narrow_measurement_panel_scrolls_without_overflow_and_keeps_busy_cancel_reachable() {
    for language in ["en-US", "fr-FR"] {
        for dark in [false, true] {
            let (mut app, requests, _) = crate::app::tests::test_app();
            app.settings.language = language.into();
            app.settings.dark_theme = dark;
            let mut canvas = Canvas::new(&mut app);
            canvas.screen_size = vec2(1000.0, 360.0);
            app.annotate.measure.kind = MeasurementKind::Angle;
            for _ in 0..3 {
                canvas.frame(&mut app, Vec::new());
            }
            assert!(canvas.panel_rect.width() <= 280.0);
            assert!(
                canvas.panel_content_size.x <= canvas.panel_rect.width() + 1.0,
                "long translated type labels must not widen the panel: {language}, dark={dark}"
            );
            assert!(canvas.panel_content_size.y > canvas.panel_rect.height());
            canvas.rectangle_rotation(&mut app, "-22.5");
            let output = canvas.frame(&mut app, Vec::new());
            assert!(
                text_position(&output.shapes, tr(app.lang(), "measure.rectangle.rotation"))
                    .is_some()
            );
            assert!(canvas.panel_content_size.x <= canvas.panel_rect.width() + 1.0);
            assert_no_edit_or_preview(&requests);
            canvas.control(&mut app, "measure.calibrate");
            assert!(app.annotate.measure.is_active_draft());
            app.saving = true;
            canvas.control(&mut app, "measure.cancel_draft");
            assert!(
                canvas.panel_scroll > 0.0,
                "the button was reached by scrolling"
            );
            assert!(!app.annotate.measure.is_active_draft());
            assert!(!app.dirty);
            assert_no_edit_or_preview(&requests);
            app.saving = false;
            canvas.control(&mut app, "measure.saved.inspect");
            assert!(app.annotate.measure.saved.dialog_open());
            canvas.key(&mut app, Key::Escape);
            assert!(!app.annotate.measure.saved.dialog_open());
            assert_eq!(app.annotate.measure.kind, MeasurementKind::Angle);
            assert_eq!(app.annotate.measure.unit, MeasurementUnit::Metres);
            assert_eq!(app.annotate.measure.precision, 2);
            assert_no_edit_or_preview(&requests);
        }
    }
}

#[test]
fn canvas_exposes_regions_and_batch_preview_without_mutating_before_confirmation() {
    for language in ["en-US", "fr-FR"] {
        let (mut app, requests, _) = crate::app::tests::test_app();
        app.settings.language = language.into();
        let mut canvas = Canvas::new(&mut app);
        let (page, epoch, generation) = requests
            .try_iter()
            .find_map(|r| match r {
                Request::MeasurementPage {
                    page,
                    epoch,
                    generation,
                } => Some((page, epoch, generation)),
                _ => None,
            })
            .unwrap();
        app.on_measurement_page(
            page,
            epoch,
            generation,
            Ok(crate::measurement_saved::MeasurementPageData::new(
                Ok(None),
                Ok(vec![]),
                true,
            )),
        );
        canvas.calibrate(&mut app, "10");
        canvas.control(&mut app, "measure.viewport.create");
        assert!(app.annotate.measure.scales_ui.region.is_some());
        canvas.point(&mut app, (30.0, 680.0), false);
        canvas.point(&mut app, (180.0, 610.0), false);
        assert!(
            app.annotate.adv.rich.is_none(),
            "region corners are not measurements"
        );
        assert_no_edit_or_preview(&requests);
        canvas.control(&mut app, "measure.cancel_draft");
        assert!(!app.annotate.measure.is_active_draft());
        canvas.control(&mut app, "measure.batch.open");
        assert!(app.annotate.measure.scales_ui.dialog_open());
        canvas.control(&mut app, "measure.batch.preview");
        let batch = requests
            .try_iter()
            .find_map(|r| match r {
                Request::PreviewRecalibration { requests, .. } => Some(requests),
                Request::Edit(_) => panic!("preview must not mutate"),
                _ => None,
            })
            .expect("batch preview from visible controls");
        assert_eq!(batch.len(), 1);
        assert!(batch[0].measurements.is_empty());
        canvas.control(&mut app, "register.close");
        assert!(!app.annotate.measure.scales_ui.dialog_open());
        assert_no_edit_or_preview(&requests);
    }
}
