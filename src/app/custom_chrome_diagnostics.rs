//! Opt-in read-only geometry observations; frame fault experiments are debug-only.
use std::sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering},
};
use std::time::Instant;
#[cfg(debug_assertions)]
use std::{sync::atomic::AtomicBool, time::Duration};

use eframe::{egui, glow::HasContext};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows_sys::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindowRect, WM_DPICHANGED, WM_NCCALCSIZE, WM_SIZE, WM_WINDOWPOSCHANGED,
};

use crate::infrastructure::record_startup_event;
use crate::infrastructure::startup_diagnostics::is_startup_diagnostics_enabled;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Observe,
    #[cfg(debug_assertions)]
    Baseline,
    #[cfg(debug_assertions)]
    SkipRefresh,
    #[cfg(debug_assertions)]
    DropSize,
}

fn parse_mode(value: &str) -> Option<Mode> {
    match value {
        "--chrome-diagnostic=observe" => Some(Mode::Observe),
        #[cfg(debug_assertions)]
        "--chrome-diagnostic=baseline" => Some(Mode::Baseline),
        #[cfg(debug_assertions)]
        "--chrome-diagnostic=skip-refresh" => Some(Mode::SkipRefresh),
        #[cfg(debug_assertions)]
        "--chrome-diagnostic=drop-size" => Some(Mode::DropSize),
        _ => None,
    }
}

fn mode() -> Option<Mode> {
    static MODE: OnceLock<Option<Mode>> = OnceLock::new();
    *MODE.get_or_init(|| {
        if !is_startup_diagnostics_enabled() {
            return None;
        }
        let mode = std::env::args().find_map(|arg| parse_mode(&arg));
        if let Some(mode) = mode {
            record_startup_event("chrome-diagnostic", format!("mode={mode:?}"));
        }
        mode
    })
}

pub(super) fn after_install(hwnd: HWND) {
    record_geometry(hwnd, "after-subclass");
    // Normal startup and Observe never inject changes to client geometry.
    #[cfg(debug_assertions)]
    if matches!(mode(), Some(Mode::Baseline | Mode::DropSize)) {
        refresh_non_client_frame(hwnd);
        record_geometry(hwnd, "debug-initial-refresh");
    }
}

fn is_geometry_message(msg: u32) -> bool {
    matches!(
        msg,
        WM_SIZE | WM_NCCALCSIZE | WM_WINDOWPOSCHANGED | WM_DPICHANGED
    )
}

// Only an explicit debug DropSize experiment may consume a notification.
pub(super) fn observe_message(hwnd: HWND, msg: u32, lparam: LPARAM) -> bool {
    let Some(mode) = mode() else {
        return false;
    };
    if !is_geometry_message(msg) {
        return false;
    }
    #[cfg(debug_assertions)]
    let drop_size = {
        static SIZE_DROPPED: AtomicBool = AtomicBool::new(false);
        msg == WM_SIZE && mode == Mode::DropSize && !SIZE_DROPPED.swap(true, Ordering::Relaxed)
    };
    #[cfg(not(debug_assertions))]
    let drop_size = false;
    record_startup_event(
        "chrome-message",
        format!("mode={mode:?} msg=0x{msg:04x} lparam={lparam} dropped={drop_size}"),
    );
    record_geometry(hwnd, "message-entry");
    drop_size
}

pub(super) fn record_geometry_after_message(hwnd: HWND, msg: u32) {
    if is_geometry_message(msg) {
        record_geometry(hwnd, "message-return");
    }
}

pub(super) fn record_geometry(hwnd: HWND, stage: &str) {
    if mode().is_none() {
        return;
    }
    let mut outer = RECT::default();
    let mut client = RECT::default();
    let mut origin = POINT::default();
    unsafe {
        let outer_ok = GetWindowRect(hwnd, &mut outer) != 0;
        let client_ok = GetClientRect(hwnd, &mut client) != 0;
        let origin_ok = ClientToScreen(hwnd, &mut origin) != 0;
        record_startup_event(
            "chrome-client-origin",
            format!(
                "stage={stage} ok={origin_ok} screen=({},{})",
                origin.x, origin.y
            ),
        );
        record_startup_event(
            "chrome-geometry",
            format!(
                "stage={stage} outer_ok={outer_ok} client_ok={client_ok} outer=({},{},{},{}) client=({},{},{},{}) dpi={}",
                outer.left,
                outer.top,
                outer.right,
                outer.bottom,
                client.left,
                client.top,
                client.right,
                client.bottom,
                GetDpiForWindow(hwnd)
            ),
        );
    }
}

pub(crate) fn tick(ctx: &egui::Context, frame: &eframe::Frame) {
    let Some(selected_mode) = mode() else {
        return;
    };
    static START: OnceLock<Instant> = OnceLock::new();
    static LAST_BUCKET: AtomicU64 = AtomicU64::new(u64::MAX);
    let elapsed = START.get_or_init(Instant::now).elapsed();
    let Some(window) = frame.winit_window() else {
        return;
    };
    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };
    let hwnd = handle.hwnd.get() as HWND;
    let bucket = elapsed.as_millis() as u64 / 250;
    if LAST_BUCKET.swap(bucket, Ordering::Relaxed) != bucket {
        record_geometry(hwnd, "frame");
        record_startup_event(
            "chrome-winit",
            format!(
                "elapsed_ms={} decorated={} inner={:?} scale={} maximized={}",
                elapsed.as_millis(),
                window.is_decorated(),
                window.inner_size(),
                window.scale_factor(),
                window.is_maximized()
            ),
        );
        let pixels_per_point = ctx.pixels_per_point();
        ctx.input(|input| {
            record_startup_event(
                "chrome-egui",
                format!(
                    "elapsed_ms={} content={:?} inner={:?} native_ppp={:?} ppp={}",
                    elapsed.as_millis(),
                    input.content_rect(),
                    input.viewport().inner_rect,
                    input.viewport().native_pixels_per_point,
                    pixels_per_point
                ),
            )
        });
        if let Some(gl) = frame.gl() {
            let mut viewport = [0; 4];
            unsafe { gl.get_parameter_i32_slice(eframe::glow::VIEWPORT, &mut viewport) };
            // logic runs before paint: explicitly label this as the previous GL viewport.
            record_startup_event("chrome-gl", format!("previous_paint_viewport={viewport:?}"));
        }
    }
    if selected_mode == Mode::Observe {
        return; // No refresh, artificial repaint cadence or automatic close.
    }
    #[cfg(debug_assertions)]
    {
        static REFRESHED: AtomicBool = AtomicBool::new(false);
        if elapsed >= Duration::from_secs(3) && !REFRESHED.swap(true, Ordering::Relaxed) {
            record_geometry(hwnd, "before-controlled-refresh");
            refresh_non_client_frame(hwnd);
            record_geometry(hwnd, "after-controlled-refresh");
        }
        if elapsed >= Duration::from_secs(6) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

#[cfg(debug_assertions)]
fn refresh_non_client_frame(hwnd: HWND) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
    };
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}
