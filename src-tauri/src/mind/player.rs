use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::Duration;
use tauri::{Emitter, Manager};

const MPV_IPC_PIPE: &str = r"\\.\pipe\vora-mind-mpv";
static MPV_PROCESS: OnceLock<Mutex<Option<Child>>> = OnceLock::new();

fn process_lock() -> Result<MutexGuard<'static, Option<Child>>, String> {
    MPV_PROCESS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|error| format!("Could not lock the MPV process state: {error}"))
}

fn mpv_exe_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let bundled_path = app
        .path()
        .resolve("binaries/mpv/mpv.exe", tauri::path::BaseDirectory::Resource)
        .ok();
    let nested_bundled_path = app
        .path()
        .resource_dir()
        .ok()
        .map(|path| path.join("resources").join("binaries").join("mpv").join("mpv.exe"));
    let development_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("binaries")
        .join("mpv")
        .join("mpv.exe");

    for candidate in [bundled_path, nested_bundled_path, Some(development_path)]
        .into_iter()
        .flatten()
    {
        if candidate.is_file() {
            return candidate
                .canonicalize()
                .map_err(|error| format!("Could not resolve the MPV runtime path: {error}"));
        }
    }

    Err("The bundled MPV runtime is missing. Reinstall Vora Mind or restore its player resources.".to_string())
}

#[tauri::command]
pub fn player_spawn(app: tauri::AppHandle, window: tauri::Window) -> Result<u32, String> {
    let mut process = process_lock()?;
    if let Some(child) = process.as_mut() {
        match child.try_wait() {
            Ok(None) => return Ok(child.id()),
            Ok(Some(_)) => *process = None,
            Err(error) => return Err(format!("Could not inspect the MPV process: {error}")),
        }
    }

    let executable = mpv_exe_path(&app)?;
    let parent = executable
        .parent()
        .ok_or_else(|| "The bundled MPV executable has no parent directory.".to_string())?;
    let mut command = Command::new(&executable);
    command
        .current_dir(parent)
        .args([
            "--no-config",
            "--idle=yes",
            "--force-window=yes",
            "--keep-open=yes",
            "--hr-seek=yes",
            "--gpu-api=d3d11",
            "--gpu-context=d3d11",
            "--osc=no",
            "--input-default-bindings=no",
            "--input-vo-keyboard=no",
            "--no-save-position-on-quit",
            "--no-resume-playback",
            "--no-terminal",
        ])
        .arg(format!("--input-ipc-server={MPV_IPC_PIPE}"));

    #[cfg(target_os = "windows")]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let handle = window
            .window_handle()
            .map_err(|error| format!("Could not attach MPV to the Vora Mind window: {error}"))?;
        if let RawWindowHandle::Win32(handle) = handle.as_raw() {
            command
                .arg(format!("--wid={}", handle.hwnd.get()))
                .args(["--no-border", "--no-window-dragging"]);
        }
    }

    #[cfg(not(target_os = "windows"))]
    let _ = window;

    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Could not start the bundled MPV player: {error}"))?;
    let process_id = child.id();
    *process = Some(child);
    Ok(process_id)
}

fn open_pipe() -> Result<std::fs::File, String> {
    let mut last_error = String::new();
    for _ in 0..30 {
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(MPV_IPC_PIPE)
        {
            Ok(pipe) => return Ok(pipe),
            Err(error) => {
                last_error = error.to_string();
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
    Err(format!("MPV is not responding on its local control pipe: {last_error}"))
}

fn send_command(command: Vec<Value>) -> Result<Value, String> {
    let mut pipe = open_pipe()?;
    let payload = json!({
        "command": command,
        "request_id": 1
    });
    serde_json::to_writer(&mut pipe, &payload)
        .map_err(|error| format!("Could not encode the MPV command: {error}"))?;
    pipe.write_all(b"\n")
        .and_then(|_| pipe.flush())
        .map_err(|error| format!("Could not send the MPV command: {error}"))?;

    let mut response = String::new();
    BufReader::new(pipe)
        .read_line(&mut response)
        .map_err(|error| format!("Could not read the MPV response: {error}"))?;
    let response: Value = serde_json::from_str(response.trim())
        .map_err(|error| format!("MPV returned an invalid response: {error}"))?;
    if response.get("error").and_then(Value::as_str) != Some("success") {
        let reason = response
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("unknown MPV error");
        return Err(format!("MPV rejected the command: {reason}"));
    }
    Ok(response.get("data").cloned().unwrap_or(Value::Null))
}

fn property(name: &str) -> Result<Value, String> {
    send_command(vec![json!("get_property"), json!(name)])
}

#[tauri::command]
pub fn player_kill() -> Result<(), String> {
    if let Some(mut child) = process_lock()?.take() {
        child
            .kill()
            .map_err(|error| format!("Could not stop MPV: {error}"))?;
        child
            .wait()
            .map_err(|error| format!("Could not wait for MPV to stop: {error}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn player_load(path: String) -> Result<(), String> {
    send_command(vec![json!("loadfile"), json!(path), json!("replace")]).map(|_| ())
}

#[tauri::command]
pub fn player_load_at(path: String, start_seconds: f64) -> Result<(), String> {
    if !start_seconds.is_finite() || start_seconds < 0.0 {
        return Err("Seek position must be finite and non-negative.".to_string());
    }
    send_command(vec![
        json!("loadfile"),
        json!(path),
        json!("replace"),
        json!(format!("start={start_seconds}")),
    ])
    .map(|_| ())
}

#[tauri::command]
pub fn player_play() -> Result<(), String> {
    send_command(vec![json!("set_property"), json!("pause"), json!(false)]).map(|_| ())
}

#[tauri::command]
pub fn player_pause() -> Result<(), String> {
    send_command(vec![json!("set_property"), json!("pause"), json!(true)]).map(|_| ())
}

#[tauri::command]
pub fn player_seek(seconds: f64) -> Result<(), String> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err("Seek position must be finite and non-negative.".to_string());
    }
    send_command(vec![
        json!("seek"),
        json!(seconds),
        json!("absolute+exact"),
    ])
    .map(|_| ())
}

#[tauri::command]
pub fn player_set_speed(speed: f64) -> Result<(), String> {
    if !speed.is_finite() || !(0.25..=4.0).contains(&speed) {
        return Err("Playback speed must be between 0.25x and 4x.".to_string());
    }
    send_command(vec![json!("set_property"), json!("speed"), json!(speed)]).map(|_| ())
}

#[tauri::command]
pub fn player_set_volume(volume: f64) -> Result<(), String> {
    if !volume.is_finite() || !(0.0..=100.0).contains(&volume) {
        return Err("Volume must be between 0 and 100.".to_string());
    }
    send_command(vec![json!("set_property"), json!("volume"), json!(volume)]).map(|_| ())
}

#[tauri::command]
pub fn player_get_position() -> Result<f64, String> {
    property("time-pos")?
        .as_f64()
        .ok_or_else(|| "MPV has no current playback position.".to_string())
}

#[tauri::command]
pub fn player_get_duration() -> Result<f64, String> {
    property("duration")?
        .as_f64()
        .ok_or_else(|| "MPV has not loaded a media duration yet.".to_string())
}

pub fn start_position_polling(app: tauri::AppHandle) {
    thread::spawn(move || {
        let mut poll_count = 0u32;
        loop {
            thread::sleep(Duration::from_millis(200));
            let running = process_lock()
                .map(|mut process| match process.as_mut() {
                    Some(child) => match child.try_wait() {
                        Ok(None) => true,
                        Ok(Some(_)) => {
                            *process = None;
                            false
                        }
                        Err(_) => false,
                    },
                    None => false,
                })
                .unwrap_or(false);
            if !running {
                continue;
            }

            if let Ok(position) = player_get_position() {
                let _ = app.emit("mind-player-position", position);
            }
            poll_count = poll_count.wrapping_add(1);
            if poll_count % 5 == 0 {
                if let Ok(duration) = player_get_duration() {
                    let _ = app.emit("mind-player-duration", duration);
                }
            }
        }
    });
}
