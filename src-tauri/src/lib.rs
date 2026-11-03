//! Tauri shell: wires the pc-core engine to the UI through commands and events.

mod commands;
mod state;

use std::sync::Arc;
use std::time::Instant;

use pc_core::context::{ApiEndpoints, Ctx};
use pc_core::events::{EventSink, Sink};
use pc_core::paths::AppPaths;
use tauri::{Emitter, Manager};
use tracing_subscriber::prelude::*;

use state::AppState;

/// Forwards engine events to the webview and applies the "on game launch" window setting.
struct TauriSink {
    app: tauri::AppHandle,
}

impl EventSink for TauriSink {
    fn emit_json(&self, event: &str, payload: serde_json::Value) {
        if event == "game:state" {
            let st = payload["state"].as_str().unwrap_or("");
            if let Some(state) = self.app.try_state::<AppState>() {
                let behaviour = state.settings.read().unwrap().on_game_launch.clone();
                if let Some(w) = self.app.get_webview_window("main") {
                    match (st, behaviour.as_str()) {
                        ("running", "minimize") => {
                            let _ = w.minimize();
                        }
                        ("running", "hide") => {
                            let _ = w.hide();
                        }
                        ("exited" | "crashed" | "failed", "minimize" | "hide") => {
                            let _ = w.show();
                            let _ = w.unminimize();
                            if st != "exited" {
                                let _ = w.set_focus();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        let _ = self.app.emit(event, payload);
    }
}

fn init_logging(paths: &AppPaths, debug: bool) -> (tracing_subscriber::reload::Handle<tracing_subscriber::filter::LevelFilter, tracing_subscriber::Registry>, tracing_appender::non_blocking::WorkerGuard) {
    let level = if debug { tracing_subscriber::filter::LevelFilter::DEBUG } else { tracing_subscriber::filter::LevelFilter::INFO };
    let (filter, handle) = tracing_subscriber::reload::Layer::new(level);
    let appender = tracing_appender::rolling::daily(paths.logs_dir(), "launcher.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let fmt = tracing_subscriber::fmt::layer().with_ansi(false).with_writer(writer).with_target(false);
    let _ = tracing_subscriber::registry().with(filter).with(fmt.with_filter(tracing_subscriber::filter::filter_fn(|m| {
        // Only our own crates; dependency chatter (hyper, reqwest…) stays out of the log.
        m.target().starts_with("pc_core") || m.target().starts_with("shadow_client")
    }))).try_init();
    // Remove launcher logs older than 14 days (only files this app writes).
    if let Ok(rd) = std::fs::read_dir(paths.logs_dir()) {
        for e in rd.flatten() {
            let old = e.metadata().and_then(|m| m.modified()).map(|t| t.elapsed().map(|d| d.as_secs() > 14 * 86400).unwrap_or(false)).unwrap_or(false);
            if old && e.file_name().to_string_lossy().starts_with("launcher.log") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    (handle, guard)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let started = Instant::now();
    let mut builder = tauri::Builder::default();
    // A dev build with its own data dir may run next to an installed copy; release builds stay single-instance.
    if !(cfg!(debug_assertions) && std::env::var_os("SHADOW_DATA_DIR").is_some()) {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // a share link clicked while Shadow Client is already open arrives here
            if let Some(link) = pc_core::share::link_in_args(args) {
                commands::share_link_arrived(app, link);
            }
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }));
    }
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let paths = AppPaths::resolve();
            paths.ensure()?;
            // shadowclient:// share links open this copy (release builds only; per user, no admin needed)
            #[cfg(not(debug_assertions))]
            if let Ok(exe) = std::env::current_exe() {
                if let Err(e) = pc_core::share::register_scheme(&exe) {
                    eprintln!("couldn't register share links: {e}");
                }
            }
            if let Some(link) = pc_core::share::link_in_args(std::env::args()) {
                commands::share_link_arrived(app.handle(), link);
            }
            let settings = pc_core::settings::load(&paths);
            let (log_handle, guard) = init_logging(&paths, settings.debug_logging);
            tracing::info!("Shadow Client {} starting; data dir {}", pc_core::VERSION, paths.root.display());
            let sink: Sink = Arc::new(TauriSink { app: app.handle().clone() });
            let ua = pc_core::http::build_user_agent(settings.modrinth_contact.as_deref());
            let ctx = Ctx::new(paths, ApiEndpoints::default(), sink, ua)?;
            ctx.downloads.set_limits(settings.download_concurrency, settings.download_retries);
            ctx.http.set_default_ttl(std::time::Duration::from_secs(settings.cache_ttl_minutes as u64 * 60));
            let st = AppState::new(ctx, settings, log_handle, guard, started);
            app.manage(st);
            tracing::info!("backend ready in {} ms", started.elapsed().as_millis());
            Ok(())
        })
        .invoke_handler(commands::handler())
        .run(tauri::generate_context!())
        .expect("error while running Shadow Client");
}
