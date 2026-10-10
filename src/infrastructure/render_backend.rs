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
