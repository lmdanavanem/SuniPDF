//! Actual application frames rendered offscreen with PDFium tiles and DX12 WARP.

use crate::app::{OpenDocument, Zoom};
use crate::worker::{Request, Response};
use eframe::egui_wgpu::{self, wgpu};
use egui::{pos2, vec2, Rect};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use sunipdf_core::{Command, Document, Engine, TileJob};
use sunipdf_i18n::{tr, Language};

type EvidenceResult<T> = Result<T, Box<dyn std::error::Error>>;

fn require(condition: bool, message: &str) -> EvidenceResult<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn relative_luminance(rgb: [u8; 3]) -> f64 {
    let channel = |value: u8| {
        let value = f64::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(rgb[0]) + 0.7152 * channel(rgb[1]) + 0.0722 * channel(rgb[2])
}

fn readable_text_pixels(
    pixels: &[u8],
    size: [u32; 2],
    rect: Rect,
    foreground: egui::Color32,
    background: egui::Color32,
) -> usize {
    let [width, height] = size;
    let rect = rect.intersect(Rect::from_min_size(
        egui::Pos2::ZERO,
        vec2(width as f32, height as f32),
    ));
    let background = relative_luminance([background.r(), background.g(), background.b()]);
    let foreground = [foreground.r(), foreground.g(), foreground.b()];
    let mut readable = 0;
    for y in rect.top().ceil().max(0.0) as u32..rect.bottom().floor().max(0.0) as u32 {
        for x in rect.left().ceil().max(0.0) as u32..rect.right().floor().max(0.0) as u32 {
            let offset = ((y * width + x) * 4) as usize;
            let rgb = [pixels[offset], pixels[offset + 1], pixels[offset + 2]];
            // Match actual text ink, excluding borders, selection fills and background pixels.
            if rgb
                .iter()
                .zip(foreground)
                .any(|(&actual, expected)| actual.abs_diff(expected) > 64)
            {
                continue;
            }
            let luminance = relative_luminance(rgb);
            let contrast = (luminance.max(background) + 0.05) / (luminance.min(background) + 0.05);
            if contrast >= 4.5 {
                readable += 1;
            }
        }
    }
    readable
}

#[test]
fn text_pixel_evidence_rejects_blank_and_low_contrast_headings() {
    let rect = Rect::from_min_size(egui::Pos2::ZERO, vec2(32.0, 4.0));
    for (foreground, background) in [
        (egui::Color32::BLACK, egui::Color32::WHITE),
        (egui::Color32::WHITE, egui::Color32::BLACK),
    ] {
        let mut pixels = background.to_array().repeat(128);
        assert_eq!(
            readable_text_pixels(&pixels, [32, 4], rect, foreground, background),
            0
        );
        for pixel in pixels.as_chunks_mut::<4>().0.iter_mut().take(20) {
            pixel.copy_from_slice(&foreground.to_array());
        }
        assert_eq!(
            readable_text_pixels(&pixels, [32, 4], rect, foreground, background),
            20
        );
    }
    let pixels = egui::Color32::from_gray(220).to_array().repeat(128);
    assert_eq!(
        readable_text_pixels(
            &pixels,
            [32, 4],
            rect,
            egui::Color32::from_gray(220),
            egui::Color32::WHITE
        ),
        0
    );
}

fn wait_future<T>(stage: &str, future: impl Future<Output = T>) -> EvidenceResult<T> {
    let mut future = std::pin::pin!(future);
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    let started = Instant::now();
    loop {
        if let std::task::Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return Ok(value);
        }
        if started.elapsed() >= Duration::from_secs(30) {
            return Err(format!("Reader UI GPU timed out while {stage}").into());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

struct Scene {
    name: &'static str,
    language: Language,
    dark: bool,
    width: u32,
    height: u32,
    measurements: bool,
    zoom: Zoom,
    table: Option<TableKind>,
}

#[derive(Clone, Copy)]
enum TableKind {
    Register,
    Quantities,
}

/// Respond to the real view's read requests using the fixture document.
fn fulfill_requests(
    document: &Document<'_>,
    requests: &mpsc::Receiver<Request>,
    responses: &mpsc::Sender<Response>,
    tiles: &mut Vec<TileJob>,
) -> EvidenceResult<()> {
    for (index, request) in requests.try_iter().enumerate() {
        require(index < 128, "Reader UI exceeded its fixture request bound")?;
        let response = match request {
            Request::SetWanted { epoch, jobs } => {
                require(
                    jobs.len() <= 32,
                    "Reader UI exceeded its tile request bound",
                )?;
                for job in jobs {
                    require(
                        job.region.width <= 4096 && job.region.height <= 4096,
                        "Reader UI requested an oversized fixture tile",
                    )?;
                    let image = document.render_region(job.page, job.scale(), job.region)?;
                    responses.send(Response::Tile {
                        epoch,
                        job,
                        image: Arc::new(image),
                    })?;
                    if !tiles.contains(&job) {
                        tiles.push(job);
                    }
                }
                continue;
            }
            Request::Thumbnail { page, width_px } => {
                require(
                    width_px <= 1024,
                    "Reader UI requested an oversized thumbnail",
                )?;
                let size = document.page_size(page)?;
                Response::Thumbnail {
                    page,
                    image: Arc::new(document.render_page(page, width_px as f32 / size.width_pt)?),
                }
            }
            Request::MeasurementPage {
                page,
                epoch,
                generation,
            } => {
                let mut data = crate::measurement_saved::MeasurementPageData::new(
                    document.page_calibration(page).map_err(|e| e.to_string()),
                    document
                        .measurement_records(page)
                        .map_err(|e| e.to_string()),
                    document.properties().can_modify,
                );
                data.can_annotate = document.properties().can_annotate;
                data.viewports = document
                    .measurement_viewports(page)
                    .map(Arc::from)
                    .map_err(|e| e.to_string());
                data.snap_geometry = document
                    .vector_snap_geometry(page)
                    .map(Arc::new)
                    .map_err(|e| e.to_string());
                Response::MeasurementPage {
                    page,
                    epoch,
                    generation,
                    result: Ok(data),
                }
            }
            Request::Annotations { page, detailed } => Response::Annotations {
                page,
                list: document.annotations(page)?,
                details: if detailed {
                    Some(document.annotation_details(page)?)
                } else {
                    None
                },
            },
            Request::Links { page } => Response::Links {
                page,
                list: document.page_links(page)?,
            },
            Request::FormInfo => Response::FormInfo(document.form_info()),
            Request::Bookmarks => Response::Bookmarks(document.bookmarks()),
            Request::Properties => Response::Properties(document.properties()),
            Request::AnnotationRegister { epoch, generation } => Response::AnnotationRegister {
                epoch,
                generation,
                result: document.annotation_register(),
            },
            Request::QuantityTable { epoch, generation } => Response::QuantityTable {
                epoch,
                generation,
                result: document.quantity_table(),
            },
            Request::FinishMutation | Request::SetRecovery(_) => continue,
            _ => {
                return Err(
                    "Reader UI requested an unexpected operation during read-only evidence".into(),
                );
            }
        };
        responses.send(response)?;
    }
    Ok(())
}

fn readback(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &mut egui_wgpu::Renderer,
    context: &egui::Context,
    frame: egui::FullOutput,
    size: [u32; 2],
) -> EvidenceResult<Vec<u8>> {
    let [width, height] = size;
    require(
        width <= 1600 && height <= 1000,
        "Reader UI screenshot exceeds its resolution bound",
    )?;
    let jobs = context.tessellate(frame.shapes, frame.pixels_per_point);
    let screen = egui_wgpu::ScreenDescriptor {
        size_in_pixels: size,
        pixels_per_point: frame.pixels_per_point,
    };
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("SuniPDF application UI evidence"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    let padded_row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("SuniPDF bounded UI pixels"),
        size: u64::from(padded_row) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut encoder = device.create_command_encoder(&Default::default());
    let callbacks = renderer.update_buffers(device, queue, &mut encoder, &jobs, &screen);
    {
        let attachments = [Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })];
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Actual SuniApp UI through egui_wgpu"),
            color_attachments: &attachments,
            ..Default::default()
        });
        renderer.render(&mut pass.forget_lifetime(), &jobs, &screen);
    }
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(
        callbacks
            .into_iter()
            .chain(std::iter::once(encoder.finish())),
    );
    let (sender, receiver) = mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(Duration::from_secs(30)),
    })?;
    receiver.recv_timeout(Duration::from_secs(30))??;
    require(
        wait_future("validating application pixels", validation.pop())?.is_none(),
        "Reader UI GPU validation failed",
    )?;
    let mapped = buffer.slice(..).get_mapped_range()?;
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for row in mapped.chunks_exact(padded_row as usize) {
        pixels.extend_from_slice(&row[..(width * 4) as usize]);
    }
    drop(mapped);
    buffer.unmap();
    Ok(pixels)
}

fn capture_scene(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    document: &Document<'_>,
    scene: Scene,
    output: &Path,
) -> EvidenceResult<Value> {
    eprintln!("Reader UI scene: {}", scene.name);
    let context = egui::Context::default();
    context.enable_accesskit();
    let (mut app, requests, responses) = crate::app::tests::test_app();
    app.has_icons = crate::icons::install(&context);
    crate::theme::apply(&context, scene.dark);
    app.settings.dark_theme = scene.dark;
    app.settings.set_language(scene.language);
    app.settings.autosave = false;
    app.settings.show_thumbnails = !scene.measurements && scene.table.is_none();
    app.show_tools = scene.measurements;
    app.settings.show_tools = scene.measurements;
    let mut info = document.info().clone();
    info.path = PathBuf::from("fixtures/two-pages.pdf");
    app.document = Some(OpenDocument { info, page: 0 });
    app.tabs.tabs[0].path = PathBuf::from("fixtures/two-pages.pdf");
    app.zoom = scene.zoom;
    // The mixed-orientation fixture centers its portrait page within a wider
    // continuous layout. Navigate the narrow zoomed scene to known page content
    // so the viewport includes the blue rectangle at (72, 142) from the top left.
    let fixture_destination = (scene.width <= 720
        && matches!(scene.zoom, Zoom::Fixed(zoom) if (zoom - 1.49).abs() < 0.001))
    .then_some((48.0, 96.0));
    if let Some((x_pt, y_from_top_pt)) = fixture_destination {
        app.view.request(crate::view::ScrollRequest::Destination {
            page: 0,
            x_pt,
            y_from_top_pt,
        });
    }
    if scene.measurements {
        app.execute(&context, Command::HubMeasure);
    }
    if let Some(table) = scene.table {
        app.settings.tables.placement = crate::workspace_table::Placement::Bottom;
        app.execute(
            &context,
            match table {
                TableKind::Register => Command::AnnotationRegister,
                TableKind::Quantities => Command::QuantityTable,
            },
        );
    }
    let mut renderer = egui_wgpu::Renderer::new(
        device,
        wgpu::TextureFormat::Rgba8Unorm,
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut tiles = Vec::new();
    let mut final_frame = None;
    let mut pending_texture_frees = Vec::new();
    let mut frame_states = Vec::with_capacity(12);
    for index in 0..12 {
        // Earlier frames are not painted; their deferred frees are safe before
        // building the next frame. Keep the final frame's textures until readback.
        for id in pending_texture_frees.drain(..) {
            renderer.free_texture(&id);
        }
        let mut frame = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(scene.width as f32, scene.height as f32),
                )),
                max_texture_side: Some(device.limits().max_texture_dimension_2d as usize),
                time: Some(index as f64 * 0.25),
                ..Default::default()
            },
            |ui| app.render_ui(ui),
        );
        for (id, deltas) in frame.textures_delta.set.drain() {
            for delta in deltas {
                renderer.update_texture(device, queue, id, &delta);
            }
        }
        pending_texture_frees.extend(frame.textures_delta.free.drain());
        fulfill_requests(document, &requests, &responses, &mut tiles)?;
        frame_states.push(json!({
            "frame": index, "rendering": app.view.is_rendering(), "epoch": app.view.epoch,
            "effectiveZoom": app.effective_zoom, "tableHeight": app.settings.tables.height,
        }));
        final_frame = Some(frame);
    }
    let frame = final_frame.ok_or("Missing application UI frame")?;
    // Persist the actual failed scene too: structural checks below must not hide the frame
    // needed to distinguish a product layout regression from an evidence expectation error.
    let pixel_result = readback(
        device,
        queue,
        &mut renderer,
        &context,
        frame.clone(),
        [scene.width, scene.height],
    );
    for id in pending_texture_frees {
        renderer.free_texture(&id);
    }
    let pixels = pixel_result?;
    let png_name = format!("{}.png", scene.name);
    let png = output.join(&png_name);
    image::save_buffer(
        &png,
        &pixels,
        scene.width,
        scene.height,
        image::ColorType::Rgba8,
    )?;
    let tree = frame
        .platform_output
        .accesskit_update
        .as_ref()
        .ok_or("Missing application accessibility tree")?;
    require(
        tree.nodes.len() <= 10_000,
        "Reader UI accessibility tree exceeds its evidence bound",
    )?;
    let app_error = app
        .error
        .map(|error| error.chars().take(512).collect::<String>());
    std::fs::write(
        output.join(format!("{}-accessibility.json", scene.name)),
        serde_json::to_vec_pretty(&json!({
            "scene": scene.name,
            "settleState": {
                "rendering": app.view.is_rendering(), "appError": app_error,
                "documentOpen": app.document.is_some(), "unsavedChanges": app.has_unsaved_changes(),
                "tileRequests": tiles.len(), "frames": frame_states,
            },
            "nodes": tree.nodes.iter().map(|(_, node)| json!({
                "role": format!("{:?}", node.role()), "label": node.label(), "value": node.value(),
                "bounds": node.bounds().map(|rect| [rect.x0, rect.y0, rect.x1, rect.y1]),
            })).collect::<Vec<_>>(),
            "paintedText": frame.shapes.iter().filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(json!({
                    "text": text.galley.text(),
                    "bounds": [text.pos.x, text.pos.y, text.pos.x + text.galley.size().x, text.pos.y + text.galley.size().y],
                    "clip": [shape.clip_rect.min.x, shape.clip_rect.min.y, shape.clip_rect.max.x, shape.clip_rect.max.y],
                })),
                _ => None,
            }).collect::<Vec<_>>()
        }))?,
    )?;
    require(
        !app.view.is_rendering() && app.error.is_none(),
        &format!(
            "{}: reader fixture did not settle; rendering={}, document_open={}, app_error={app_error:?}; see scene PNG and settleState",
            scene.name,
            app.view.is_rendering(),
            app.document.is_some()
        ),
    )?;
    require(
        !app.has_unsaved_changes(),
        "Read-only UI evidence unexpectedly changed the document",
    )?;
    require(
        !tiles.is_empty(),
        "Actual document view did not request PDF tiles",
    )?;
    let viewport = Rect::from_min_size(
        pos2(0.0, 0.0),
        vec2(scene.width as f32, scene.height as f32),
    );
    for key in ["reading.tools", "reading.zoom"] {
        let label = tr(scene.language, key);
        require(
            tree.nodes
                .iter()
                .any(|(_, node)| node.label() == Some(label)),
            "Expected reader control is absent from the real UI",
        )?;
    }
    let table_name = scene.table.map(|table| match table {
        TableKind::Register => "annotations",
        TableKind::Quantities => "quantities",
    });
    let mut table_text_contrast = Vec::new();
    if let Some(table) = scene.table {
        let (title, close) = match table {
            TableKind::Register => ("register.title", "register.close"),
            TableKind::Quantities => ("quantity.title", "quantity.close"),
        };
        let title = tr(scene.language, title);
        // egui exposes static Role::Label text through value(), while actionable controls use
        // label(). Verify both accessibility and visible paint for the non-interactive title.
        require(
            tree.nodes.iter().any(|(_, node)| node.value() == Some(title) || node.label() == Some(title))
                && frame.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text)
                    if text.galley.text() == title && Rect::from_min_size(text.pos, text.galley.size())
                        .intersect(shape.clip_rect).intersect(viewport).is_positive())),
            &format!("{}: docked table title {title:?} is not accessible and visible", scene.name),
        )?;
        let palette = crate::theme::palette(scene.dark);
        for (part, label) in [
            ("title", title),
            ("firstColumnHeader", tr(scene.language, "register.page")),
        ] {
            let rect = frame
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == label => Some(
                        Rect::from_min_size(text.pos, text.galley.size())
                            .intersect(shape.clip_rect)
                            .intersect(viewport),
                    ),
                    _ => None,
                })
                .ok_or_else(|| {
                    format!("{}: missing docked table {part} paint bounds", scene.name)
                })?;
            let ink = readable_text_pixels(
                &pixels,
                [scene.width, scene.height],
                rect,
                palette.text,
                palette.surface,
            );
            require(
                ink >= 20,
                &format!(
                    "{}: docked table {part} ({label:?}) has only {ink} readable text pixels; need20 at contrast>=4.5:1 against its surface",
                    scene.name
                ),
            )?;
            table_text_contrast
                .push(json!({"part": part, "readablePixels": ink, "minimumContrast": 4.5}));
        }
        for key in ["workspace.table.dock", "workspace.table.float", close] {
            let label = tr(scene.language, key);
            require(
                tree.nodes
                    .iter()
                    .any(|(_, node)| node.label() == Some(label)),
                &format!(
                    "{}: docked table control {key} ({label:?}) is missing; see the scene accessibility JSON",
                    scene.name
                ),
            )?;
        }
        require(
            frame.shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::Shape::Text(text)
                if matches!(text.galley.text(), "Fixture equipment" | "Count: Fixture equipment")
                && Rect::from_min_size(text.pos, text.galley.size()).intersect(shape.clip_rect)
                    .intersect(viewport).is_positive())
            }),
            &format!(
                "{}: the docked table does not show its actual fixture count row; see scene PNG and accessibility/paint bounds",
                scene.name
            ),
        )?;
        require(
            !app.dialog_open(),
            "Docked table incorrectly blocks document interactions",
        )?;
    }
    let tile_ids: Vec<_> = tiles
        .iter()
        .filter_map(|job| app.view.pool.exact(job).map(|handle| handle.id()))
        .collect();
    let regions: Vec<_> = frame
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Mesh(mesh) if tile_ids.contains(&mesh.texture_id) => {
                let bounds = mesh
                    .calc_bounds()
                    .intersect(shape.clip_rect)
                    .intersect(viewport);
                bounds.is_positive().then_some(bounds)
            }
            _ => None,
        })
        .collect();
    require(
        !regions.is_empty(),
        "Real PDF tiles were not painted in the document canvas",
    )?;
    let visible = regions
        .iter()
        .copied()
        .reduce(Rect::union)
        .ok_or("Missing visible PDF canvas")?;
    require(
        visible.width() >= 100.0 && visible.height() >= 120.0,
        "Reader panels leave too little visible PDF canvas",
    )?;
    if scene.measurements {
        let label = tr(scene.language, "hub.measure.name");
        require(
            tree.nodes.iter().any(|(_, node)| {
                node.label() == Some(label) && node.role() == egui::accesskit::Role::Button
            }),
            "Measurement discovery row is absent",
        )?;
        require(frame.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == tr(scene.language, "tools.group.review"))), "Review tool family is not visible")?;
    }
    if matches!(scene.zoom, Zoom::Fixed(zoom) if (zoom - 1.49).abs() < 0.001) {
        require(
            frame.shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::Shape::Text(text)
            if text.galley.text() == "149%" && text.galley.rows.len() == 1
                && text.pos.x >= 0.0 && text.pos.x + text.galley.size().x <= scene.width as f32)
            }),
            "The 149% zoom label is missing, wrapped or outside the window",
        )?;
    }
    let visible_regions: Vec<_> = regions
        .iter()
        .map(|rect| [rect.min.x, rect.min.y, rect.max.x, rect.max.y])
        .collect();
    let mut blue_ink = 0_u64;
    for y in visible.top().ceil() as u32..visible.bottom().floor() as u32 {
        for x in visible.left().ceil() as u32..visible.right().floor() as u32 {
            if !regions
                .iter()
                .any(|rect| rect.contains(pos2(x as f32 + 0.5, y as f32 + 0.5)))
            {
                continue;
            }
            let offset = ((y * scene.width + x) * 4) as usize;
            let pixel = &pixels[offset..offset + 4];
            if pixel[0] < 80 && pixel[1] < 80 && pixel[2] > 180 {
                blue_ink += 1;
            }
        }
    }
    require(
        blue_ink > 20,
        "PDFium fixture ink is missing from the actual UI pixel readback",
    )?;
    Ok(json!({
        "name": scene.name, "language": scene.language.code(), "darkTheme": scene.dark,
        "outputPixels": [scene.width, scene.height], "measurementToolsOpen": scene.measurements,
        "dockedTable": table_name, "dockedTableControlsAndRowVerified": scene.table.is_some(),
        "dockedTableTextContrast": table_text_contrast,
        "effectiveZoom": app.effective_zoom, "actualApplicationRenderUi": true,
        "fixtureNavigation": fixture_destination.map(|(x_pt, y_from_top_pt)| json!({
            "pageIndex": 0, "coordinateOrigin": "page-top-left", "unit": "PDF points",
            "destination": [x_pt, y_from_top_pt], "method": "ScrollRequest::Destination"
        })),
        "actualPdfiumTiles": tiles.len(), "visiblePdfRegions": visible_regions,
        "pdfFixtureBlueInkPixels": blue_ink, "readerLabelsVerified": true,
        "zoom149SingleLineVerified": matches!(scene.zoom, Zoom::Fixed(zoom) if (zoom - 1.49).abs() < 0.001),
        "png": png_name, "rgbaSha256": sha256(&pixels), "pngSha256": sha256(&std::fs::read(png)?)
    }))
}

fn evidence(root: &Path, output: &Path) -> EvidenceResult<Value> {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::DX12;
    let instance = wgpu::Instance::new(descriptor);
    let adapter = wait_future(
        "requesting DX12 WARP",
        instance.request_adapter(&wgpu::RequestAdapterOptions {
            force_fallback_adapter: true,
            compatible_surface: None,
            power_preference: wgpu::PowerPreference::LowPower,
            apply_limit_buckets: false,
        }),
    )??;
    let info = adapter.get_info();
    require(
        info.backend == wgpu::Backend::Dx12 && info.device_type == wgpu::DeviceType::Cpu,
        "Reader UI evidence requires the DX12 software WARP adapter",
    )?;
    let (device, queue) = wait_future(
        "requesting bounded UI device",
        adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("SuniPDF reader UI evidence"),
            required_limits: wgpu::Limits::downlevel_defaults(),
            ..Default::default()
        }),
    )??;
    let fixture = root.join("crates/sunipdf-core/tests/fixtures/two-pages.pdf");
    let fixture_sha = sha256(&std::fs::read(&fixture)?);
    let engine = Engine::new()?;
    let document = engine.open(&fixture, None)?;
    let scenes = [
        Scene {
            name: "reader-en-dark-measurements",
            language: Language::English,
            dark: true,
            width: 1280,
            height: 780,
            measurements: true,
            zoom: Zoom::FitPage,
            table: None,
        },
        Scene {
            name: "reader-fr-light-720-zoom149",
            language: Language::French,
            dark: false,
            width: 720,
            height: 480,
            measurements: true,
            zoom: Zoom::Fixed(1.49),
            table: None,
        },
        Scene {
            name: "reader-en-light-pages",
            language: Language::English,
            dark: false,
            width: 1280,
            height: 780,
            measurements: false,
            zoom: Zoom::FitPage,
            table: None,
        },
    ];
    let mut results = Vec::new();
    for scene in scenes {
        results.push(capture_scene(&device, &queue, &document, scene, output)?);
    }
    // Prepare actual editable records in memory; never alter the checked-in fixture file.
    let mut table_document = engine.open_bytes(std::fs::read(&fixture)?, None, &fixture)?;
    table_document.add_count_marker(
        0,
        (200.0, 100.0),
        18.0,
        "Fixture equipment",
        Some("Fixture reviewer"),
    )?;
    table_document.add_measurement_annotation(
        0,
        &sunipdf_core::measurement::Measurement {
            kind: sunipdf_core::measurement::MeasurementKind::Length,
            points: vec![(40.0, 80.0), (140.0, 80.0)],
            calibration: [(40.0, 40.0), (140.0, 40.0)],
            known_distance: 10.0,
            unit: sunipdf_core::measurement::MeasurementUnit::Metres,
            precision: 2,
        },
        None,
    )?;
    for scene in [
        Scene {
            name: "reader-en-dark-docked-register",
            language: Language::English,
            dark: true,
            width: 1280,
            height: 780,
            measurements: false,
            zoom: Zoom::FitPage,
            table: Some(TableKind::Register),
        },
        Scene {
            name: "reader-fr-light-720-docked-quantities",
            language: Language::French,
            dark: false,
            width: 720,
            height: 480,
            measurements: false,
            zoom: Zoom::FitPage,
            table: Some(TableKind::Quantities),
        },
    ] {
        results.push(capture_scene(
            &device,
            &queue,
            &table_document,
            scene,
            output,
        )?);
    }
    require(
        sha256(&std::fs::read(&fixture)?) == fixture_sha,
        "UI evidence changed the fixture document",
    )?;
    Ok(json!({
        "schema": 1, "status": "passed", "version": env!("CARGO_PKG_VERSION"),
        "sourceSha": std::env::var("GITHUB_SHA").ok(),
        "qualification": "Actual application UI and PDFium document tiles, rendered offscreen with DX12 WARP. Native window presentation and physical GPU interaction are not covered.",
        "adapter": { "name": info.name, "backend": format!("{:?}", info.backend), "deviceType": format!("{:?}", info.device_type) },
        "fixture": "crates/sunipdf-core/tests/fixtures/two-pages.pdf", "fixtureSha256": fixture_sha,
        "fixtureUnchanged": true, "scenes": results
        , "tableFixture": { "source": "same two-page fixture, prepared only in memory", "countGroup": "Fixture equipment", "count": 1, "lengthMetres": 10 }
    }))
}

#[test]
#[ignore = "Requires Windows DX12 WARP and PDFium; writes actual application screenshots"]
fn reader_ui_dx12_warp_evidence() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = root.join("target/reader-ui-evidence");
    std::fs::create_dir_all(&output).unwrap();
    let (sender, receiver) = mpsc::channel();
    let worker_output = output.clone();
    std::thread::spawn(move || {
        let result = evidence(&root, &worker_output).map_err(|error| error.to_string());
        let _ = sender.send(result);
    });
    let result = receiver
        .recv_timeout(Duration::from_secs(180))
        .unwrap_or_else(|error| Err(format!("Reader UI evidence exceeded 180 seconds: {error}")));
    let report = match &result {
        Ok(value) => value.clone(),
        Err(error) => json!({ "schema": 1, "status": "failed", "error": error }),
    };
    std::fs::write(
        output.join("reader-ui-report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    if let Err(error) = result {
        panic!("Reader UI evidence failed: {error}");
    }
    eprintln!("Reader UI offscreen evidence PASS: 5 actual application scenes");
}
