use std::path::{Path, PathBuf};
use std::process::Command;

/// Open a web URL using the operating system's default browser.
///
/// The current eframe build disables its default features (including `links`),
/// so `egui::Context::open_url` only records an output request; there is no
/// enabled egui-winit browser handler to consume it. Reuse the existing native
/// shell opener instead of adding another browser dependency or changing the
/// locked Cargo dependency graph.
pub fn open_external_url(url: &str) -> Result<(), String> {
    let url = url.trim();
    if !(url.starts_with("https://") || url.starts_with("http://")) || url.contains('\0') {
        return Err("Only valid HTTP(S) URLs can be opened".to_owned());
    }

    #[cfg(target_os = "windows")]
    {
        return shell_execute_open(std::ffi::OsStr::new(url));
    }
    #[cfg(target_os = "macos")]
    {
        return Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("Could not open URL: {error}"));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("Could not open URL: {error}"));
    }
    #[cfg(not(any(target_os = "windows", unix)))]
    {
        Err("Opening URLs is not supported on this platform".to_owned())
    }
}

pub fn output_file_exists(path: &str) -> bool {
    output_path(path).is_file()
}

pub fn output_parent_folder_exists(path: &str) -> bool {
    let path = output_path(path);
    output_parent_folder(&path).is_some_and(|folder| folder.is_dir())
}

pub fn open_output_file(path: &str) -> Result<(), String> {
    let path = output_path(path);
    if !path.is_file() {
        return Err(format!("File does not exist: {}", path.display()));
    }

    open_file_with_system(&path)
}

pub fn open_output_folder(path: &str) -> Result<(), String> {
    let path = output_path(path);

    if path.is_file() {
        return open_folder_selecting_file(&path);
    }

    let Some(folder) = output_parent_folder(&path).filter(|folder| folder.is_dir()) else {
        return Err(format!("File location does not exist: {}", path.display()));
    };

    open_folder(&folder)
}

fn output_path(path: &str) -> PathBuf {
    PathBuf::from(path.trim())
}

fn output_parent_folder(path: &Path) -> Option<PathBuf> {
    if path.is_dir() {
        return Some(path.to_path_buf());
    }
    path.parent().map(Path::to_path_buf)
}

#[cfg(target_os = "windows")]
fn open_file_with_system(path: &Path) -> Result<(), String> {
    shell_execute_open(path.as_os_str())
}

#[cfg(target_os = "windows")]
fn shell_execute_open(target: &std::ffi::OsStr) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide_path = target
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            std::ptr::null(),
            wide_path.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;
    if result > 32 {
        Ok(())
    } else {
        Err(format!(
            "Could not open target with the Windows shell (ShellExecuteW error {result})."
        ))
    }
}

#[cfg(target_os = "windows")]
fn open_folder_selecting_file(path: &Path) -> Result<(), String> {
    Command::new("explorer.exe")
        .arg(format!("/select,{}", path.display()))
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open containing folder: {error}"))
}

#[cfg(target_os = "windows")]
fn open_folder(path: &Path) -> Result<(), String> {
    Command::new("explorer.exe")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open folder: {error}"))
}

#[cfg(target_os = "macos")]
fn open_file_with_system(path: &Path) -> Result<(), String> {
    Command::new("open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open file: {error}"))
}

#[cfg(target_os = "macos")]
fn open_folder_selecting_file(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        return open_folder(parent);
    }
    open_file_with_system(path)
}

#[cfg(target_os = "macos")]
fn open_folder(path: &Path) -> Result<(), String> {
    Command::new("open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open folder: {error}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_file_with_system(path: &Path) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open file: {error}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_folder_selecting_file(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        return open_folder(parent);
    }
    open_file_with_system(path)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_folder(path: &Path) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open folder: {error}"))
}
