#[cfg(windows)]
fn main() {
    if let Err(error) = run() {
        let log_path = std::env::args_os().nth(5);
        if let Some(path) = log_path {
            let _ = std::fs::write(path, format!("Updater helper failed: {error}\n"));
        }
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("The detached updater helper is only supported on Windows.");
    std::process::exit(1);
}

#[cfg(windows)]
fn run() -> Result<(), String> {
    use std::path::PathBuf;
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, INFINITE, PROCESS_SYNCHRONIZE,
    };

    let mut args = std::env::args_os().skip(1);
    let installer_path = PathBuf::from(args.next().ok_or("Missing installer path")?);
    let process_id = args
        .next()
        .ok_or("Missing app process ID")?
        .to_string_lossy()
        .parse::<u32>()
        .map_err(|error| format!("Invalid app process ID: {error}"))?;
    let app_executable = PathBuf::from(args.next().ok_or("Missing app executable path")?);
    let ready_path = PathBuf::from(args.next().ok_or("Missing readiness marker path")?);
    let log_path = PathBuf::from(args.next().ok_or("Missing helper log path")?);

    let process_handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, process_id) };
    if process_handle.is_null() {
        return Err(format!(
            "Could not open app process {process_id}: {}",
            std::io::Error::last_os_error()
        ));
    }
    let ready_result = std::fs::write(&ready_path, b"ready");
    if let Err(error) = ready_result {
        unsafe { CloseHandle(process_handle) };
        return Err(format!("Could not signal helper readiness: {error}"));
    }

    let wait_result = unsafe { WaitForSingleObject(process_handle, INFINITE) };
    unsafe { CloseHandle(process_handle) };
    if wait_result != WAIT_OBJECT_0 {
        return Err(format!(
            "Waiting for the app process failed with code {wait_result}."
        ));
    }

    append_log(&log_path, "The app exited; starting the update installer.")?;
    let installer_status = if installer_path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("msi"))
    {
        std::process::Command::new("msiexec.exe")
            .arg("/i")
            .arg(&installer_path)
            .args(["/quiet", "/norestart"])
            .status()
    } else {
        std::process::Command::new(&installer_path)
            .arg("/S")
            .status()
    }
    .map_err(|error| format!("Could not start the installer: {error}"))?;

    let exit_code = installer_status.code().unwrap_or(-1);
    append_log(
        &log_path,
        &format!("Installer exited with code {exit_code}."),
    )?;
    if !installer_status.success() && exit_code != 3010 {
        return Err(format!("Installer failed with exit code {exit_code}."));
    }

    std::process::Command::new(&app_executable)
        .spawn()
        .map_err(|error| format!("Could not relaunch the updated app: {error}"))?;

    let _ = std::fs::remove_file(&installer_path);
    Ok(())
}

#[cfg(windows)]
fn append_log(path: &std::path::Path, message: &str) -> Result<(), String> {
    use std::io::Write;

    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("Could not open updater log: {error}"))?;
    writeln!(log, "{message}").map_err(|error| format!("Could not write updater log: {error}"))
}
