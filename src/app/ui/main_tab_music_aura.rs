use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Ui, mutex::Mutex};
use eframe::egui_glow;
use eframe::glow::{self, HasContext as _};

use crate::app::state::{MusicPlayerAuraDisplay, MusicPlayerAuraTrackField};
use crate::app::widgets::url_input::accent_blue_for_ui;

use super::main_tab_music_aura_dynamics::MusicPlayerAuraDynamics;

#[path = "main_tab_music_aura_wgpu.rs"]
mod wgpu_renderer;

const AURA_TARGET_FRAME_INTERVAL: Duration = Duration::from_micros(33_333);
const WINDOW_MATERIAL_TRANSITION_SECONDS: f32 = 0.72;
const WINDOW_BACKDROP_DIM_MAX_ALPHA: u8 = 88;
const LIGHT_WINDOW_PAPER_TINT: Color32 = Color32::from_rgb(220, 228, 238);
const LIGHT_WINDOW_PAPER_TINT_ALPHA: u8 = 72;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MusicAuraPresentation {
    PlayerOnly,
    WindowAmbient,
    WindowBackground,
}

impl Default for MusicAuraPresentation {
    fn default() -> Self {
        Self::WindowBackground
    }
}

impl MusicAuraPresentation {
    pub(super) fn toggled(self) -> Self {
        match self {
            Self::PlayerOnly => Self::WindowAmbient,
            Self::WindowAmbient => Self::WindowBackground,
            Self::WindowBackground => Self::PlayerOnly,
        }
    }

    pub(super) fn window_layer_enabled(self) -> bool {
        matches!(self, Self::WindowAmbient | Self::WindowBackground)
    }

    pub(super) fn player_layer_enabled(self) -> bool {
        true
    }

    pub(super) fn background_glass_enabled(self) -> bool {
        self == Self::WindowBackground
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MusicAuraLayer {
    Player,
    WindowAmbient,
}

impl MusicAuraLayer {
    fn uniform_value(self) -> f32 {
        match self {
            Self::Player => 0.0,
            Self::WindowAmbient => 1.0,
        }
    }
}

const AURA_VERTEX_SHADER: &str = r#"#version 140
out vec2 v_uv;

const vec2 POSITIONS[3] = vec2[](
    vec2(-1.0, -1.0),
    vec2( 3.0, -1.0),
    vec2(-1.0,  3.0)
);

void main() {
    vec2 position = POSITIONS[gl_VertexID];
    v_uv = position * 0.5 + 0.5;
    gl_Position = vec4(position, 0.0, 1.0);
}
"#;

const AURA_FRAGMENT_SHADER: &str = include_str!("main_tab_music_aura_fragment.glsl");

#[derive(Clone)]
pub(super) struct MusicPlayerAuraRenderer {
    renderer: AuraBackend,
    animation: Arc<Mutex<AuraAnimationState>>,
}

#[derive(Clone)]
enum AuraBackend {
    Glow(Arc<Mutex<MusicPlayerAuraGlowRenderer>>),
    Wgpu(Arc<wgpu_renderer::WgpuAuraRenderer>),
}

impl MusicPlayerAuraRenderer {
    pub(super) fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
        let renderer = if let Some(gl) = &cc.gl {
            AuraBackend::Glow(Arc::new(Mutex::new(MusicPlayerAuraGlowRenderer::new(gl)?)))
        } else if let Some(state) = &cc.wgpu_render_state {
            AuraBackend::Wgpu(Arc::new(wgpu_renderer::WgpuAuraRenderer::new(
                &state.device,
                state.target_format,
            )?))
        } else {
            return Err("No GPU context for Aura".to_owned());
        };
        Ok(Self {
            renderer,
            animation: Arc::new(Mutex::new(AuraAnimationState::new())),
        })
    }

    pub(super) fn destroy(&self, gl: &glow::Context) {
        if let AuraBackend::Glow(renderer) = &self.renderer {
            renderer.lock().destroy(gl);
        }
    }
}

pub(super) fn render_music_player_aura_at(
    ui: &Ui,
    rect: egui::Rect,
    renderer: Option<&MusicPlayerAuraRenderer>,
    display: MusicPlayerAuraDisplay,
    corner_radius: f32,
) {
    render_music_aura_at(
        ui,
        rect,
        renderer,
        display,
        corner_radius,
        MusicAuraLayer::Player,
    );
}

pub(super) fn render_music_window_ambient_at(
    ui: &Ui,
    rect: egui::Rect,
    renderer: Option<&MusicPlayerAuraRenderer>,
    display: MusicPlayerAuraDisplay,
    material_progress: f32,
) {
    render_music_window_backdrop_material(ui, rect, material_progress);
    render_music_aura_at(
        ui,
        rect,
        renderer,
        display,
        1.0,
        MusicAuraLayer::WindowAmbient,
    );
}

pub(super) fn music_window_material_progress(ui: &Ui, enabled: bool, playing: bool) -> f32 {
    if !enabled {
        return 0.0;
    }
    ui.ctx().animate_bool_with_time(
        egui::Id::new("music-window-material-transition"),
        playing,
        WINDOW_MATERIAL_TRANSITION_SECONDS,
    )
}

fn render_music_window_backdrop_material(ui: &Ui, rect: egui::Rect, progress: f32) {
    if ui.visuals().dark_mode {
        let alpha = window_backdrop_dim_alpha(progress);
        if alpha > 0 {
            ui.painter()
                .rect_filled(rect, 0.0, Color32::from_black_alpha(alpha));
        }
    } else {
        let alpha =
            (f32::from(LIGHT_WINDOW_PAPER_TINT_ALPHA) * progress.clamp(0.0, 1.0)).round() as u8;
        if alpha > 0 {
            ui.painter().rect_filled(
                rect,
                0.0,
                Color32::from_rgba_unmultiplied(
                    LIGHT_WINDOW_PAPER_TINT.r(),
                    LIGHT_WINDOW_PAPER_TINT.g(),
                    LIGHT_WINDOW_PAPER_TINT.b(),
                    alpha,
                ),
            );
        }
    }
}

fn window_backdrop_dim_alpha(progress: f32) -> u8 {
    (f32::from(WINDOW_BACKDROP_DIM_MAX_ALPHA) * progress.clamp(0.0, 1.0)).round() as u8
}

fn render_music_aura_at(
    ui: &Ui,
    rect: egui::Rect,
    renderer: Option<&MusicPlayerAuraRenderer>,
    display: MusicPlayerAuraDisplay,
    corner_radius: f32,
    layer: MusicAuraLayer,
) {
    let (Some(renderer), Some(_)) = (renderer, display.primary) else {
        return;
    };
    if rect.width() <= 2.0 || rect.height() <= 2.0 {
        return;
    }

    let accent = color_to_linearish_rgb(accent_blue_for_ui(ui));
    let dark_mode = ui.visuals().dark_mode;
    let display = renderer.animation.lock().resolve(display);
    match &renderer.renderer {
        AuraBackend::Glow(renderer) => {
            let renderer = renderer.clone();
            let callback = egui_glow::CallbackFn::new(move |info, painter| {
                renderer.lock().paint(
                    painter.gl().as_ref(),
                    info,
                    corner_radius,
                    accent,
                    dark_mode,
                    display,
                    layer,
                );
            });
            ui.painter().add(egui::PaintCallback {
                rect,
                callback: Arc::new(callback),
            });
        }
        AuraBackend::Wgpu(renderer) => {
            ui.painter().add(renderer.callback(
                rect,
                corner_radius,
                accent,
                dark_mode,
                display,
                layer,
            ));
        }
    }

    if display.animating {
        ui.ctx().request_repaint_after(AURA_TARGET_FRAME_INTERVAL);
    }
}

fn color_to_linearish_rgb(color: Color32) -> [f32; 3] {
    [
        f32::from(color.r()) / 255.0,
        f32::from(color.g()) / 255.0,
        f32::from(color.b()) / 255.0,
    ]
}

struct MusicPlayerAuraGlowRenderer {
    program: Option<glow::Program>,
    vertex_array: Option<glow::VertexArray>,
    uniforms: MusicPlayerAuraUniforms,
}

struct AuraAnimationState {
    dynamics: MusicPlayerAuraDynamics,
    last_dynamics_update: Instant,
    last_track_identity: (Option<u64>, Option<u64>),
    cached_display: Option<MusicPlayerAuraDisplay>,
}

impl AuraAnimationState {
    fn new() -> Self {
        Self {
            dynamics: MusicPlayerAuraDynamics::new(),
            last_dynamics_update: Instant::now(),
            last_track_identity: (None, None),
            cached_display: None,
        }
    }
    fn resolve(&mut self, display: MusicPlayerAuraDisplay) -> MusicPlayerAuraDisplay {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_dynamics_update);
        let identity = (display.primary_item_id, display.secondary_item_id);
        if aura_dynamics_should_advance(
            elapsed,
            self.cached_display.is_none(),
            identity != self.last_track_identity,
        ) {
            self.cached_display = Some(self.dynamics.advance(display, elapsed.as_secs_f32()));
            self.last_dynamics_update = now;
            self.last_track_identity = identity;
        }
        self.cached_display.unwrap_or(display)
    }
}

impl MusicPlayerAuraGlowRenderer {
    fn new(gl: &glow::Context) -> Result<Self, String> {
        let program = create_aura_program(gl)?;
        let vertex_array = match unsafe { gl.create_vertex_array() } {
            Ok(vertex_array) => vertex_array,
            Err(error) => {
                unsafe {
                    gl.delete_program(program);
                }
                return Err(format!("cannot create aura vertex array: {error}"));
            }
        };
        let uniforms = MusicPlayerAuraUniforms::new(gl, program);
        Ok(Self {
            program: Some(program),
            vertex_array: Some(vertex_array),
            uniforms,
        })
    }

    fn paint(
        &mut self,
        gl: &glow::Context,
        info: egui::PaintCallbackInfo,
        corner_radius: f32,
        accent: [f32; 3],
        dark_mode: bool,
        display: MusicPlayerAuraDisplay,
        layer: MusicAuraLayer,
    ) {
        let (Some(program), Some(vertex_array)) = (self.program, self.vertex_array) else {
            return;
        };
        let Some(primary) = display.primary else {
            return;
        };
        let secondary = display.secondary.unwrap_or(primary);
        let mix_progress = if display.secondary.is_some() {
            display.mix_progress.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let environment = [
            mix_progress,
            f32::from(display.animating),
            f32::from(dark_mode),
            layer.uniform_value(),
        ];
        let viewport = info.viewport_in_pixels();
        let size_radius = [
            viewport.width_px.max(1) as f32,
            viewport.height_px.max(1) as f32,
            (corner_radius * info.pixels_per_point).max(1.0),
        ];
        let primary = PackedAuraTrackField::from(primary);
        let secondary = PackedAuraTrackField::from(secondary);

        unsafe {
            gl.use_program(Some(program));
            gl.bind_vertex_array(Some(vertex_array));
            gl.uniform_3_f32(
                self.uniforms.size_radius.as_ref(),
                size_radius[0],
                size_radius[1],
                size_radius[2],
            );
            gl.uniform_3_f32(
                self.uniforms.accent.as_ref(),
                accent[0],
                accent[1],
                accent[2],
            );
            uniform_vec4(gl, self.uniforms.environment.as_ref(), environment);
            uniform_vec4(gl, self.uniforms.primary_a.as_ref(), primary.a);
            uniform_vec4(gl, self.uniforms.primary_b.as_ref(), primary.b);
            uniform_vec4(gl, self.uniforms.primary_c.as_ref(), primary.c);
            uniform_vec4(
                gl,
                self.uniforms.primary_spectrum_low.as_ref(),
                primary.spectrum_low,
            );
            uniform_vec4(
                gl,
                self.uniforms.primary_spectrum_high.as_ref(),
                primary.spectrum_high,
            );
            uniform_vec4(
                gl,
                self.uniforms.primary_pulses[0].as_ref(),
                primary.pulses[0],
            );
            uniform_vec4(
                gl,
                self.uniforms.primary_pulses[1].as_ref(),
                primary.pulses[1],
            );
            uniform_vec4(
                gl,
                self.uniforms.primary_pulses[2].as_ref(),
                primary.pulses[2],
            );
            uniform_vec4(
                gl,
                self.uniforms.primary_pulses[3].as_ref(),
                primary.pulses[3],
            );
            uniform_vec4(gl, self.uniforms.secondary_a.as_ref(), secondary.a);
            uniform_vec4(gl, self.uniforms.secondary_b.as_ref(), secondary.b);
            uniform_vec4(gl, self.uniforms.secondary_c.as_ref(), secondary.c);
            uniform_vec4(
                gl,
                self.uniforms.secondary_spectrum_low.as_ref(),
                secondary.spectrum_low,
            );
            uniform_vec4(
                gl,
                self.uniforms.secondary_spectrum_high.as_ref(),
                secondary.spectrum_high,
            );
            uniform_vec4(
                gl,
                self.uniforms.secondary_pulses[0].as_ref(),
                secondary.pulses[0],
            );
            uniform_vec4(
                gl,
                self.uniforms.secondary_pulses[1].as_ref(),
                secondary.pulses[1],
            );
            uniform_vec4(
                gl,
                self.uniforms.secondary_pulses[2].as_ref(),
                secondary.pulses[2],
            );
            uniform_vec4(
                gl,
                self.uniforms.secondary_pulses[3].as_ref(),
                secondary.pulses[3],
            );
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            gl.bind_vertex_array(None);
            gl.use_program(None);
        }
    }

    fn destroy(&mut self, gl: &glow::Context) {
        unsafe {
            if let Some(program) = self.program.take() {
                gl.delete_program(program);
            }
            if let Some(vertex_array) = self.vertex_array.take() {
                gl.delete_vertex_array(vertex_array);
            }
        }
    }
}

fn uniform_vec4(gl: &glow::Context, location: Option<&glow::UniformLocation>, values: [f32; 4]) {
    unsafe {
        gl.uniform_4_f32(location, values[0], values[1], values[2], values[3]);
    }
}

fn aura_dynamics_should_advance(
    elapsed: Duration,
    cache_empty: bool,
    track_identity_changed: bool,
) -> bool {
    cache_empty || track_identity_changed || elapsed >= AURA_TARGET_FRAME_INTERVAL
}

struct PackedAuraTrackField {
    a: [f32; 4],
    b: [f32; 4],
    c: [f32; 4],
    spectrum_low: [f32; 4],
    spectrum_high: [f32; 4],
    pulses: [[f32; 4]; 4],
}

impl From<MusicPlayerAuraTrackField> for PackedAuraTrackField {
    fn from(field: MusicPlayerAuraTrackField) -> Self {
        Self {
            a: [
                field.bar_phase,
                field.beat_phase,
                field.downbeat_strength,
                field.energy,
            ],
            b: [
                field.energy_momentum,
                field.boundary,
                field.novelty,
                field.recurrence,
            ],
            c: [
                field.chorusness,
                field.chroma_hue,
                field.chroma_coherence,
                0.0,
            ],
            spectrum_low: [
                field.spectrum_bands[0],
                field.spectrum_bands[1],
                field.spectrum_bands[2],
                field.spectrum_bands[3],
            ],
            spectrum_high: [
                field.spectrum_bands[4],
                field.spectrum_bands[5],
                field.spectrum_bands[6],
                field.spectrum_bands[7],
            ],
            pulses: field
                .pulses
                .map(|pulse| [pulse.origin, pulse.age, pulse.strength, pulse.air]),
        }
    }
}

struct MusicPlayerAuraUniforms {
    size_radius: Option<glow::UniformLocation>,
    accent: Option<glow::UniformLocation>,
    environment: Option<glow::UniformLocation>,
    primary_a: Option<glow::UniformLocation>,
    primary_b: Option<glow::UniformLocation>,
    primary_c: Option<glow::UniformLocation>,
    primary_spectrum_low: Option<glow::UniformLocation>,
    primary_spectrum_high: Option<glow::UniformLocation>,
    primary_pulses: [Option<glow::UniformLocation>; 4],
    secondary_a: Option<glow::UniformLocation>,
    secondary_b: Option<glow::UniformLocation>,
    secondary_c: Option<glow::UniformLocation>,
    secondary_spectrum_low: Option<glow::UniformLocation>,
    secondary_spectrum_high: Option<glow::UniformLocation>,
    secondary_pulses: [Option<glow::UniformLocation>; 4],
}

impl MusicPlayerAuraUniforms {
    fn new(gl: &glow::Context, program: glow::Program) -> Self {
        unsafe {
            Self {
                size_radius: gl.get_uniform_location(program, "u_size_radius"),
                accent: gl.get_uniform_location(program, "u_accent"),
                environment: gl.get_uniform_location(program, "u_environment"),
                primary_a: gl.get_uniform_location(program, "u_primary_a"),
                primary_b: gl.get_uniform_location(program, "u_primary_b"),
                primary_c: gl.get_uniform_location(program, "u_primary_c"),
                primary_spectrum_low: gl.get_uniform_location(program, "u_primary_spectrum_low"),
                primary_spectrum_high: gl.get_uniform_location(program, "u_primary_spectrum_high"),
                primary_pulses: [
                    gl.get_uniform_location(program, "u_primary_pulse_1"),
                    gl.get_uniform_location(program, "u_primary_pulse_2"),
                    gl.get_uniform_location(program, "u_primary_pulse_3"),
                    gl.get_uniform_location(program, "u_primary_pulse_4"),
                ],
                secondary_a: gl.get_uniform_location(program, "u_secondary_a"),
                secondary_b: gl.get_uniform_location(program, "u_secondary_b"),
                secondary_c: gl.get_uniform_location(program, "u_secondary_c"),
                secondary_spectrum_low: gl
                    .get_uniform_location(program, "u_secondary_spectrum_low"),
                secondary_spectrum_high: gl
                    .get_uniform_location(program, "u_secondary_spectrum_high"),
                secondary_pulses: [
                    gl.get_uniform_location(program, "u_secondary_pulse_1"),
                    gl.get_uniform_location(program, "u_secondary_pulse_2"),
                    gl.get_uniform_location(program, "u_secondary_pulse_3"),
                    gl.get_uniform_location(program, "u_secondary_pulse_4"),
                ],
            }
        }
    }
}

fn create_aura_program(gl: &glow::Context) -> Result<glow::Program, String> {
    unsafe {
        let program = gl
            .create_program()
            .map_err(|error| format!("cannot create aura shader program: {error}"))?;
        let vertex = match compile_aura_shader(gl, glow::VERTEX_SHADER, AURA_VERTEX_SHADER) {
            Ok(shader) => shader,
            Err(error) => {
                gl.delete_program(program);
                return Err(error);
            }
        };
        let fragment = match compile_aura_shader(gl, glow::FRAGMENT_SHADER, AURA_FRAGMENT_SHADER) {
            Ok(shader) => shader,
            Err(error) => {
                gl.delete_shader(vertex);
                gl.delete_program(program);
                return Err(error);
            }
        };

        gl.attach_shader(program, vertex);
        gl.attach_shader(program, fragment);
        gl.link_program(program);
        let linked = gl.get_program_link_status(program);
        let link_log = gl.get_program_info_log(program);
        gl.detach_shader(program, vertex);
        gl.detach_shader(program, fragment);
        gl.delete_shader(vertex);
        gl.delete_shader(fragment);

        if linked {
            Ok(program)
        } else {
            gl.delete_program(program);
            Err(format!("cannot link aura shader program: {link_log}"))
        }
    }
}

fn compile_aura_shader(
    gl: &glow::Context,
    shader_type: u32,
    source: &str,
) -> Result<glow::Shader, String> {
    unsafe {
        let shader = gl
            .create_shader(shader_type)
            .map_err(|error| format!("cannot create aura shader: {error}"))?;
        gl.shader_source(shader, source);
        gl.compile_shader(shader);
        if gl.get_shader_compile_status(shader) {
            Ok(shader)
        } else {
            let compile_log = gl.get_shader_info_log(shader);
            gl.delete_shader(shader);
            Err(format!("cannot compile aura shader: {compile_log}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_track_field_keeps_analysis_channels_in_explicit_uniform_groups() {
        let packed = PackedAuraTrackField::from(MusicPlayerAuraTrackField {
            bar_phase: 0.1,
            beat_phase: 0.2,
            downbeat_strength: 0.3,
            energy: 0.4,
            energy_momentum: -0.5,
            boundary: 0.6,
            novelty: 0.7,
            recurrence: 0.8,
            chorusness: 0.9,
            chroma_hue: 0.25,
            chroma_coherence: 0.75,
            section_color_unit: 0.5,
            section_color_strength: 0.0,
            spectrum_bands: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
            spectrum_peaks: [0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1],
            pulses: [
                crate::app::state::MusicPlayerAuraPulse {
                    origin: 0.1,
                    age: 0.2,
                    strength: 0.3,
                    air: 0.0,
                },
                crate::app::state::MusicPlayerAuraPulse {
                    origin: 0.4,
                    age: 0.5,
                    strength: 0.6,
                    air: 0.0,
                },
                crate::app::state::MusicPlayerAuraPulse {
                    origin: 0.7,
                    age: 0.8,
                    strength: 0.9,
                    air: 1.0,
                },
                crate::app::state::MusicPlayerAuraPulse::default(),
            ],
        });

        assert_eq!(packed.a, [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(packed.b, [-0.5, 0.6, 0.7, 0.8]);
        assert_eq!(packed.c, [0.9, 0.25, 0.75, 0.0]);
        assert_eq!(packed.spectrum_low, [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(packed.spectrum_high, [0.5, 0.6, 0.7, 0.8]);
        assert_eq!(packed.pulses[0], [0.1, 0.2, 0.3, 0.0]);
        assert_eq!(packed.pulses[2], [0.7, 0.8, 0.9, 1.0]);
    }

    #[test]
    fn shader_keeps_distributed_boundary_waves_searchable() {
        assert!(AURA_FRAGMENT_SHADER.contains("rounded_rect_sdf"));
        assert!(AURA_FRAGMENT_SHADER.contains("boundary_wavefront"));
        assert!(AURA_FRAGMENT_SHADER.contains("track_wave_field"));
        assert!(AURA_FRAGMENT_SHADER.contains("source_axis"));
        assert!(AURA_FRAGMENT_SHADER.contains("x_bias"));
        assert!(AURA_FRAGMENT_SHADER.contains("slow_lateral_drift"));
        assert!(AURA_FRAGMENT_SHADER.contains("along_distance"));
        assert!(AURA_FRAGMENT_SHADER.contains("inward_distance"));
        assert!(AURA_FRAGMENT_SHADER.contains("cycle_envelope"));
        assert!(AURA_FRAGMENT_SHADER.contains("previous frame"));
        assert!(AURA_FRAGMENT_SHADER.contains("color_coordinate"));
        assert!(AURA_FRAGMENT_SHADER.contains("ambient_fluid_field"));
        assert!(AURA_FRAGMENT_SHADER.contains("(low.x + low.y)"));
        assert!(AURA_FRAGMENT_SHADER.contains("(high.z + high.w)"));
        assert!(AURA_FRAGMENT_SHADER.contains("material_gate"));
        assert!(AURA_FRAGMENT_SHADER.contains("wave_wake"));
        assert!(AURA_FRAGMENT_SHADER.contains("wave_crest"));
        assert!(AURA_FRAGMENT_SHADER.contains("neutral_outer_cut"));
        assert!(AURA_FRAGMENT_SHADER.contains("neutral_inner_cut"));
        assert!(!AURA_FRAGMENT_SHADER.contains("CAUSTIC_EDGE_DEPTH"));
        assert!(!AURA_FRAGMENT_SHADER.contains("broad_caustic_pulse"));
        assert!(!AURA_FRAGMENT_SHADER.contains("precision_rgb_rail"));
        assert!(!AURA_FRAGMENT_SHADER.contains("peak_cap"));
        assert!(!AURA_FRAGMENT_SHADER.contains("atan(point.y, point.x)"));
        assert!(!AURA_FRAGMENT_SHADER.contains("rounded_perimeter_u"));
        assert!(!AURA_FRAGMENT_SHADER.contains("circular_distance"));
        assert!(!AURA_FRAGMENT_SHADER.contains("analysis_pearl"));
        assert!(!AURA_FRAGMENT_SHADER.contains("u_primary_peak_low"));
        assert!(AURA_FRAGMENT_SHADER.contains("mix(0.88, 0.76, dark_mode)"));
        assert!(AURA_FRAGMENT_SHADER.contains("cos(mix_progress * PI * 0.5)"));
        assert!(AURA_FRAGMENT_SHADER.contains("sin(mix_progress * PI * 0.5)"));
    }

    #[test]
    fn aura_dynamics_are_capped_at_thirty_fps_between_track_changes() {
        assert!(!aura_dynamics_should_advance(
            Duration::from_millis(16),
            false,
            false,
        ));
        assert!(aura_dynamics_should_advance(
            AURA_TARGET_FRAME_INTERVAL,
            false,
            false,
        ));
    }

    #[test]
    fn aura_dynamics_refresh_immediately_for_new_track_identity() {
        assert!(aura_dynamics_should_advance(Duration::ZERO, false, true,));
        assert!(aura_dynamics_should_advance(Duration::ZERO, true, false,));
    }

    #[test]
    fn window_background_is_the_reversible_experimental_default() {
        let presentation = MusicAuraPresentation::default();
        assert_eq!(presentation, MusicAuraPresentation::WindowBackground);
        assert_eq!(presentation.toggled(), MusicAuraPresentation::PlayerOnly);
        assert_eq!(
            presentation.toggled().toggled(),
            MusicAuraPresentation::WindowAmbient
        );
        assert_eq!(presentation.toggled().toggled().toggled(), presentation);
        assert!(presentation.window_layer_enabled());
        assert!(presentation.player_layer_enabled());
        assert!(presentation.background_glass_enabled());
    }

    #[test]
    fn shader_keeps_window_ambient_profile_separate_from_player_material() {
        assert!(AURA_FRAGMENT_SHADER.contains("float window_ambient"));
        assert!(AURA_FRAGMENT_SHADER.contains("mix(1.0, 0.08, window_ambient)"));
        assert!(AURA_FRAGMENT_SHADER.contains("mix(1.0, 0.34, window_ambient)"));
        assert!(AURA_FRAGMENT_SHADER.contains("mix(1.0, 0.20, window_ambient)"));
    }

    #[test]
    fn dark_window_backdrop_dim_is_bounded_and_reversible() {
        assert_eq!(WINDOW_MATERIAL_TRANSITION_SECONDS, 0.72);
        assert_eq!(window_backdrop_dim_alpha(-1.0), 0);
        assert_eq!(window_backdrop_dim_alpha(0.5), 44);
        assert_eq!(window_backdrop_dim_alpha(1.0), 88);
        assert_eq!(window_backdrop_dim_alpha(2.0), 88);
    }

    #[test]
    fn light_window_profile_keeps_high_value_and_uses_chroma_for_definition() {
        assert_eq!(LIGHT_WINDOW_PAPER_TINT, Color32::from_rgb(220, 228, 238));
        assert_eq!(LIGHT_WINDOW_PAPER_TINT_ALPHA, 72);
        assert!(AURA_FRAGMENT_SHADER.contains("float value = mix(0.92, 1.0, dark_mode)"));
        assert!(!AURA_FRAGMENT_SHADER.contains("visible_ambient_field"));
        assert!(AURA_FRAGMENT_SHADER.contains("mix(1.48, 1.0, dark_mode)"));
        assert!(AURA_FRAGMENT_SHADER.contains("mix(0.12, 0.34, dark_mode)"));
        assert!(
            AURA_FRAGMENT_SHADER.contains("float dark_mode = clamp(u_environment.z, 0.0, 1.0)")
        );
    }
}
