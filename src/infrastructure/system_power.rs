//! Power actions used by the one-shot "when the queue finishes" option.

/// Seconds Windows waits (and shows its own notice) before shutting down.
/// `shutdown /a` cancels during this time.
#[cfg(windows)]
const SHUTDOWN_GRACE_SECONDS: u32 = 60;

#[cfg(windows)]
pub fn shutdown_computer() -> Result<(), String> {
    let mut command = std::process::Command::new("shutdown.exe");
    command.args([
        "/s",
        "/t",
        &SHUTDOWN_GRACE_SECONDS.to_string(),
        "/c",
        "yt-dlp-gui: the download queue finished.",
    ]);
    super::configure_background_command(&mut command);
    let status = command
        .status()
        .map_err(|error| format!("Could not start shutdown.exe: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("shutdown.exe failed with {status}."))
    }
}

#[cfg(windows)]
pub fn sleep_computer() -> Result<(), String> {
    use windows_sys::Win32::System::Power::SetSuspendState;
    // SAFETY: SetSuspendState takes plain flags and has no pointer arguments.
    let ok = unsafe { SetSuspendState(false, false, false) };
    if ok {
        Ok(())
    } else {
        Err("Windows did not enter sleep.".to_owned())
    }
}

#[cfg(not(windows))]
pub fn shutdown_computer() -> Result<(), String> {
    Err("Shutting down is only supported on Windows.".to_owned())
}

#[cfg(not(windows))]
pub fn sleep_computer() -> Result<(), String> {
    Err("Sleep is only supported on Windows.".to_owned())
}
