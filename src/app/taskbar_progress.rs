//! Windows taskbar button progress (ITaskbarList3). A no-op elsewhere.
use super::state::TaskbarProgress;

pub(super) struct TaskbarProgressIndicator {
    #[cfg(windows)]
    inner: Option<windows_impl::Taskbar>,
    last: Option<TaskbarProgress>,
}

impl TaskbarProgressIndicator {
    #[cfg(windows)]
    pub(super) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            inner: windows_impl::Taskbar::new(cc),
            last: None,
        }
    }

    #[cfg(not(windows))]
    pub(super) fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self { last: None }
    }

    /// Push a new state; repeated identical states are not resent.
    pub(super) fn update(&mut self, progress: TaskbarProgress) {
        let progress = quantize(progress);
        if self.last == Some(progress) {
            return;
        }
        self.last = Some(progress);
        #[cfg(windows)]
        if let Some(inner) = &self.inner {
            inner.apply(progress);
        }
    }
}

/// Round to 0.1 % so per-frame byte progress does not spam COM calls.
fn quantize(progress: TaskbarProgress) -> TaskbarProgress {
    let round = |value: f32| (value.clamp(0.0, 1.0) * 1000.0).round() / 1000.0;
    match progress {
        TaskbarProgress::Normal(value) => TaskbarProgress::Normal(round(value)),
        TaskbarProgress::Error(value) => TaskbarProgress::Error(round(value)),
        other => other,
    }
}

#[cfg(windows)]
mod windows_impl {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    };
    use windows::Win32::UI::Shell::{
        ITaskbarList3, TBPF_ERROR, TBPF_INDETERMINATE, TBPF_NOPROGRESS, TBPF_NORMAL, TaskbarList,
    };

    use super::TaskbarProgress;

    pub(super) struct Taskbar {
        list: ITaskbarList3,
        hwnd: HWND,
    }

    impl Taskbar {
        pub(super) fn new(cc: &eframe::CreationContext<'_>) -> Option<Self> {
            let window = cc.winit_window()?;
            let handle = window.window_handle().ok()?;
            let RawWindowHandle::Win32(handle) = handle.as_raw() else {
                return None;
            };
            let hwnd = HWND(handle.hwnd.get() as *mut core::ffi::c_void);
            // SAFETY: COM is initialised for this (UI) thread before use. A
            // different existing apartment mode is fine for this in-proc object.
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
                let list: ITaskbarList3 =
                    CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER).ok()?;
                list.HrInit().ok()?;
                Some(Self { list, hwnd })
            }
        }

        pub(super) fn apply(&self, progress: TaskbarProgress) {
            const SCALE: u64 = 1000;
            // SAFETY: the window handle belongs to this process and outlives the app.
            unsafe {
                let _ = match progress {
                    TaskbarProgress::Hidden => {
                        self.list.SetProgressState(self.hwnd, TBPF_NOPROGRESS)
                    }
                    TaskbarProgress::Indeterminate => {
                        self.list.SetProgressState(self.hwnd, TBPF_INDETERMINATE)
                    }
                    TaskbarProgress::Normal(value) => self
                        .list
                        .SetProgressState(self.hwnd, TBPF_NORMAL)
                        .and_then(|_| {
                            self.list.SetProgressValue(
                                self.hwnd,
                                (value * SCALE as f32) as u64,
                                SCALE,
                            )
                        }),
                    TaskbarProgress::Error(value) => self
                        .list
                        .SetProgressState(self.hwnd, TBPF_ERROR)
                        .and_then(|_| {
                            self.list.SetProgressValue(
                                self.hwnd,
                                (value * SCALE as f32) as u64,
                                SCALE,
                            )
                        }),
                };
            }
        }
    }
}
