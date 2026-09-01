// Tauri editor GUI for the Midi Fighter key-mapper. This is the "dumb adapter"
// bridge: every command delegates to the pure core (config/edit) or to the
// engine's device layer. No mapping logic lives here.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

use midifighter_keymapper_core::config::{self, Config, PadBinding, Settings};
use midifighter_keymapper_core::edit;
use midifighter_keymapper_core::midi::{self, Color};
use midifighter_keymapper_core::palette::{self, Swatch};
use midifighter_keymapper_engine::accessibility;
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

fn persist(state: &AppState) -> Result<(), String> {
    let cfg = state.config.lock().unwrap();
    let path = state
        .path
        .clone()
        .ok_or_else(|| "no config path available".to_string())?;
    config::save(&cfg, &path).map_err(|e| e.to_string())
}

/// Start the mapping engine for the active profile. Shared by the command, the
/// tray, and launch auto-start.
fn spawn_engine(state: &AppState) -> Result<(), String> {
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
    {
        let cfg = state.config.lock().unwrap();
        config::validate(&cfg).map_err(|errs| format!("invalid config: {errs:?}"))?;
    }
    persist(&state)
}

/// Live-preview a pad color on the device (only valid while the engine is not
/// running, i.e. from the editor).
#[tauri::command]
fn preview_color(base_note: u8, bank: u8, cell: u8, color: u8) -> Result<(), String> {
    let mut out = device::open_output()?;
    out.send(&midi::encode_led(base_note, bank, cell, Color(color)))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn start_engine(state: tauri::State<AppState>) -> Result<(), String> {
    spawn_engine(&state)
}

#[tauri::command]
fn stop_engine(state: tauri::State<AppState>) {
    if let Some(handle) = state.engine.lock().unwrap().take() {
        handle.stop();
    }
}

#[tauri::command]
fn engine_running(state: tauri::State<AppState>) -> bool {
    state.engine.lock().unwrap().is_some()
}

/// macOS Accessibility trust (D12). `enigo` keystrokes silently fail unless this
/// app is a trusted Accessibility client; the frontend polls this to show/clear
/// the "grant Accessibility" banner. Always `true` off macOS.
#[tauri::command]
fn accessibility_status() -> bool {
    accessibility::is_trusted()
}

/// Trigger the macOS system prompt to grant Accessibility trust, and open the
/// Privacy & Security → Accessibility pane so the user can flip the switch.
/// Returns the trust state at call time (usually still `false` until they do).
/// No-op returning `true` off macOS.
#[tauri::command]
fn request_accessibility() -> bool {
    let trusted = accessibility::request_trust();
    #[cfg(target_os = "macos")]
    if !trusted {
        // Deep-link straight to the Accessibility list. The system prompt above
        // offers this too, but it only appears once per launch, so open it
        // ourselves for repeat clicks.
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
    }
    trusted
}

/// The device color palette (velocity + name + approximate hex).
#[tauri::command]
fn get_palette() -> Vec<Swatch> {
    palette::palette()
}

#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> Settings {
    state.config.lock().unwrap().settings.clone()
}

/// Toggle "start mapping when the app launches" and persist it.
#[tauri::command]
fn set_start_on_launch(state: tauri::State<AppState>, enabled: bool) -> Result<(), String> {
    state.config.lock().unwrap().settings.start_mapping_on_launch = enabled;
    persist(&state)
}

/// Whether the app is registered to launch at OS login.
#[tauri::command]
fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

/// Register/unregister the app to launch at OS login.
#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())
    } else {
        manager.disable().map_err(|e| e.to_string())
    }
}

fn main() {
    let path = config::default_config_path();
    let config = path
        .as_ref()
        .and_then(|p| config::load(p).ok())
        .unwrap_or_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState {
            config: Mutex::new(config),
            path,
            engine: Mutex::new(None),
        })
        .setup(|app| {
            // System tray: show the window, or quit entirely.
            let show = MenuItem::with_id(app, "show", "Show window", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Midi Fighter Key-Mapper")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // Auto-start mapping on launch if the user enabled it.
            let state = app.state::<AppState>();
            let start = state.config.lock().unwrap().settings.start_mapping_on_launch;
            if start {
                if let Err(e) = spawn_engine(state.inner()) {
                    eprintln!("auto-start mapping failed: {e}");
                }
            }
            Ok(())
        })
        // Closing the window hides it to the tray instead of quitting.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
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
            engine_running,
            accessibility_status,
            request_accessibility,
            get_palette,
            get_settings,
            set_start_on_launch,
            get_autostart,
            set_autostart
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
