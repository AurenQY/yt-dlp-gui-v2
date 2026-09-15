use super::*;
use crate::app::state::{MusicPlayerAuraPulse, MusicPlayerAuraTrackField};
use eframe::egui_wgpu::CallbackTrait;

#[test]
fn uniform_contract_has_twenty_one_aligned_vectors() {
    assert_eq!(UNIFORM_BYTES, 336);
    assert!(FRAGMENT.contains("u_primary_pulse_4"));
    assert!(FRAGMENT.contains("u_secondary_pulse_4"));
    assert!(FRAGMENT.contains("boundary_wavefront"));
    assert!(FRAGMENT.contains("ambient_fluid_field"));
}

#[test]
#[ignore = "Run after explicit GPU pixel tests on all three APIs"]
fn generated_api_pixels_agree_within_rounding_tolerance() {
    let baseline = std::path::Path::new("target/aura-pixels-dx12");
    let files = std::fs::read_dir(baseline)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(files.len(), 32);
    for api in ["vulkan", "opengl"] {
        let mut max_error = 0;
        let mut total_error = 0u64;
        let mut count = 0u64;
        for file in &files {
            let a = image::open(file.path()).unwrap().to_rgba8();
            let b = image::open(
                std::path::Path::new("target")
                    .join(format!("aura-pixels-{api}"))
                    .join(file.file_name()),
            )
            .unwrap()
            .to_rgba8();
            assert_eq!(a.dimensions(), b.dimensions());
            for (a, b) in a.as_raw().iter().zip(b.as_raw()) {
                let error = a.abs_diff(*b);
                max_error = max_error.max(error);
                total_error += error as u64;
                count += 1;
            }
        }
        let mean = total_error as f64 / count as f64;
        println!("DX12 vs {api}: max byte error={max_error}; mean={mean:.6}");
        assert!(max_error <= 2 && mean < 0.05, "cross-API visual mismatch");
    }
}

fn sample(phase: f32) -> MusicPlayerAuraDisplay {
    MusicPlayerAuraDisplay {
        animating: true,
        primary_item_id: Some(1),
        primary: Some(MusicPlayerAuraTrackField {
            bar_phase: phase,
            beat_phase: phase,
            energy: 0.85,
            downbeat_strength: 0.8,
            energy_momentum: 0.2,
            boundary: 0.3,
            novelty: 0.7,
            recurrence: 0.5,
            chorusness: 0.8,
            chroma_hue: phase,
            chroma_coherence: 0.8,
            spectrum_bands: [0.8, 0.6, 0.7, 0.9, 0.4, 0.7, 0.6, 0.8],
            pulses: std::array::from_fn(|i| MusicPlayerAuraPulse {
                origin: i as f32 * 0.23,
                age: phase,
                strength: 0.9,
                air: 0.3,
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn render_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    draw: &AuraDraw,
    size: [u32; 2],
    ppp: f32,
    format: wgpu::TextureFormat,
) -> Vec<u8> {
    let extent = wgpu::Extent3d {
        width: size[0],
        height: size[1],
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Aura test"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let stride = (size[0] * 4).div_ceil(256) * 256;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Aura pixels"),
        size: (stride * size[1]) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    let mut resources = egui_wgpu::CallbackResources::default();
    let screen = egui_wgpu::ScreenDescriptor {
        size_in_pixels: size,
        pixels_per_point: ppp,
    };
    draw.prepare(device, queue, &screen, &mut encoder, &mut resources);
    let info = egui::PaintCallbackInfo {
        viewport: draw.rect,
        clip_rect: draw.rect,
        pixels_per_point: ppp,
        screen_size_px: size,
    };
    let pixels = info.viewport_in_pixels();
    {
        let attachments = [Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })];
        let mut pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &attachments,
                ..Default::default()
            })
            .forget_lifetime();
        pass.set_viewport(
            pixels.left_px as f32,
            pixels.top_px as f32,
            pixels.width_px as f32,
            pixels.height_px as f32,
            0.0,
            1.0,
        );
        pass.set_scissor_rect(
            pixels.left_px as u32,
            pixels.top_px as u32,
            pixels.width_px as u32,
            pixels.height_px as u32,
        );
        draw.paint(info, &mut pass, &resources);
    }
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(size[1]),
            },
        },
        extent,
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(15)),
        })
        .unwrap();
    rx.recv_timeout(std::time::Duration::from_secs(15))
        .unwrap()
        .unwrap();
    let mapped = readback.slice(..).get_mapped_range();
    let mut rgba = Vec::new();
    for row in mapped.chunks(stride as usize) {
        rgba.extend_from_slice(&row[..size[0] as usize * 4]);
    }
    drop(mapped);
    readback.unmap();
    rgba
}

#[test]
#[ignore = "Requires a GPU; set AURA_TEST_API=dx12 or vulkan or opengl and run explicitly"]
fn aura_gpu_pixels_are_visible_animated_and_clipped() {
    let api = std::env::var("AURA_TEST_API").expect("explicit test API required");
    let backend = match api.as_str() {
        "dx12" => wgpu::Backends::DX12,
        "vulkan" => wgpu::Backends::VULKAN,
        "opengl" => wgpu::Backends::GL,
        _ => panic!("unknown test API"),
    };
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: backend,
        flags: Default::default(),
        backend_options: Default::default(),
        memory_budget_thresholds: Default::default(),
        display: None,
    });
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .unwrap();
    println!("Aura GPU test adapter: {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let output = std::path::PathBuf::from("target").join(format!("aura-pixels-{api}"));
    std::fs::create_dir_all(&output).unwrap();
    for (size, ppp) in [([384, 192], 1.0), ([192, 128], 1.75)] {
        for format in [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        ] {
            let renderer = Arc::new(WgpuAuraRenderer::new(&device, format).unwrap());
            for dark in [false, true] {
                for layer in [MusicAuraLayer::Player, MusicAuraLayer::WindowAmbient] {
                    let rect = egui::Rect::from_min_max(
                        egui::pos2(8.0, 8.0),
                        egui::pos2(size[0] as f32 / ppp - 8.0, size[1] as f32 / ppp - 8.0),
                    );
                    let mut frames = Vec::new();
                    for (index, phase) in [0.15, 0.58].into_iter().enumerate() {
                        let draw =
                            renderer.draw(rect, 12.0, [0.2, 0.5, 0.9], dark, sample(phase), layer);
                        assert_eq!(
                            draw.uniform_bytes(egui::PaintCallbackInfo {
                                viewport: rect,
                                clip_rect: rect,
                                pixels_per_point: ppp,
                                screen_size_px: size
                            })
                            .len(),
                            UNIFORM_BYTES as usize
                        );
                        let rgba = render_pixels(&device, &queue, &draw, size, ppp, format);
                        assert_eq!(&rgba[..4], &[0, 0, 0, 0], "must not paint outside viewport");
                        assert!(
                            rgba.chunks_exact(4).filter(|p| p[3] > 0).count() > 100,
                            "blank Aura"
                        );
                        image::save_buffer(
                            output.join(format!(
                                "{}-{format:?}-{dark}-{layer:?}-{index}.png",
                                size[0]
                            )),
                            &rgba,
                            size[0],
                            size[1],
                            image::ColorType::Rgba8,
                        )
                        .unwrap();
                        frames.push(rgba);
                    }
                    assert!(
                        frames[0]
                            .iter()
                            .zip(&frames[1])
                            .filter(|(a, b)| a != b)
                            .count()
                            > 100,
                        "Aura must respond to changing audio/animation inputs"
                    );
                }
            }
        }
    }
}
