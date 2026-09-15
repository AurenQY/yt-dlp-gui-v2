//! Persisted graphics preference and command-line renderer/API selection.
use super::wgpu_api::WgpuApi;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderBackend {
    Wgpu,
    #[default]
    #[serde(other)]
    Glow,
}

impl RenderBackend {
    pub fn label(self) -> &'static str {
        match self {
            Self::Glow => "OpenGL (Glow)",
            Self::Wgpu => "Wgpu",
        }
    }

    pub fn renderer(self) -> eframe::Renderer {
        match self {
            Self::Glow => eframe::Renderer::Glow,
            Self::Wgpu => eframe::Renderer::Wgpu,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartupGraphicsSelection {
    pub backend: RenderBackend,
    pub wgpu_api: WgpuApi,
}

pub fn startup_graphics_selection(
    saved_backend: RenderBackend,
    saved_wgpu_api: WgpuApi,
    args: impl IntoIterator<Item = String>,
) -> Result<StartupGraphicsSelection, String> {
    let mut selected = StartupGraphicsSelection {
        backend: saved_backend,
        wgpu_api: saved_wgpu_api,
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let value = if arg == "--renderer" {
            Some(args.next().ok_or_else(renderer_value_error)?)
        } else {
            arg.strip_prefix("--renderer=").map(str::to_owned)
        };
        if let Some(value) = value {
            match value.as_str() {
                "glow" => selected.backend = RenderBackend::Glow,
                "wgpu" => selected.backend = RenderBackend::Wgpu,
                "auto" => {
                    selected.backend = RenderBackend::Wgpu;
                    selected.wgpu_api = WgpuApi::Auto;
                }
                "dx12" => {
                    selected.backend = RenderBackend::Wgpu;
                    selected.wgpu_api = WgpuApi::Dx12;
                }
                "vulkan" => {
                    selected.backend = RenderBackend::Wgpu;
                    selected.wgpu_api = WgpuApi::Vulkan;
                }
                "opengl" | "open_gl" => {
                    selected.backend = RenderBackend::Wgpu;
                    selected.wgpu_api = WgpuApi::OpenGl;
                }
                _ => return Err(renderer_value_error()),
            }
        }
    }
    Ok(selected)
}

fn renderer_value_error() -> String {
    "--renderer requires glow, wgpu, auto, dx12, vulkan, or opengl".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::AppConfig;

    #[test]
    fn legacy_and_unknown_preferences_preserve_glow_default() {
        let legacy: AppConfig = serde_yaml::from_str("ui_scale_percent: 125").unwrap();
        assert_eq!(legacy.render_backend, RenderBackend::Glow);
        let unknown: AppConfig =
            serde_yaml::from_str("render_backend: future\nui_scale_percent: 125").unwrap();
        assert_eq!(unknown.render_backend, RenderBackend::Glow);
        assert_eq!(unknown.ui_scale_percent, 125);
    }

    #[test]
    fn preferences_round_trip_and_select_the_corresponding_renderer() {
        for backend in [RenderBackend::Glow, RenderBackend::Wgpu] {
            let mut config = AppConfig::default();
            config.render_backend = backend;
            let yaml = serde_yaml::to_string(&config).unwrap();
            assert_eq!(
                serde_yaml::from_str::<AppConfig>(&yaml)
                    .unwrap()
                    .render_backend,
                backend
            );
            assert_eq!(
                startup_graphics_selection(backend, WgpuApi::Auto, [])
                    .unwrap()
                    .backend,
                backend
            );
        }
        assert_eq!(RenderBackend::Glow.renderer(), eframe::Renderer::Glow);
        assert_eq!(RenderBackend::Wgpu.renderer(), eframe::Renderer::Wgpu);
    }

    #[test]
    fn command_line_selection_supports_renderers_and_wgpu_apis() {
        let args = |values: &[&str]| values.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            startup_graphics_selection(
                RenderBackend::Wgpu,
                WgpuApi::Vulkan,
                args(&["--renderer=glow"]),
            )
            .unwrap(),
            StartupGraphicsSelection {
                backend: RenderBackend::Glow,
                wgpu_api: WgpuApi::Vulkan,
            }
        );
        assert_eq!(
            startup_graphics_selection(
                RenderBackend::Glow,
                WgpuApi::Auto,
                args(&["--renderer", "dx12"]),
            )
            .unwrap(),
            StartupGraphicsSelection {
                backend: RenderBackend::Wgpu,
                wgpu_api: WgpuApi::Dx12,
            }
        );
        assert_eq!(
            startup_graphics_selection(
                RenderBackend::Wgpu,
                WgpuApi::OpenGl,
                args(&["--startup-diagnostics"]),
            )
            .unwrap(),
            StartupGraphicsSelection {
                backend: RenderBackend::Wgpu,
                wgpu_api: WgpuApi::OpenGl,
            }
        );
        for values in [
            vec!["--renderer"],
            vec!["--renderer="],
            vec!["--renderer=invalid"],
        ] {
            assert!(
                startup_graphics_selection(RenderBackend::Glow, WgpuApi::Auto, args(&values))
                    .is_err()
            );
        }
    }
}
