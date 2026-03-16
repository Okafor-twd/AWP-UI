use std::env;
use std::fs;
use std::process::Command;
use std::path::PathBuf;
use std::path::Path;
mod synapsezapi;
use sysinfo::System;
use std::sync::Mutex;
use tauri::{State, Emitter};
use serde::Serialize;

pub struct SysState(pub Mutex<System>);

// -- utilites ------------

#[tauri::command]
fn get_exe_dir() -> String {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_string_lossy()
        .to_string()
}

#[tauri::command]
fn rename_file(old_path: String, new_path: String) -> Result<(), String> {
    std::fs::rename(&old_path, &new_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_file(path: String) -> Result<(), String> {
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}

use serde_json::Value;

#[tauri::command]
fn get_setting_output_redirection() -> bool {
    let settings_path = std::env::current_exe().unwrap()
        .parent().unwrap().to_path_buf()
        .join("settings").join("settings.json");
    let contents = fs::read_to_string(settings_path).unwrap_or_default();
    let json: Value = serde_json::from_str(&contents).unwrap_or(Value::Null);
    json["output_redirection"].as_bool().unwrap_or(false)
}

#[tauri::command]
fn set_setting_output_redirection(value: bool) {
    let settings_path = std::env::current_exe().unwrap()
        .parent().unwrap().to_path_buf()
        .join("settings").join("settings.json");
    let contents = fs::read_to_string(&settings_path).unwrap_or("{}".to_string());
    let mut json: Value = serde_json::from_str(&contents)
        .unwrap_or(Value::Object(Default::default()));
    json["output_redirection"] = Value::Bool(value);
    fs::write(settings_path, json.to_string()).unwrap();
}

#[tauri::command]
fn get_setting_topmost() -> bool {
    let settings_path = std::env::current_exe().unwrap()
        .parent().unwrap().to_path_buf()
        .join("settings").join("settings.json");
    let contents = fs::read_to_string(settings_path).unwrap_or_default();
    let json: Value = serde_json::from_str(&contents).unwrap_or(Value::Null);
    json["topmost"].as_bool().unwrap_or(false)
}

#[tauri::command]
fn set_setting_topmost(value: bool) {
    let settings_path = std::env::current_exe().unwrap()
        .parent().unwrap().to_path_buf()
        .join("settings").join("settings.json");
    let contents = fs::read_to_string(&settings_path).unwrap_or("{}".to_string());
    let mut json: Value = serde_json::from_str(&contents)
        .unwrap_or(Value::Object(Default::default()));
    json["topmost"] = Value::Bool(value);
    fs::write(settings_path, json.to_string()).unwrap();
}

#[tauri::command]
fn execute_script(script: String, pid: u32) {
    synapsezapi::SynapseZAPI2::execute(&script, pid)
}

#[tauri::command]
fn open_synz_folder() {
    let path = PathBuf::from(env::var("LOCALAPPDATA").unwrap_or_default())
        .join("Synapse Z");
    #[cfg(target_os = "windows")]
    { Command::new("explorer").arg(&path).spawn().ok(); }
    #[cfg(target_os = "macos")]
    { Command::new("open").arg(&path).spawn().ok(); }
    #[cfg(target_os = "linux")]
    { Command::new("xdg-open").arg(&path).spawn().ok(); }
}

#[tauri::command]
fn is_synz(pid: u32, sys: State<Mutex<System>>) -> bool {
    let mut sys = sys.lock().unwrap();
    synapsezapi::SynapseZAPI::is_synz(pid, &mut sys)
}

#[tauri::command]
fn create_lnk_folders() -> &'static str {
    let base = PathBuf::from(env::var("LOCALAPPDATA").unwrap_or_default()).join("Synapse Z");
    let autoexec = base.join("autoexec");
    let workspace = base.join("workspace");

    if !autoexec.exists() || !workspace.exists() {
        return "Synapse Z folders not found. Is Synapse Z installed?";
    }

    let exe_dir = get_exe_dir();
    let shortcuts = [(&autoexec, "SynZ-autoexec.lnk"), (&workspace, "SynZ-workspace.lnk")];

    for (target, lnk_name) in &shortcuts {
        let lnk_path = Path::new(&exe_dir).join(lnk_name);
        let target_str = match target.to_str() { Some(s) => s, None => return "Invalid path encoding." };
        let lnk_str   = match lnk_path.to_str() { Some(s) => s, None => return "Invalid .lnk path encoding." };

        let script = format!(
            "$ws = New-Object -ComObject WScript.Shell; \
             $s = $ws.CreateShortcut('{lnk}'); \
             $s.TargetPath = '{target}'; \
             $s.Save()",
            lnk = lnk_str, target = target_str,
        );

        let ok = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .status().map(|s| s.success()).unwrap_or(false);

        if !ok { return "Failed to create .lnk file."; }
    }
    "Created .lnk of workspace and autoexec on your ui directory."
}

#[tauri::command]
fn launch_robloxproc() {
    open::that("roblox-player://").expect("Failed to open URI");
}

#[tauri::command]
fn open_folder(path: String) {
    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer").arg(path).spawn().unwrap();
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(path).spawn().unwrap();
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(path).spawn().unwrap();
}



// -- shared types ------------

#[derive(Serialize, Clone)]
pub struct RbxProcess {
    pub name: String,
    pub pid:  u32,
    pub path: String,
}

#[derive(Serialize, Clone)]
struct SessionPayload {
    pid: u32,
}

#[derive(Serialize, Clone)]
struct SessionOutputPayload {
    pid:         u32,
    output_type: i32,
    output:      String,
}


#[tauri::command]
fn get_roblox_pids(sys: State<Mutex<System>>) -> Vec<RbxProcess> {
    let mut sys = sys.lock().unwrap();
    synapsezapi::SynapseZAPI::get_roblox_processes(&mut sys)
        .into_iter()
        .map(|(pid, path)| RbxProcess {
            name: "RobloxPlayerBeta.exe".to_string(),
            pid,
            path: path.to_string_lossy().to_string(),
        })
        .collect()
}

// -- synapsezapi 2 ------------

#[tauri::command]
fn start_instances_timer(app: tauri::AppHandle) {
    

    let app_added = app.clone();
    synapsezapi::SynapseZAPI2::on_session_added(move |session| {
        let _ = app_added.emit("session_added", SessionPayload { pid: session.pid });
    });

    let app_removed = app.clone();
    synapsezapi::SynapseZAPI2::on_session_removed(move |session| {
        let _ = app_removed.emit("session_removed", SessionPayload { pid: session.pid });
    });

    synapsezapi::SynapseZAPI2::on_session_output(|session, output_type, content| {
        let log_line = format!("[PID {}] type={} content={}\n", session.pid, output_type, content);
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("synz_output.log")
            .and_then(|mut f| std::io::Write::write_all(&mut f, log_line.as_bytes()));
    });

    synapsezapi::SynapseZAPI2::on_session_output(move |session, output_type, output| {
        let _ = app.emit("session_output_debug", SessionOutputPayload {
            pid: session.pid,
            output_type,
            output,
        });
    });

    synapsezapi::SynapseZAPI2::start_instances_timer();
}

#[tauri::command]
fn stop_instances_timer() {
    synapsezapi::SynapseZAPI2::stop_instances_timer();
}

#[tauri::command]
fn get_instances() -> Vec<u32> {
    synapsezapi::SynapseZAPI2::get_instances()
        .keys()
        .cloned()
        .collect()
}


#[tauri::command]
fn get_account_key() -> String {
    synapsezapi::SynapseZAPI::get_account_key()
}



#[tauri::command]
async fn reset_hwid() -> i32 {
    synapsezapi::SynapseZAPI::reset_hwid_async().await
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let sys = Mutex::new(System::new_all());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(sys)
        .invoke_handler(tauri::generate_handler![
            // utility
            get_exe_dir,
            rename_file,
            delete_file,
            open_folder,
            open_synz_folder,
            launch_robloxproc,
            create_lnk_folders,
            get_setting_output_redirection,
            set_setting_output_redirection,
            // settings
            get_setting_topmost,
            set_setting_topmost,
            // process
            get_roblox_pids,
            is_synz,
            // scripting
            execute_script,
            // session lifecycle
            start_instances_timer,
            stop_instances_timer,
            get_instances,
            // account
            get_account_key,
            reset_hwid,
        ])
        .setup(|_app| {
            let exe_path = env::current_exe().expect("GetExecutable Fail.");
            let exe_dir  = exe_path.parent().expect("GetDir Fail.");
            let _ = env::set_current_dir(exe_dir);

            for folder in &["scripts", "settings"] {
                let p = exe_dir.join(folder);
                if !p.exists() { let _ = std::fs::create_dir_all(p); }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}