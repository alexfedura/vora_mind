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
        OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
        PROCESS_TERMINATE,
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
    let update_version = args
        .next()
        .ok_or("Missing update version")?
        .to_string_lossy()
        .into_owned();
    let failure_marker_path = PathBuf::from(args.next().ok_or("Missing failure marker path")?);

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

    if let Err(update_error) = update_result {
        let marker_error = write_failure_marker(&failure_marker_path, &update_version);
        log_or_stderr(
            &log_path,
            &format!(
                "Update to {update_version} failed; the app was not relaunched: {update_error}"
            ),
        );
        return match marker_error {
            Ok(()) => Err(update_error),
            Err(marker_error) => Err(format!("{update_error}; {marker_error}")),
        };
    }

    if failure_marker_path.exists() {
        if let Err(error) = std::fs::remove_file(&failure_marker_path) {
            log_or_stderr(
                &log_path,
                &format!("Could not clear the previous update failure marker: {error}"),
            );
        }
    }

    std::process::Command::new(&app_executable)
        .spawn()
        .map_err(|error| format!("Update succeeded, but the app could not be relaunched: {error}"))?;

    let _ = std::fs::remove_file(&installer_path);
    Ok(())
}

#[cfg(windows)]
fn run_installer(
    installer_path: &std::path::Path,
    log_path: &std::path::Path,
) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::process::ExitStatusExt;
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, ERROR_CANCELLED, WAIT_OBJECT_0,
    };
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, WaitForSingleObject, INFINITE,
    };
    use windows_sys::Win32::UI::Shell::{
        ShellExecuteExW, SHELLEXECUTEINFOW, SEE_MASK_NOCLOSEPROCESS,
    };

    let is_msi = installer_path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("msi"));
    let installer_status = if is_msi {
        let msi_log_path = log_path
            .parent()
            .ok_or("Could not determine updater data directory for MSI log")?
            .join("msi_install.log");
        let verb: Vec<u16> = std::ffi::OsStr::new("runas")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let executable: Vec<u16> = std::ffi::OsStr::new("msiexec.exe")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut parameters: Vec<u16> = std::ffi::OsStr::new("/i \"").encode_wide().collect();
        parameters.extend(installer_path.as_os_str().encode_wide());
        parameters.extend(
            std::ffi::OsStr::new("\" /quiet /norestart /L*V \"")
                .encode_wide(),
        );
        parameters.extend(msi_log_path.as_os_str().encode_wide());
        parameters.extend(std::ffi::OsStr::new("\"").encode_wide());
        parameters.push(0);
        let mut execute_info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
        execute_info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        execute_info.fMask = SEE_MASK_NOCLOSEPROCESS;
        execute_info.lpVerb = verb.as_ptr();
        execute_info.lpFile = executable.as_ptr();
        execute_info.lpParameters = parameters.as_ptr();
        execute_info.nShow = 0;

        if unsafe { ShellExecuteExW(&mut execute_info) } == 0 {
            let error_code = unsafe { GetLastError() };
            let error = std::io::Error::from_raw_os_error(error_code as i32);
            return Err(if error_code == ERROR_CANCELLED {
                format!("Administrator approval for MSI installation was cancelled: {error}")
            } else {
                format!("Could not start elevated msiexec.exe: {error}")
            });
        }

        if execute_info.hProcess.is_null() {
            return Err("Elevated msiexec.exe started without a process handle.".to_string());
        }
        let wait_result = unsafe { WaitForSingleObject(execute_info.hProcess, INFINITE) };
        let mut exit_code = 0_u32;
        let exit_code_result = if wait_result == WAIT_OBJECT_0 {
            unsafe { GetExitCodeProcess(execute_info.hProcess, &mut exit_code) }
        } else {
            0
        };
        unsafe { CloseHandle(execute_info.hProcess) };
        if wait_result != WAIT_OBJECT_0 || exit_code_result == 0 {
            return Err(format!(
                "Could not wait for elevated msiexec.exe or read its exit code (wait result {wait_result})."
            ));
        }
        let status = std::process::ExitStatus::from_raw(exit_code);
        status
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
fn write_failure_marker(path: &std::path::Path, version: &str) -> Result<(), String> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let failed_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("Could not determine update failure time: {error}"))?
        .as_millis();
    let marker = serde_json::json!({
        "version": version,
        "failedAt": failed_at,
    });
    let bytes = serde_json::to_vec(&marker)
        .map_err(|error| format!("Could not serialize update failure marker: {error}"))?;
    std::fs::write(path, bytes).map_err(|error| {
        format!(
            "Could not write update failure marker to {}: {error}",
            path.display()
        )
    })
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
