use crate::app::{SuniApp, Zoom, SCREEN_PER_PDF_POINT};
use crate::worker::{EditOp, Request};
use egui::{vec2, Event, FullOutput, Key, Modifiers, Pos2, Rect};
use std::sync::mpsc::Receiver;
use sunipdf_core::annot_rich::{RichAnnotation, RichAnnotationPreview};
use sunipdf_core::measurement::{Measurement, MeasurementKind};
use sunipdf_core::Command;
use sunipdf_i18n::tr;

struct Canvas {
    ctx: egui::Context,
    time: f64,
}

impl Canvas {
    fn new(app: &mut SuniApp) -> Self {
        let ctx = egui::Context::default();
        crate::icons::install(&ctx);
        app.document.as_mut().unwrap().info.form = sunipdf_core::FormKind::None;
        app.zoom = Zoom::Fixed(0.8);
        app.view.mode = crate::view::ViewMode::Single;
        app.execute(&ctx, Command::ToolMeasure);
        let mut canvas = Self { ctx, time: 1.0 };
        for _ in 0..3 {
            canvas.frame(app, Vec::new());
        }
        canvas
    }

    fn frame(&mut self, app: &mut SuniApp, events: Vec<Event>) -> FullOutput {
        self.time += 0.025;
        let mut output = self.ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 1100.0))),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ui| {
                app.handle_input(&self.ctx);
                egui::Panel::top("measure-integration-controls")
                    .show(ui, |ui| app.measurement_bar(&self.ctx, ui));
                app.document_view(&self.ctx, ui);
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
        self.frame(app, Vec::new());
        let output = self.frame(app, Vec::new());
        let label = tr(app.lang(), key);
        let position = text_position(&output.shapes, label)
            .unwrap_or_else(|| panic!("visible measurement control: {key} ({label})"));
        self.click(app, position, false);
        self.frame(app, Vec::new());
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
    shapes.iter().find_map(|shape| find(&shape.shape, label))
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
        let (mut app, requests, _) = crate::app::tests::test_app();
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
            index: 0,
            annotation_id: Some("scroll-measurement".into()),
            measurement: Measurement {
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
