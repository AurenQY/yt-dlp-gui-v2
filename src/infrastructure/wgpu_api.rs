//! Requested next-launch API is distinct from the adapter actually created.
use eframe::{egui_wgpu, wgpu};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WgpuApi {
    Dx12,
    Vulkan,
    OpenGl,
    #[default]
    #[serde(other)]
    Auto,
}

impl WgpuApi {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Dx12 => "DirectX 12",
            Self::Vulkan => "Vulkan",
            Self::OpenGl => "OpenGL",
        }
    }

    pub fn backends(self) -> Option<wgpu::Backends> {
        match self {
            Self::Auto => None,
            Self::Dx12 => Some(wgpu::Backends::DX12),
            Self::Vulkan => Some(wgpu::Backends::VULKAN),
            Self::OpenGl => Some(wgpu::Backends::GL),
        }
    }

    pub fn configure(self, options: &mut egui_wgpu::WgpuConfiguration) {
        if let (Some(backends), egui_wgpu::WgpuSetup::CreateNew(setup)) =
            (self.backends(), &mut options.wgpu_setup)
        {
            setup.instance_descriptor.backends = backends;
        }
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeRendererInfo {
    pub api: String,
    pub gpu: String,
    pub is_wgpu: bool,
}

impl RuntimeRendererInfo {
    pub fn from_creation(cc: &eframe::CreationContext<'_>) -> Self {
        let info = if let Some(state) = &cc.wgpu_render_state {
            let adapter = state.adapter.get_info();
            Self {
                api: match adapter.backend {
                    wgpu::Backend::Dx12 => "DirectX 12",
                    wgpu::Backend::Vulkan => "Vulkan",
                    wgpu::Backend::Gl => "OpenGL",
                    wgpu::Backend::Metal => "Metal",
                    wgpu::Backend::BrowserWebGpu => "WebGPU",
                    _ => "Unknown",
                }
                .to_owned(),
                gpu: format!(
                    "{} / {} {}",
                    adapter.name, adapter.driver, adapter.driver_info
                ),
                is_wgpu: true,
            }
        } else {
            use eframe::glow::HasContext;
            Self {
                api: "OpenGL (Glow)".to_owned(),
                gpu: cc
                    .gl
                    .as_ref()
                    .map(|gl| unsafe {
                        format!(
                            "{} / {}",
                            gl.get_parameter_string(eframe::glow::RENDERER),
                            gl.get_parameter_string(eframe::glow::VERSION)
                        )
                    })
                    .unwrap_or_default(),
                is_wgpu: false,
            }
        };
        crate::infrastructure::record_startup_event(
            "graphics-adapter",
            format!("api={} gpu={}", info.api, info.gpu),
        );
        info
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_defaults_round_trip_and_explicit_masks() {
        let legacy: crate::infrastructure::AppConfig = serde_yaml::from_str("{}").unwrap();
        assert_eq!(legacy.wgpu_api, WgpuApi::Auto);
        assert_eq!(
            serde_yaml::from_str::<WgpuApi>("future").unwrap(),
            WgpuApi::Auto
        );
        for api in [
            WgpuApi::Auto,
            WgpuApi::Dx12,
            WgpuApi::Vulkan,
            WgpuApi::OpenGl,
        ] {
            assert_eq!(
                serde_yaml::from_str::<WgpuApi>(&serde_yaml::to_string(&api).unwrap()).unwrap(),
                api
            );
            let mut options = egui_wgpu::WgpuConfiguration::default();
            api.configure(&mut options);
            if let Some(mask) = api.backends() {
                let egui_wgpu::WgpuSetup::CreateNew(setup) = options.wgpu_setup else {
                    panic!()
                };
                assert_eq!(setup.instance_descriptor.backends, mask);
                assert_eq!(mask.bits().count_ones(), 1);
            }
        }
    }
}
