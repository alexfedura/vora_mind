#[cfg(windows)]
fn main() {
    if let Err(error) = run() {
        let log_path = std::env::args_os().nth(5);
        if let Some(path) = log_path {
            if let Err(log_error) = append_log(
                std::path::Path::new(&path),
                &format!("Updater helper failed: {error}"),
            ) {
                eprintln!("Updater helper failed: {error}; could not write log: {log_error}");
            }
        } else {
            eprintln!("Updater helper failed: {error}");
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
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0, WAIT_TIMEOUT};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
    };

    const APP_EXIT_TIMEOUT_MS: u32 = 30_000;
    const TERMINATED_APP_TIMEOUT_MS: u32 = 5_000;

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

    let process_handle =
        unsafe { OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, 0, process_id) };
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

    let update_result = (|| {
        let wait_result = unsafe { WaitForSingleObject(process_handle, APP_EXIT_TIMEOUT_MS) };
        if wait_result == WAIT_TIMEOUT {
            log_or_stderr(
                &log_path,
                &format!(
                    "The app did not exit within {} seconds; terminating process {process_id}.",
                    APP_EXIT_TIMEOUT_MS / 1_000
                ),
            );
            if unsafe { TerminateProcess(process_handle, 1) } == 0 {
                let termination_error = std::io::Error::last_os_error();
                let terminated_wait =
                    unsafe { WaitForSingleObject(process_handle, TERMINATED_APP_TIMEOUT_MS) };
                if terminated_wait != WAIT_OBJECT_0 {
                    return Err(format!(
                        "The app did not exit within {} seconds and could not be terminated: {termination_error}",
                        APP_EXIT_TIMEOUT_MS / 1_000
                    ));
                }
            } else {
                let terminated_wait =
                    unsafe { WaitForSingleObject(process_handle, TERMINATED_APP_TIMEOUT_MS) };
                if terminated_wait != WAIT_OBJECT_0 {
                    return Err(format!(
                        "The app was terminated after the {} second timeout, but process exit could not be confirmed (wait result {terminated_wait}).",
                        APP_EXIT_TIMEOUT_MS / 1_000
                    ));
                }
            }
        } else if wait_result != WAIT_OBJECT_0 {
            return Err(format!(
                "Waiting for the app process failed with code {wait_result}."
            ));
        }

        log_or_stderr(&log_path, "The app exited; starting the update installer.");
        run_installer(&installer_path, &log_path)
    })();

    unsafe { CloseHandle(process_handle) };

    let relaunch_result = std::process::Command::new(&app_executable)
        .spawn()
        .map_err(|error| format!("Could not relaunch the app: {error}"));
    match (update_result, relaunch_result) {
        (Ok(()), Ok(_)) => {}
        (Err(update_error), Ok(_)) => {
            log_or_stderr(
                &log_path,
                &format!("Update failed; the app was relaunched: {update_error}"),
            );
            return Err(update_error);
        }
        (Ok(()), Err(relaunch_error)) => return Err(relaunch_error),
        (Err(update_error), Err(relaunch_error)) => {
            return Err(format!("{update_error}; {relaunch_error}"));
        }
    }

    let _ = std::fs::remove_file(&installer_path);
    Ok(())
}

#[cfg(windows)]
fn run_installer(
    installer_path: &std::path::Path,
    log_path: &std::path::Path,
) -> Result<(), String> {
    let is_msi = installer_path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("msi"));
    let installer_status = if is_msi {
        std::process::Command::new("msiexec.exe")
            .arg("/i")
            .arg(installer_path)
            .args(["/quiet", "/norestart"])
            .status()
            .map_err(|error| format!("Could not start msiexec.exe: {error}"))?
    } else {
        std::process::Command::new(installer_path)
            .arg("/S")
            .status()
            .map_err(|error| format!("Could not start the installer: {error}"))?
    };

    let exit_code = installer_status.code();
    let exit_message = match exit_code {
        Some(code) => format!("Installer exited with code {code}."),
        None => "Installer exited without a process exit code.".to_string(),
    };
    log_or_stderr(log_path, &exit_message);

    if !installer_status.success() && exit_code != Some(3010) {
        return Err(match exit_code {
            Some(code) => format!("Installer failed with exit code {code}."),
            None => "Installer failed without a process exit code.".to_string(),
        });
    }

    Ok(())
}

#[cfg(windows)]
fn log_or_stderr(path: &std::path::Path, message: &str) {
    if let Err(error) = append_log(path, message) {
        eprintln!("{message}; could not write updater log: {error}");
    }
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
