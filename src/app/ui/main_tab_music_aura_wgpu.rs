//! Wgpu resources and per-draw uniforms; effect math is generated from shared GLSL.
use super::{MusicAuraLayer, MusicPlayerAuraDisplay, PackedAuraTrackField};
use eframe::{egui, egui_wgpu, wgpu};
use std::sync::Arc;

const VERTEX: &str = r#"
struct Output { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn main(@builtin(vertex_index) index: u32) -> Output {
    let positions = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    let p = positions[index];
    return Output(vec4(p, 0.0, 1.0), p * 0.5 + 0.5);
}
"#;
const FRAGMENT: &str = include_str!(concat!(env!("OUT_DIR"), "/aura.wgsl"));
const FRAGMENT_SRGB: &str = include_str!(concat!(env!("OUT_DIR"), "/aura_srgb.wgsl"));
const UNIFORM_BYTES: u64 = 21 * 16;

pub(super) struct WgpuAuraRenderer {
    device: wgpu::Device,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

impl WgpuAuraRenderer {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Result<Self, String> {
        let errors = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let vertex = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Aura vertex"),
            source: wgpu::ShaderSource::Wgsl(VERTEX.into()),
        });
        let fragment = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Aura shared effect"),
            source: wgpu::ShaderSource::Wgsl(
                if format.is_srgb() {
                    FRAGMENT_SRGB
                } else {
                    FRAGMENT
                }
                .into(),
            ),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Aura uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(UNIFORM_BYTES),
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Aura pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Aura"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: Some("main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        if let Some(error) = pollster::block_on(errors.pop()) {
            return Err(format!("Aura pipeline: {error}"));
        }
        Ok(Self {
            device: device.clone(),
            pipeline,
            layout,
        })
    }

    pub(super) fn callback(
        self: &Arc<Self>,
        rect: egui::Rect,
        radius: f32,
        accent: [f32; 3],
        dark: bool,
        display: MusicPlayerAuraDisplay,
        layer: MusicAuraLayer,
    ) -> egui::PaintCallback {
        egui_wgpu::Callback::new_paint_callback(
            rect,
            self.draw(rect, radius, accent, dark, display, layer),
        )
    }

    fn draw(
        self: &Arc<Self>,
        rect: egui::Rect,
        radius: f32,
        accent: [f32; 3],
        dark: bool,
        display: MusicPlayerAuraDisplay,
        layer: MusicAuraLayer,
    ) -> AuraDraw {
        // Each callback owns its buffer, so prepare-all/paint-all cannot alias
        // player, ambient, or multiple viewport data within the same frame.
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Aura draw uniforms"),
            size: UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Aura draw"),
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        AuraDraw {
            renderer: self.clone(),
            buffer,
            bind_group,
            rect,
            radius,
            accent,
            dark,
            display,
            layer,
        }
    }
}

struct AuraDraw {
    renderer: Arc<WgpuAuraRenderer>,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    rect: egui::Rect,
    radius: f32,
    accent: [f32; 3],
    dark: bool,
    display: MusicPlayerAuraDisplay,
    layer: MusicAuraLayer,
}

impl AuraDraw {
    fn uniform_bytes(&self, info: egui::PaintCallbackInfo) -> Vec<u8> {
        let primary = self.display.primary.expect("Aura requires a primary track");
        let secondary = self.display.secondary.unwrap_or(primary);
        let viewport = info.viewport_in_pixels();
        let mut groups = vec![
            [
                viewport.width_px.max(1) as f32,
                viewport.height_px.max(1) as f32,
                (self.radius * info.pixels_per_point).max(1.0),
                0.0,
            ],
            [self.accent[0], self.accent[1], self.accent[2], 0.0],
            [
                if self.display.secondary.is_some() {
                    self.display.mix_progress.clamp(0.0, 1.0)
                } else {
                    0.0
                },
                f32::from(self.display.animating),
                f32::from(self.dark),
                self.layer.uniform_value(),
            ],
        ];
        for track in [primary, secondary] {
            let packed = PackedAuraTrackField::from(track);
            groups.extend([
                packed.a,
                packed.b,
                packed.c,
                packed.spectrum_low,
                packed.spectrum_high,
            ]);
            groups.extend(packed.pulses);
        }
        groups
            .into_iter()
            .flatten()
            .flat_map(f32::to_le_bytes)
            .collect()
    }
}

impl egui_wgpu::CallbackTrait for AuraDraw {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        _resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let info = egui::PaintCallbackInfo {
            viewport: self.rect,
            clip_rect: self.rect,
            pixels_per_point: screen.pixels_per_point,
            screen_size_px: screen.size_in_pixels,
        };
        queue.write_buffer(&self.buffer, 0, &self.uniform_bytes(info));
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        _resources: &egui_wgpu::CallbackResources,
    ) {
        pass.set_pipeline(&self.renderer.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
#[path = "main_tab_music_aura_wgpu_tests.rs"]
mod tests;
