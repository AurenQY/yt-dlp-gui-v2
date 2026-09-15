//! Debug-only native frame experiments. Never enabled by normal startup.
use std::sync::{
    OnceLock,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use eframe::egui;
use windows_sys::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindowRect, WM_NCCALCSIZE, WM_SIZE, WM_WINDOWPOSCHANGED,
};

use crate::infrastructure::record_startup_event;
use crate::infrastructure::startup_diagnostics::is_startup_diagnostics_enabled;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Baseline,
    SkipRefresh,
    DropSize,
}

fn parse_mode(value: &str) -> Option<Mode> {
    match value {
        "--chrome-diagnostic=baseline" => Some(Mode::Baseline),
        "--chrome-diagnostic=skip-refresh" => Some(Mode::SkipRefresh),
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
            record_startup_event(
                "chrome-diagnostic",
                format!("mode={mode:?}; refresh at 3s; close at 6s"),
            );
        }
        mode
    })
}

pub(super) fn skip_initial_refresh() -> bool {
    mode() == Some(Mode::SkipRefresh)
}

// Returning true consumes only the first size notification after subclassing.
pub(super) fn observe_message(hwnd: HWND, msg: u32, lparam: LPARAM) -> bool {
    let Some(mode) = mode() else {
        return false;
    };
    if !matches!(msg, WM_SIZE | WM_NCCALCSIZE | WM_WINDOWPOSCHANGED) {
        return false;
    }
    static SIZE_DROPPED: AtomicBool = AtomicBool::new(false);
    let drop_size =
        msg == WM_SIZE && mode == Mode::DropSize && !SIZE_DROPPED.swap(true, Ordering::Relaxed);
    record_startup_event(
        "chrome-message",
        format!("msg=0x{msg:04x} lparam={lparam} dropped={drop_size}"),
    );
    record_geometry(hwnd, "message-entry");
    drop_size
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

pub(crate) fn tick(ctx: &egui::Context) {
    if mode().is_none() {
        return;
    }
    static START: OnceLock<Instant> = OnceLock::new();
    static REFRESHED: AtomicBool = AtomicBool::new(false);
    static LAST_BUCKET: AtomicU64 = AtomicU64::new(u64::MAX);
    let elapsed = START.get_or_init(Instant::now).elapsed();
    let Some(hwnd) = super::windows::find_app_window() else {
        return;
    };
    let bucket = elapsed.as_millis() as u64 / 250;
    if LAST_BUCKET.swap(bucket, Ordering::Relaxed) != bucket {
        record_geometry(hwnd, "frame");
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
    }
    if elapsed >= Duration::from_secs(3) && !REFRESHED.swap(true, Ordering::Relaxed) {
        record_geometry(hwnd, "before-controlled-refresh");
        super::windows::refresh_non_client_frame(hwnd);
        record_geometry(hwnd, "after-controlled-refresh");
    }
    if elapsed >= Duration::from_secs(6) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    ctx.request_repaint_after(Duration::from_millis(100));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_explicit_diagnostic_arguments_select_faults() {
        assert_eq!(
            parse_mode("--chrome-diagnostic=baseline"),
            Some(Mode::Baseline)
        );
        assert_eq!(
            parse_mode("--chrome-diagnostic=skip-refresh"),
            Some(Mode::SkipRefresh)
        );
        assert_eq!(
            parse_mode("--chrome-diagnostic=drop-size"),
            Some(Mode::DropSize)
        );
        for value in [
            "",
            "--startup-diagnostics",
            "--chrome-diagnostic",
            "--chrome-diagnostic=unknown",
        ] {
            assert_eq!(parse_mode(value), None);
        }
    }
}
