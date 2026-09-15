#version 140
in vec2 v_uv;
out vec4 out_color;

uniform vec3 u_size_radius;
uniform vec3 u_accent;
uniform vec4 u_environment;
uniform vec4 u_primary_a;
uniform vec4 u_primary_b;
uniform vec4 u_primary_c;
uniform vec4 u_primary_spectrum_low;
uniform vec4 u_primary_spectrum_high;
uniform vec4 u_primary_pulse_1;
uniform vec4 u_primary_pulse_2;
uniform vec4 u_primary_pulse_3;
uniform vec4 u_primary_pulse_4;
uniform vec4 u_secondary_a;
uniform vec4 u_secondary_b;
uniform vec4 u_secondary_c;
uniform vec4 u_secondary_spectrum_low;
uniform vec4 u_secondary_spectrum_high;
uniform vec4 u_secondary_pulse_1;
uniform vec4 u_secondary_pulse_2;
uniform vec4 u_secondary_pulse_3;
uniform vec4 u_secondary_pulse_4;

const float PI = 3.14159265358979323846;
const float TAU = 6.28318530717958647692;

float rounded_rect_sdf(vec2 point, vec2 half_size, float radius) {
    vec2 inner = half_size - vec2(radius);
    vec2 q = abs(point) - inner;
    return length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

vec3 hsv_to_rgb(vec3 hsv) {
    vec3 p = abs(fract(hsv.xxx + vec3(0.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - 3.0);
    return hsv.z * mix(vec3(1.0), clamp(p - 1.0, 0.0, 1.0), hsv.y);
}

vec2 boundary_wavefront(
    vec4 pulse,
    vec2 point,
    vec2 half_size,
    float x_bias
) {
    float strength = clamp(pulse.z, 0.0, 1.0);
    if (strength <= 0.001) {
        return vec2(0.0);
    }

    float age = clamp(pulse.y, 0.0, 1.0);
    float air = clamp(pulse.w, 0.0, 1.0);
    float source_angle = TAU * pulse.x;
    vec2 source_axis = vec2(cos(source_angle), sin(source_angle));
    vec2 source_extent = max(half_size - vec2(4.0), vec2(1.0));
    float source_scale = min(
        source_extent.x / max(abs(source_axis.x), 0.001),
        source_extent.y / max(abs(source_axis.y), 0.001)
    );
    vec2 source = source_axis * source_scale;
    float slow_lateral_drift = sin(source_angle * 1.73) * 0.035;
    source.x = clamp(
        source.x + half_size.x * (x_bias + slow_lateral_drift),
        -source_extent.x,
        source_extent.x
    );
    vec2 tangent = vec2(-source_axis.y, source_axis.x);
    vec2 inward = -source_axis;
    vec2 delta = point - source;
    float along_distance = abs(dot(delta, tangent));
    float inward_distance = max(dot(delta, inward), 0.0);
    float radius = mix(5.0, mix(94.0, 56.0, air), age);
    float width = mix(13.5, 6.5, air) * (0.82 + strength * 0.28);
    float distortion =
        1.0
        + sin(
            along_distance * 0.041 + source_angle * 1.7 + age * TAU
        ) * 0.10
        + sin(
            inward_distance * 0.057 - source_angle + age * TAU * 0.61
        ) * 0.045;
    float inward_scale = mix(0.72, 1.12, air);
    float metric = sqrt(
        pow(along_distance * distortion, 2.0)
        + pow(inward_distance * inward_scale, 2.0)
    );
    // A persistent lobe cycles only while its visibility is zero at the wrap.
    // Its identity, position and energy continue from the previous frame.
    float cycle_envelope = pow(sin(PI * age), mix(1.15, 1.55, air));
    float crest = exp(-pow((metric - radius) / width, 2.0))
        * strength * cycle_envelope;
    float wake =
        (1.0 - smoothstep(radius - width * 1.7, radius + width * 0.30, metric))
        * exp(-inward_distance / mix(35.0, 19.0, air))
        * strength
        * cycle_envelope;
    return vec2(wake, crest);
}

vec2 track_wave_field(
    vec4 pulse_1,
    vec4 pulse_2,
    vec4 pulse_3,
    vec4 pulse_4,
    vec2 point,
    vec2 half_size
) {
    vec2 field =
        boundary_wavefront(pulse_1, point, half_size, -0.18)
        + boundary_wavefront(pulse_2, point, half_size, 0.14)
        + boundary_wavefront(pulse_3, point, half_size, -0.08)
        + boundary_wavefront(pulse_4, point, half_size, 0.22);
    return clamp(field, vec2(0.0), vec2(1.35));
}

float ambient_fluid_field(
    vec4 a,
    vec4 b,
    vec4 c,
    vec4 low,
    vec4 high,
    vec2 normalized_point,
    float direction
) {
    float energy = clamp(a.w, 0.0, 1.0);
    float momentum = clamp(b.x, -1.0, 1.0);
    float low_current = clamp((low.x + low.y) * 0.5, 0.0, 1.0);
    float air_current = clamp((high.z + high.w) * 0.5, 0.0, 1.0);
    float phase = TAU * (a.x + c.y * 0.09);
    vec2 center_1 = vec2(
        -0.72 * direction + sin(phase) * 0.08,
        -0.58 + cos(phase * 0.73) * 0.10
    );
    vec2 center_2 = vec2(
        0.64 * direction + cos(phase * 0.61) * 0.11,
        0.64 + sin(phase * 0.87) * 0.08
    );
    vec2 delta_1 = (normalized_point - center_1) / vec2(0.78, 0.56);
    vec2 delta_2 = (normalized_point - center_2) / vec2(0.66, 0.62);
    float blob_1 = exp(-dot(delta_1, delta_1) * 1.65);
    float blob_2 = exp(-dot(delta_2, delta_2) * 1.85);
    float fold = 0.5 + 0.5 * sin(TAU * (
        normalized_point.x * 0.34
        - normalized_point.y * 0.27
        + direction * sin(phase) * 0.07
        + momentum * 0.04
    ));
    return clamp(
        (blob_1 + blob_2) * (0.055 + energy * 0.095)
        + blob_1 * low_current * 0.075
        + blob_2 * air_current * 0.060
        + fold * clamp(b.z, 0.0, 1.0) * 0.035,
        0.0,
        0.30
    );
}

vec3 analytic_track_color(vec4 a, vec4 b, vec4 c, float u, float dark_mode) {
    float hue = fract(
        c.y
        + u * 0.34
        + sin(TAU * (2.0 * u + a.x)) * (0.030 + b.z * 0.025)
        + b.x * 0.015
    );
    float saturation = clamp(
        mix(0.88, 0.76, dark_mode) + c.z * 0.16 + c.x * 0.05,
        mix(0.88, 0.76, dark_mode),
        0.98
    );
    // Light mode gains definition from chroma, not from muddy low luminance.
    float value = mix(0.92, 1.0, dark_mode);
    vec3 harmonic = hsv_to_rgb(vec3(hue, saturation, value));
    return mix(u_accent, harmonic, 0.74 + c.z * 0.18);
}

void main() {
    vec2 size = max(u_size_radius.xy, vec2(1.0));
    vec2 half_size = size * 0.5;
    float radius = clamp(u_size_radius.z, 1.0, min(half_size.x, half_size.y));
    vec2 point = (v_uv - 0.5) * size;
    float signed_distance = rounded_rect_sdf(point, half_size, radius);
    float inside = 1.0 - smoothstep(-0.35, 0.85, signed_distance);
    if (inside <= 0.001) {
        out_color = vec4(0.0);
        return;
    }

    float edge_distance = max(-signed_distance, 0.0);
    vec2 normalized_point = point / max(half_size, vec2(1.0));
    // Color drifts continuously across the material. It must not reuse a
    // nearest-edge perimeter coordinate because that coordinate has diagonal
    // medial-axis seams when extended through the panel interior.
    float color_coordinate = fract(
        0.50 + normalized_point.x * 0.18 + normalized_point.y * 0.13
    );
    float mix_progress = clamp(u_environment.x, 0.0, 1.0);
    float window_ambient = clamp(u_environment.w, 0.0, 1.0);
    float dark_mode = clamp(u_environment.z, 0.0, 1.0);
    float weight_a = cos(mix_progress * PI * 0.5);
    float weight_b = sin(mix_progress * PI * 0.5);
    vec2 wave_a = track_wave_field(
        u_primary_pulse_1,
        u_primary_pulse_2,
        u_primary_pulse_3,
        u_primary_pulse_4,
        point,
        half_size
    );
    vec2 wave_b = track_wave_field(
        u_secondary_pulse_1,
        u_secondary_pulse_2,
        u_secondary_pulse_3,
        u_secondary_pulse_4,
        point,
        half_size
    );
    vec2 wave_amplitude_a = weight_a * wave_a;
    vec2 wave_amplitude_b = weight_b * wave_b;
    float wave_wake = sqrt(
        wave_amplitude_a.x * wave_amplitude_a.x
        + wave_amplitude_b.x * wave_amplitude_b.x
    );
    float wave_crest = sqrt(
        wave_amplitude_a.y * wave_amplitude_a.y
        + wave_amplitude_b.y * wave_amplitude_b.y
    );
    float ambient_a = ambient_fluid_field(
        u_primary_a,
        u_primary_b,
        u_primary_c,
        u_primary_spectrum_low,
        u_primary_spectrum_high,
        normalized_point,
        1.0
    );
    float ambient_b = ambient_fluid_field(
        u_secondary_a,
        u_secondary_b,
        u_secondary_c,
        u_secondary_spectrum_low,
        u_secondary_spectrum_high,
        normalized_point,
        -1.0
    );
    float ambient_field = sqrt(
        pow(weight_a * ambient_a, 2.0) + pow(weight_b * ambient_b, 2.0)
    );
    vec3 color_a = analytic_track_color(
        u_primary_a, u_primary_b, u_primary_c, color_coordinate, u_environment.z
    );
    vec3 color_b = analytic_track_color(
        u_secondary_a, u_secondary_b, u_secondary_c, color_coordinate, u_environment.z
    );
    float color_weight_a = weight_a * (0.12 + ambient_a + wave_a.x + wave_a.y);
    float color_weight_b = weight_b * (0.12 + ambient_b + wave_b.x + wave_b.y);
    vec3 color = (
        color_a * color_weight_a + color_b * color_weight_b
    ) / max(color_weight_a + color_weight_b, 0.001);

    float top_light = clamp(0.5 - normalized_point.y * 0.32, 0.0, 1.0);
    float neutral_outer_cut = exp(-pow((edge_distance - 0.60) / 0.50, 2.0));
    float neutral_inner_cut = exp(-pow((edge_distance - 1.85) / 0.78, 2.0));
    float neutral_alpha = inside * (
        neutral_outer_cut * 0.040
        + neutral_inner_cut * (0.052 + top_light * 0.042)
    ) * mix(1.0, 0.08, window_ambient);

    // The neutral frame owns the first pixels. Beyond that moat, wave age and
    // anisotropic distance decide the shape; there is no fixed rectangular
    // inner cutoff.
    float material_gate = smoothstep(1.65, 3.05, edge_distance);
    float motion_presence = mix(0.66, 1.0, u_environment.y);
    float fluid_alpha = inside * material_gate * motion_presence * (
        ambient_field * 0.72 + wave_wake * 0.34
    ) * mix(1.48, 1.0, dark_mode) * mix(1.0, 0.34, window_ambient);
    float crest_alpha = inside * material_gate * motion_presence * wave_crest * 0.46
        * mix(0.76, 1.0, dark_mode)
        * mix(1.0, 0.20, window_ambient);

    float alpha = clamp(
        neutral_alpha + fluid_alpha + crest_alpha,
        0.0,
        0.86
    );
    vec3 neutral_color = vec3(mix(0.52, 0.76, top_light));
    vec3 crest_color = mix(color, vec3(1.0), mix(0.12, 0.34, dark_mode));
    vec3 premultiplied = (
        neutral_color * neutral_alpha
        + color * fluid_alpha
        + crest_color * crest_alpha
    );

    out_color = vec4(premultiplied, alpha);
}
