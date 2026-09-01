// Tauri editor GUI for the Midi Fighter key-mapper. This is the "dumb adapter"
// bridge: every command delegates to the pure core (config/edit) or to the
// engine's device layer. No mapping logic lives here.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;

use midifighter_keymapper_core::config::{self, Config, PadBinding};
use midifighter_keymapper_core::edit;
use midifighter_keymapper_core::midi::{self, Color};
use midifighter_keymapper_engine::device;
use midifighter_keymapper_engine::run::{self, EngineHandle};

struct AppState {
    config: Mutex<Config>,
    path: Option<PathBuf>,
    engine: Mutex<Option<EngineHandle>>,
}

fn snapshot(state: &AppState) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> Config {
    snapshot(&state)
}

#[tauri::command]
fn add_profile(state: tauri::State<AppState>, id: String, name: String) -> Result<Config, String> {
    let mut cfg = state.config.lock().unwrap();
    edit::add_profile(&mut cfg, &id, &name).map_err(|e| format!("{e:?}"))?;
    Ok(cfg.clone())
}

#[tauri::command]
fn delete_profile(state: tauri::State<AppState>, id: String) -> Result<Config, String> {
    let mut cfg = state.config.lock().unwrap();
    edit::delete_profile(&mut cfg, &id).map_err(|e| format!("{e:?}"))?;
    Ok(cfg.clone())
}

#[tauri::command]
fn rename_profile(
    state: tauri::State<AppState>,
    id: String,
    name: String,
) -> Result<Config, String> {
    let mut cfg = state.config.lock().unwrap();
    edit::rename_profile(&mut cfg, &id, &name).map_err(|e| format!("{e:?}"))?;
    Ok(cfg.clone())
}

#[tauri::command]
fn set_active(state: tauri::State<AppState>, id: String) -> Result<Config, String> {
    let mut cfg = state.config.lock().unwrap();
    edit::set_active(&mut cfg, &id).map_err(|e| format!("{e:?}"))?;
    Ok(cfg.clone())
}

#[tauri::command]
fn upsert_binding(
    state: tauri::State<AppState>,
    profile_id: String,
    pad: PadBinding,
) -> Result<Config, String> {
    let mut cfg = state.config.lock().unwrap();
    edit::upsert_binding(&mut cfg, &profile_id, pad).map_err(|e| format!("{e:?}"))?;
    Ok(cfg.clone())
}

#[tauri::command]
fn remove_binding(
    state: tauri::State<AppState>,
    profile_id: String,
    bank: u8,
    cell: u8,
) -> Result<Config, String> {
    let mut cfg = state.config.lock().unwrap();
    edit::remove_binding(&mut cfg, &profile_id, bank, cell).map_err(|e| format!("{e:?}"))?;
    Ok(cfg.clone())
}

/// Persist the current config to disk after validating it.
#[tauri::command]
fn save_config(state: tauri::State<AppState>) -> Result<(), String> {
    let cfg = state.config.lock().unwrap();
    config::validate(&cfg).map_err(|errs| format!("invalid config: {errs:?}"))?;
    let path = state
        .path
        .clone()
        .ok_or_else(|| "no config path available".to_string())?;
    config::save(&cfg, &path).map_err(|e| e.to_string())
}

/// Live-preview a pad color on the device (only valid while the engine is not
/// running, i.e. from the editor).
#[tauri::command]
fn preview_color(base_note: u8, bank: u8, cell: u8, color: u8) -> Result<(), String> {
    let mut out = device::open_output()?;
    out.send(&midi::encode_led(base_note, bank, cell, Color(color)))
        .map_err(|e| e.to_string())
}

/// Start mapping: run the active profile's engine on a background thread. The
/// engine owns the device while running, so color preview is unavailable then.
#[tauri::command]
fn start_engine(state: tauri::State<AppState>) -> Result<(), String> {
    let profile = {
        let cfg = state.config.lock().unwrap();
        let active = cfg
            .active
            .clone()
            .ok_or_else(|| "no active profile to run".to_string())?;
        cfg.profiles
            .iter()
            .find(|p| p.id == active)
            .cloned()
            .ok_or_else(|| "active profile not found".to_string())?
    };
    let mut engine = state.engine.lock().unwrap();
    if let Some(handle) = engine.take() {
        handle.stop();
    }
    *engine = Some(run::spawn(profile)?);
    Ok(())
}

/// Stop mapping.
#[tauri::command]
fn stop_engine(state: tauri::State<AppState>) {
    if let Some(handle) = state.engine.lock().unwrap().take() {
        handle.stop();
    }
}

/// Whether the mapping engine is currently running.
#[tauri::command]
fn engine_running(state: tauri::State<AppState>) -> bool {
    state.engine.lock().unwrap().is_some()
}

fn main() {
    let path = config::default_config_path();
    let config = path
        .as_ref()
        .and_then(|p| config::load(p).ok())
        .unwrap_or_default();

    tauri::Builder::default()
        .manage(AppState {
            config: Mutex::new(config),
            path,
            engine: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            add_profile,
            delete_profile,
            rename_profile,
            set_active,
            upsert_binding,
            remove_binding,
            save_config,
            preview_color,
            start_engine,
            stop_engine,
            engine_running
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
