//! Explicit native DX12 gate. WARP evidence does not qualify a physical GPU or a window surface.
#![cfg(windows)]

use eframe::egui_wgpu::{self, wgpu};
use egui::{pos2, vec2, Color32, Rect};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;
use sunipdf_core::{tiles, Engine, PixelRect, TileLimits};

const WIDTH: u32 = 512;
const HEIGHT: u32 = 384;
type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

fn require(condition: bool, message: &str) -> TestResult<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn pixel(bytes: &[u8], x: u32, y: u32) -> &[u8] {
    let start = ((y * WIDTH + x) * 4) as usize;
    &bytes[start..start + 4]
}

fn sha256(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 15)]));
    }
    encoded
}

fn smoke(root: &Path, output: &Path) -> TestResult<Value> {
    let requested = std::env::var("SUNIPDF_GPU_ADAPTER").unwrap_or_else(|_| "warp".into());
    require(
        matches!(requested.as_str(), "warp" | "physical"),
        "SUNIPDF_GPU_ADAPTER must be warp or physical",
    )?;
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::DX12;
    let instance = wgpu::Instance::new(descriptor);
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        force_fallback_adapter: requested == "warp",
        compatible_surface: None,
        power_preference: wgpu::PowerPreference::LowPower,
        apply_limit_buckets: false,
    }))
    .map_err(|error| format!("Requested DX12 adapter unavailable: {error}"))?;
    let info = adapter.get_info();
    require(
        info.backend == wgpu::Backend::Dx12,
        "The requested native DX12 backend was not selected",
    )?;
    if requested == "physical" {
        require(
            matches!(
                info.device_type,
                wgpu::DeviceType::IntegratedGpu | wgpu::DeviceType::DiscreteGpu
            ),
            "A physical adapter was requested, but only software/virtual hardware was selected",
        )?;
    } else {
        require(
            info.device_type == wgpu::DeviceType::Cpu,
            "WARP was requested, but a software adapter was not selected",
        )?;
    }
    let adapter_limits = adapter.limits();
    let limits = wgpu::Limits::downlevel_defaults();
    let requested_side = limits.max_texture_dimension_2d;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("SuniPDF native renderer regression"),
        required_limits: limits,
        ..Default::default()
    }))?;
    let device_limits = device.limits();
    let side = device_limits.max_texture_dimension_2d;
    require(
        side == requested_side && side <= adapter_limits.max_texture_dimension_2d,
        "Device texture limits differ from the requested bounded limits",
    )?;

    // Validation rejects a descriptor beyond the device limit before backing memory is allocated.
    let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let invalid_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Intentional over-limit descriptor; no backing allocation"),
        size: wgpu::Extent3d {
            width: side.checked_add(1).ok_or("Texture limit overflow")?,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let refused = pollster::block_on(validation.pop())
        .ok_or("Native GPU accepted an over-limit texture descriptor")?;
    drop(invalid_texture);
    let tile_limits = TileLimits::for_texture_side(side as usize);
    let plan = tiles::plan_tiles(
        (10_000, 7_000),
        PixelRect::new(0, 0, 2_050, 1_600),
        &tile_limits,
    );
    require(
        plan.len() > 1
            && plan
                .iter()
                .all(|tile| tile.width <= side && tile.height <= side),
        "Large PDF page planning exceeded the real device texture limit",
    )?;
    require(
        tiles::page_pixels(
            sunipdf_core::PageSize {
                width_pt: 612.0,
                height_pt: 792.0,
            },
            f32::INFINITY,
        )
        .is_none(),
        "Non-finite PDF zoom was not refused",
    )?;

    let engine = Engine::new()?;
    let document = engine.open(
        &root.join("crates/sunipdf-core/tests/fixtures/two-pages.pdf"),
        None,
    )?;
    let page = document.render_page(0, 0.25)?;
    require(
        page.width < 180 && page.height < 250,
        "Unexpected PDF fixture raster size",
    )?;
    require(
        page.rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[..3].iter().any(|channel| *channel < 128)),
        "PDF fixture has no raster ink",
    )?;
    let context = egui::Context::default();
    let pdf_texture = context.load_texture(
        "Native PDF page",
        egui::ColorImage::from_rgba_unmultiplied(
            [page.width as usize, page.height as usize],
            &page.rgba,
        ),
        egui::TextureOptions::NEAREST,
    );
    let mut colors = vec![0_u8; 32 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            colors[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4].copy_from_slice(if x < 16 {
                &[0, 255, 0, 255]
            } else {
                &[255, 0, 0, 255]
            });
        }
    }
    let image_texture = context.load_texture(
        "Native image",
        egui::ColorImage::from_rgba_unmultiplied([32, 32], &colors),
        egui::TextureOptions::NEAREST,
    );
    let frame = context.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(WIDTH as f32, HEIGHT as f32),
            )),
            max_texture_side: Some(side as usize),
            ..Default::default()
        },
        |ui| {
            let painter = ui.painter();
            painter.text(
                pos2(20.0, 20.0),
                egui::Align2::LEFT_TOP,
                "SuniPDF native text: Cafe\u{e9} \u{3a9}",
                egui::FontId::proportional(20.0),
                Color32::BLACK,
            );
            painter.image(
                pdf_texture.id(),
                Rect::from_min_size(
                    pos2(20.0, 80.0),
                    vec2(page.width as f32, page.height as f32),
                ),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            painter.image(
                image_texture.id(),
                Rect::from_min_size(pos2(220.0, 80.0), vec2(96.0, 96.0)),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        },
    );
    require(
        context.input(|input| input.max_texture_side) == side as usize,
        "egui did not retain the device texture limit",
    )?;
    let jobs = context.tessellate(frame.shapes, frame.pixels_per_point);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer =
        egui_wgpu::Renderer::new(&device, format, egui_wgpu::RendererOptions::PREDICTABLE);
    for (id, deltas) in &frame.textures_delta.set {
        for delta in deltas {
            renderer.update_texture(&device, &queue, *id, delta);
        }
    }
    let screen = egui_wgpu::ScreenDescriptor {
        size_in_pixels: [WIDTH, HEIGHT],
        pixels_per_point: frame.pixels_per_point,
    };
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Native egui readback"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    // WIDTH * 4 is 256-byte aligned, so no row padding needs to be removed.
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Native pixels"),
        size: (WIDTH * HEIGHT * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let render_errors = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut encoder = device.create_command_encoder(&Default::default());
    let callbacks = renderer.update_buffers(&device, &queue, &mut encoder, &jobs, &screen);
    {
        let attachments = [Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                store: wgpu::StoreOp::Store,
            },
        })];
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Actual egui_wgpu renderer"),
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
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(WIDTH * 4),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(
        callbacks
            .into_iter()
            .chain(std::iter::once(encoder.finish())),
    );
    let (sender, receiver) = std::sync::mpsc::channel();
    readback
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
        pollster::block_on(render_errors.pop()).is_none(),
        "Native renderer produced a validation error",
    )?;
    let pixels = readback.slice(..).get_mapped_range()?.to_vec();
    readback.unmap();
    for id in frame.textures_delta.free {
        renderer.free_texture(&id);
    }
    require(
        pixel(&pixels, 236, 96) == [0, 255, 0, 255] && pixel(&pixels, 300, 96) == [255, 0, 0, 255],
        "Uploaded image colors differ in native pixel readback",
    )?;
    let text_ink = (15..55)
        .flat_map(|y| (15..400).map(move |x| (x, y)))
        .filter(|(x, y)| {
            pixel(&pixels, *x, *y)[..3]
                .iter()
                .any(|channel| *channel < 128)
        })
        .count();
    require(
        text_ink > 100,
        "egui text was missing from native pixel readback",
    )?;
    let mut matches = 0_usize;
    let mut compared = 0_usize;
    let mut source_ink = 0_usize;
    let mut matching_ink = 0_usize;
    for (index, expected) in page.rgba.as_chunks::<4>().0.iter().enumerate() {
        let x = index as u32 % page.width;
        let y = index as u32 / page.width;
        let actual = pixel(&pixels, x + 20, y + 80);
        let same = expected
            .iter()
            .zip(actual)
            .all(|(a, b)| a.abs_diff(*b) <= 2);
        matches += usize::from(same);
        if expected[..3].iter().any(|channel| *channel < 200) {
            source_ink += 1;
            matching_ink += usize::from(same);
        }
        compared += 1;
    }
    require(
        matches as f64 / compared as f64 > 0.99,
        "PDF texture differs from its PDFium raster in native readback",
    )?;
    require(
        source_ink > 0 && matching_ink as f64 / source_ink as f64 > 0.90,
        "PDF glyphs are missing from the native texture even if its white background matches",
    )?;
    let png = output.join("native-wgpu.png");
    image::save_buffer(&png, &pixels, WIDTH, HEIGHT, image::ColorType::Rgba8)?;
    Ok(
        json!({ "schema": 1, "status": "passed", "requestedAdapter": requested,
        "qualification": if info.device_type == wgpu::DeviceType::Cpu { "DX12 software WARP only; physical GPUs, ARM64 hardware and native window presentation remain unverified" }
            else { "One physical DX12 adapter offscreen; native window presentation, other GPUs and ARM64 hardware remain unverified" },
        "adapter": { "name": info.name, "backend": format!("{:?}", info.backend), "deviceType": format!("{:?}", info.device_type),
            "vendor": info.vendor, "device": info.device, "driver": info.driver, "driverInfo": info.driver_info },
        "textureDimension2d": { "adapter": adapter_limits.max_texture_dimension_2d, "requested": requested_side, "device": side },
        "overLimitTextureRefused": refused.to_string(), "largePdfTileCount": plan.len(), "tileSide": tile_limits.tile_side,
        "nonFiniteZoomRefused": true, "outputPixels": [WIDTH, HEIGHT], "rgbaSha256": sha256(&pixels),
        "pngSha256": sha256(&std::fs::read(&png)?),
        "pdfTexture": { "sourcePixels": [page.width, page.height], "matchingPixels": matches, "comparedPixels": compared,
            "channelTolerance": 2, "minimumMatchingFraction": 0.99,
            "sourceInkPixels": source_ink, "matchingInkPixels": matching_ink, "minimumInkMatchingFraction": 0.90 },
        "imageColorProbesPassed": true, "eguiTextInkPixels": text_ink,
        "defaultRendererChanged": false }),
    )
}

#[test]
#[ignore = "Requires native Windows DX12 WARP or explicitly selected physical adapter and PDFium"]
fn native_dx12_pdf_image_text_and_limits() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = root.join("target/native-wgpu-evidence");
    std::fs::create_dir_all(&output).unwrap();
    let result = smoke(&root, &output);
    let report = match &result {
        Ok(value) => value.clone(),
        Err(error) => json!({ "schema": 1, "status": "failed", "error": error.to_string() }),
    };
    std::fs::write(
        output.join("native-wgpu-report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    if let Err(error) = result {
        panic!("Native wgpu gate failed: {error}");
    }
    eprintln!("Native wgpu readback PASS: {}", output.display());
}
