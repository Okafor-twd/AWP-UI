// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use std::env;
use std::fs;
use std::process::Command;
use std::path::PathBuf;
mod synapsezapi;
use sysinfo::{System};
use std::sync::Mutex;
use tauri::State;
use serde::Serialize;
pub struct SysState(pub Mutex<System>);
#[tauri::command]
fn get_exe_dir() -> String {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_string_lossy()
        .to_string()
}
use serde_json::Value;

#[tauri::command]
fn get_setting_topmost() -> bool {
    let exe_dir = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let settings_path = exe_dir.join("settings").join("settings.json");

    let contents = fs::read_to_string(settings_path).unwrap_or_default();
    let json: Value = serde_json::from_str(&contents).unwrap_or(Value::Null);

    json["topmost"].as_bool().unwrap_or(false)
}


#[tauri::command]
fn set_setting_topmost(value: bool) {
    let exe_dir = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

     let settings_path = exe_dir.join("settings").join("settings.json");

    let contents = fs::read_to_string(&settings_path).unwrap_or("{}".to_string());
    let mut json: Value = serde_json::from_str(&contents).unwrap_or(Value::Object(Default::default()));

    json["topmost"] = Value::Bool(value);

    fs::write(settings_path, json.to_string()).unwrap();
}

#[tauri::command]
fn execute_script(script: String, pid: u32) -> u8 {
    synapsezapi::execute(&script, pid)
}

#[tauri::command]
fn open_synz_folder() {
    let path = PathBuf::from(env::var("LOCALAPPDATA").unwrap_or_default())
        .join("Synapse Z");

    
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&path)
            .spawn()
            .ok();
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .ok();
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .ok();
    }
}

#[tauri::command]
fn is_synz(pid: u32) -> bool {
   return synapsezapi::is_synz(pid)
}

#[tauri::command]
fn launch_robloxproc() {
    let local_app_data = match env::var("LOCALAPPDATA") {
        Ok(path) => path,
        Err(_) => {
            eprintln!("Could not find LOCALAPPDATA environment variable.");
            return;
        }
    };

    let versions_path = PathBuf::from(local_app_data)
        .join("Roblox")
        .join("versions");

    let mut entries = match fs::read_dir(&versions_path) {
        Ok(read) => read.filter_map(|e| e.ok()).collect::<Vec<_>>(),
        Err(e) => {
            eprintln!("Failed to read Versions directory: {}", e);
            return;
        }
    };
    entries.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());

    if let Some(latest_version) = entries.last() {
        let exe_path = latest_version.path().join("RobloxPlayerBeta.exe");

        if exe_path.exists() {
            match Command::new(&exe_path).spawn() {
                Ok(child) => println!("Launched Roblox with PID: {}", child.id()),
                Err(e) => eprintln!("Failed to launch Roblox: {}", e),
            }
        } else {
            eprintln!("Found version folder but RobloxPlayerBeta.exe was missing.");
        }
    } else {
        eprintln!("No version folders found in {:?}", versions_path);
    }
}

#[derive(Serialize)]
pub struct RbxProcess {
    pub name: String,
    pub pid: u32,
    pub path: String,
}


#[tauri::command]
fn get_roblox_pids() -> Vec<RbxProcess> {
   let mut sys = sysinfo::System::new_all();
    sys.refresh_processes();

    sys.processes()
        .values()
        .filter(|p| p.name() == "RobloxPlayerBeta.exe")
        .map(|p| RbxProcess {
            name: p.name().to_string(),
            pid: p.pid().as_u32(),
            path: p.exe().unwrap().to_string_lossy().to_string(),
        })
        .collect()
}

#[tauri::command]
fn open_folder(path: String) {
    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer")
        .arg(path)
        .spawn()
        .unwrap();

    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg(path)
        .spawn()
        .unwrap();

    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open")
        .arg(path)
        .spawn()
        .unwrap();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            get_exe_dir, open_folder, launch_robloxproc, get_roblox_pids, execute_script,
            is_synz, open_synz_folder, get_setting_topmost, set_setting_topmost])
        .setup(|_app| {
            let exe_path = env::current_exe().expect("GetExecutable Fail.");
            let exe_dir = exe_path.parent().expect("GetDir Fail.");
            let _ = env::set_current_dir(exe_dir);

            let folders = ["workspace", "scripts", "autoexec", "settings"];
            for folder in &folders {
                let folder_path = exe_dir.join(folder);
                if !folder_path.exists() {
                    let _ = std::fs::create_dir_all(folder_path);
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}