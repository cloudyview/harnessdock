pub mod commands;
pub mod discover;
pub mod dsh;
pub mod envfile;
pub mod instances;
pub mod model;
pub mod patch;
pub mod ports;
pub mod procs;
pub mod runtimes;
pub mod store;
pub mod templates;
pub mod util;

use commands::AppState;
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = store::Store::data_dir();
    std::fs::create_dir_all(&data_dir).ok();
    let store = store::Store::open(data_dir);
    // GUI apps on macOS/Linux start with a minimal PATH (no nvm/hermes/homebrew
    // dirs), so dsh's own `pnpm` calls would fail. Prepend the dirs of the
    // configured node/pnpm binaries.
    #[cfg(not(windows))]
    {
        let mut extra: Vec<String> = Vec::new();
        for p in [&store.reg.settings.node_path, &store.reg.settings.pnpm_path] {
            if let Some(dir) = std::path::Path::new(p).parent() {
                let d = dir.to_string_lossy().to_string();
                if !d.is_empty() && dir.is_absolute() && !extra.contains(&d) {
                    extra.push(d);
                }
            }
        }
        if !extra.is_empty() {
            let cur = std::env::var("PATH").unwrap_or_default();
            std::env::set_var("PATH", format!("{}:{}", extra.join(":"), cur));
        }
    }
    let state = Arc::new(AppState { store: Mutex::new(store), procs: procs::ProcManager::default() });
    commands::adopt_orphans(&state);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state.clone())
        .setup(move |app| {
            let show = MenuItem::with_id(app, "show", "显示 HarnessDock", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出（实例继续运行）", true, None::<&str>)?;
            let quit_stop = MenuItem::with_id(app, "quit_stop", "退出并停止全部实例", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit, &quit_stop])?;
            let st = state.clone();
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("HarnessDock")
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    "quit_stop" => {
                        stop_all(&st);
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        if let Some(w) = tray.app_handle().get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let st = window.state::<Arc<AppState>>();
                let action = st.store().reg.settings.close_action.clone();
                match action.as_str() {
                    "exit" => {}
                    "exit-stop" => stop_all(&st),
                    _ => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::save_settings,
            commands::instance_start,
            commands::instance_stop,
            commands::instance_logs,
            commands::instance_clear_error,
            commands::instance_create,
            commands::instance_import,
            commands::instance_delete,
            commands::instance_restore,
            commands::instance_purge,
            commands::instance_update,
            commands::instance_set_port,
            commands::instance_set_runtime,
            commands::plugin_add,
            commands::plugin_toggle,
            commands::plugin_remove,
            commands::patch_read,
            commands::patch_write,
            commands::instance_validate,
            commands::model_apply,
            commands::default_model_set,
            commands::provider_upsert,
            commands::provider_delete,
            commands::provider_test,
            commands::env_set,
            commands::env_unset,
            commands::ports_scan,
            commands::process_info,
            commands::process_kill,
            commands::runtime_install,
            commands::runtime_remove,
            commands::runtime_set_default,
            commands::runtime_preview,
            commands::template_capture,
            commands::template_delete,
            commands::template_baseline,
            commands::discover_scan,
            commands::detect_install,
            commands::app_paths,
        ])
        .run(tauri::generate_context!())
        .expect("error while running HarnessDock");
}

fn stop_all(st: &Arc<AppState>) {
    for id in st.procs.running_ids() {
        let log = st.store().log_path(&id);
        let _ = st.procs.stop(&id, &log);
    }
}
