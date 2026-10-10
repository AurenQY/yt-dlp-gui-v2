//! Native hit testing only. winit owns decorations, client geometry and DPI resizing.
#[cfg(windows)]
#[path = "custom_chrome_diagnostics.rs"]
pub(crate) mod diagnostics;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustomChromeResult {
    Applied,
    NoWindow,
    Unsupported,
}

#[cfg(windows)]
pub fn install(cc: &eframe::CreationContext<'_>) -> CustomChromeResult {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Some(window) = cc.winit_window() else {
        return CustomChromeResult::NoWindow;
    };
    let Ok(handle) = window.window_handle() else {
        return CustomChromeResult::Unsupported;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return CustomChromeResult::Unsupported;
    };
    windows::install(handle.hwnd.get() as _)
}

#[cfg(not(windows))]
pub fn install(_cc: &eframe::CreationContext<'_>) -> CustomChromeResult {
    CustomChromeResult::Applied
}

#[cfg(windows)]
pub(crate) fn set_titlebar_client_hit_test_for_popup(enabled: bool) {
    windows::set_titlebar_client_hit_test_for_popup(enabled);
}

#[cfg(not(windows))]
pub(crate) fn set_titlebar_client_hit_test_for_popup(_enabled: bool) {}

#[cfg(windows)]
pub(crate) fn set_titlebar_hit_test_metrics(
    titlebar_height_points: f32,
    right_client_area_width_points: f32,
    pixels_per_point: f32,
) {
    windows::set_titlebar_hit_test_metrics(
        titlebar_height_points,
        right_client_area_width_points,
        pixels_per_point,
    );
}

#[cfg(not(windows))]
pub(crate) fn set_titlebar_hit_test_metrics(
    _titlebar_height_points: f32,
    _right_client_area_width_points: f32,
    _pixels_per_point: f32,
) {
}

#[cfg(windows)]
mod windows {
    use std::{
        ffi::c_void,
        mem,
        sync::{
            Mutex, OnceLock,
            atomic::{AtomicBool, AtomicI32, Ordering},
        },
    };

    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows_sys::Win32::Graphics::{Dwm::DwmSetWindowAttribute, Gdi::ClientToScreen};
    use windows_sys::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, DefWindowProcW, GWLP_WNDPROC, GetClientRect, GetWindowLongPtrW,
        GetWindowRect, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTCLIENT, HTLEFT, HTRIGHT,
        HTTOP, HTTOPLEFT, HTTOPRIGHT, IsZoomed, SM_CXFRAME, SM_CXPADDEDBORDER, SM_CYFRAME,
        SetWindowLongPtrW, WM_NCDESTROY, WM_NCHITTEST, WNDPROC,
    };

    use super::CustomChromeResult;

    const TITLEBAR_HEIGHT_LOGICAL: i32 = 26;
    const DEFAULT_RIGHT_CLIENT_AREA_WIDTH_LOGICAL: i32 = 160;
    const MIN_RESIZE_BORDER_PX: i32 = 6;

    #[derive(Default)]
    struct ChromeState {
        hwnd: isize,
        previous_wndproc: isize,
    }

    static CHROME_STATE: OnceLock<Mutex<ChromeState>> = OnceLock::new();
    static TITLEBAR_CLIENT_HIT_TEST_FOR_POPUP: AtomicBool = AtomicBool::new(false);
    static TITLEBAR_HIT_TEST_HEIGHT_PX: AtomicI32 = AtomicI32::new(TITLEBAR_HEIGHT_LOGICAL);
    static TITLEBAR_RIGHT_CLIENT_AREA_WIDTH_PX: AtomicI32 =
        AtomicI32::new(DEFAULT_RIGHT_CLIENT_AREA_WIDTH_LOGICAL);

    pub fn set_titlebar_client_hit_test_for_popup(enabled: bool) {
        TITLEBAR_CLIENT_HIT_TEST_FOR_POPUP.store(enabled, Ordering::Relaxed);
    }

    pub fn set_titlebar_hit_test_metrics(
        titlebar_height_points: f32,
        right_client_area_width_points: f32,
        pixels_per_point: f32,
    ) {
        let pixels_per_point = pixels_per_point.max(0.1);
        let titlebar_height = (titlebar_height_points * pixels_per_point).round().max(1.0) as i32;
        let min_right_client_width = (DEFAULT_RIGHT_CLIENT_AREA_WIDTH_LOGICAL as f32
            * pixels_per_point)
            .round()
            .max(1.0);
        let right_client_width = (right_client_area_width_points * pixels_per_point)
            .round()
            .max(min_right_client_width) as i32;

        TITLEBAR_HIT_TEST_HEIGHT_PX.store(titlebar_height, Ordering::Relaxed);
        TITLEBAR_RIGHT_CLIENT_AREA_WIDTH_PX.store(right_client_width, Ordering::Relaxed);
    }

    pub fn install(hwnd: HWND) -> CustomChromeResult {
        if is_installed_for(hwnd) {
            apply_rounded_corners(hwnd);
            return CustomChromeResult::Applied;
        }

        super::diagnostics::record_geometry(hwnd, "before-subclass");
        if !subclass_window(hwnd) {
            return CustomChromeResult::Unsupported;
        }

        apply_rounded_corners(hwnd);
        super::diagnostics::after_install(hwnd);
        CustomChromeResult::Applied
    }

    fn is_installed_for(hwnd: HWND) -> bool {
        let state = chrome_state().lock().ok();
        state
            .as_ref()
            .is_some_and(|state| state.hwnd == hwnd as isize && state.previous_wndproc != 0)
    }

    fn subclass_window(hwnd: HWND) -> bool {
        unsafe {
            let current = GetWindowLongPtrW(hwnd, GWLP_WNDPROC);
            if current == custom_window_proc_ptr() {
                return true;
            }

            let previous = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, custom_window_proc_ptr());
            if previous == 0 {
                return false;
            }

            if let Ok(mut state) = chrome_state().lock() {
                state.hwnd = hwnd as isize;
                state.previous_wndproc = previous;
                true
            } else {
                let _ = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, previous);
                false
            }
        }
    }

    fn chrome_state() -> &'static Mutex<ChromeState> {
        CHROME_STATE.get_or_init(|| Mutex::new(ChromeState::default()))
    }

    fn custom_window_proc_ptr() -> isize {
        custom_window_proc as *const () as usize as isize
    }

    fn apply_rounded_corners(hwnd: HWND) {
        const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
        const DWMWCP_ROUND: u32 = 2;

        unsafe {
            let preference = DWMWCP_ROUND;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &preference as *const u32 as *const c_void,
                mem::size_of::<u32>() as u32,
            );
        }
    }

    unsafe extern "system" fn custom_window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if super::diagnostics::observe_message(hwnd, msg, lparam) {
            return 0;
        }
        match msg {
            WM_NCHITTEST => {
                let hit = hit_test(hwnd, lparam);
                if hit != HTCLIENT as i32 {
                    return hit as LRESULT;
                }
            }
            WM_NCDESTROY => {
                let result = call_previous_window_proc(hwnd, msg, wparam, lparam);
                uninstall_if_current(hwnd);
                return result;
            }
            _ => {}
        }

        // In particular, forward NCCALCSIZE / WINDOWPOSCHANGED / SIZE / DPICHANGED.
        // winit must see its own undecorated client-area and sizing contract.
        let result = call_previous_window_proc(hwnd, msg, wparam, lparam);
        super::diagnostics::record_geometry_after_message(hwnd, msg);
        result
    }

    fn hit_test(hwnd: HWND, lparam: LPARAM) -> i32 {
        let Some(window_rect) = window_rect(hwnd) else {
            return HTCLIENT as i32;
        };

        let cursor_x = signed_low_word(lparam);
        let cursor_y = signed_high_word(lparam);
        let maximized = unsafe { IsZoomed(hwnd) != 0 };

        if !maximized {
            let border_x = resize_border_x(hwnd).max(MIN_RESIZE_BORDER_PX);
            let border_y = resize_border_y(hwnd).max(MIN_RESIZE_BORDER_PX);
            let on_left = cursor_x < window_rect.left + border_x;
            let on_right = cursor_x >= window_rect.right - border_x;
            let on_top = cursor_y < window_rect.top + border_y;
            let on_bottom = cursor_y >= window_rect.bottom - border_y;

            match (on_left, on_right, on_top, on_bottom) {
                (true, false, true, false) => return HTTOPLEFT as i32,
                (false, true, true, false) => return HTTOPRIGHT as i32,
                (true, false, false, true) => return HTBOTTOMLEFT as i32,
                (false, true, false, true) => return HTBOTTOMRIGHT as i32,
                (true, false, false, false) => return HTLEFT as i32,
                (false, true, false, false) => return HTRIGHT as i32,
                (false, false, true, false) => return HTTOP as i32,
                (false, false, false, true) => return HTBOTTOM as i32,
                _ => {}
            }
        }

        let titlebar_height = TITLEBAR_HIT_TEST_HEIGHT_PX.load(Ordering::Relaxed).max(1);
        let right_client_width = TITLEBAR_RIGHT_CLIENT_AREA_WIDTH_PX
            .load(Ordering::Relaxed)
            .max(1);
        let Some(client_rect) = client_screen_rect(hwnd) else {
            return HTCLIENT as i32;
        };
        let in_titlebar = cursor_y >= client_rect.top
            && cursor_y < client_rect.top + titlebar_height
            && cursor_x >= client_rect.left
            && cursor_x < client_rect.right;
        if in_titlebar {
            let in_right_client_area = cursor_x >= client_rect.right - right_client_width;
            if in_right_client_area || TITLEBAR_CLIENT_HIT_TEST_FOR_POPUP.load(Ordering::Relaxed) {
                return HTCLIENT as i32;
            }
            return HTCAPTION as i32;
        }

        HTCLIENT as i32
    }

    fn resize_border_x(hwnd: HWND) -> i32 {
        unsafe {
            let dpi = GetDpiForWindow(hwnd);
            GetSystemMetricsForDpi(SM_CXFRAME, dpi) + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi)
        }
    }

    fn resize_border_y(hwnd: HWND) -> i32 {
        unsafe {
            let dpi = GetDpiForWindow(hwnd);
            GetSystemMetricsForDpi(SM_CYFRAME, dpi) + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi)
        }
    }

    fn client_screen_rect(hwnd: HWND) -> Option<RECT> {
        let mut rect = RECT::default();
        let mut origin = POINT::default();
        unsafe {
            if GetClientRect(hwnd, &mut rect) == 0 || ClientToScreen(hwnd, &mut origin) == 0 {
                return None;
            }
        }
        rect.left += origin.x;
        rect.right += origin.x;
        rect.top += origin.y;
        rect.bottom += origin.y;
        Some(rect)
    }

    fn window_rect(hwnd: HWND) -> Option<RECT> {
        unsafe {
            let mut rect = RECT::default();
            (GetWindowRect(hwnd, &mut rect) != 0).then_some(rect)
        }
    }

    fn signed_low_word(value: LPARAM) -> i32 {
        (value as u32 & 0xffff) as i16 as i32
    }

    fn signed_high_word(value: LPARAM) -> i32 {
        ((value as u32 >> 16) & 0xffff) as i16 as i32
    }

    fn call_previous_window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if let Some(previous) = previous_window_proc(hwnd) {
            unsafe { CallWindowProcW(Some(previous), hwnd, msg, wparam, lparam) }
        } else {
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
    }

    fn previous_window_proc(hwnd: HWND) -> WNDPROC {
        let state = chrome_state().lock().ok()?;
        if state.hwnd != hwnd as isize || state.previous_wndproc == 0 {
            return None;
        }
        unsafe { mem::transmute::<isize, WNDPROC>(state.previous_wndproc) }
    }

    fn uninstall_if_current(hwnd: HWND) {
        let Ok(mut state) = chrome_state().lock() else {
            return;
        };
        if state.hwnd != hwnd as isize || state.previous_wndproc == 0 {
            return;
        }

        unsafe {
            let current = GetWindowLongPtrW(hwnd, GWLP_WNDPROC);
            if current == custom_window_proc_ptr() {
                let _ = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, state.previous_wndproc);
            }
        }

        state.hwnd = 0;
        state.previous_wndproc = 0;
    }
}
