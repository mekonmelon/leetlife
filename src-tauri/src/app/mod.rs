//! Tauri shell: tray icon, popup window, native notifications, UI push,
//! autostart, and orderly shutdown of the engine's workers.
use crate::clock::SystemClock;
use crate::commands::{self, AppState};
use crate::engine::{Engine, EngineConfig};
use crate::events::AppEvent;
use crate::notifications::{Notification, Notifier};
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_positioner::{Position, WindowExt};
use tokio_util::sync::CancellationToken;

const MAIN: &str = "main";

struct TauriNotifier(AppHandle);

impl Notifier for TauriNotifier {
    fn notify(&self, n: Notification) {
        use tauri_plugin_notification::NotificationExt;
        if let Err(e) = self.0.notification().builder().title(&n.title).body(&n.body).show() {
            tracing::warn!(error = %e, "notification failed");
        }
    }
}

struct Workers {
    cancel: CancellationToken,
    handles: Vec<tokio::task::JoinHandle<()>>,
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "leetlife=info,warn".into()))
        .init();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--autostart"])))
        .plugin(tauri_plugin_positioner::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::refresh_snapshot,
            commands::log_attempt,
            commands::purchase_item,
            commands::set_equipped,
            commands::get_shop,
            commands::get_stats,
            commands::get_recommendations,
            commands::search_problems,
            commands::list_topics,
            commands::get_recent_attempts,
            commands::get_metrics,
            commands::get_metric_history,
            commands::get_settings,
            commands::set_setting,
            commands::set_autostart,
            commands::sync_now,
            commands::get_jobs,
            commands::ui_rendered,
            commands::hide_window,
            commands::quit_app,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let data_dir = app.path().app_data_dir()?;
            let db_path = data_dir.join("leetlife.db");
            tracing::info!(path = %db_path.display(), "opening database");

            let handle = app.handle().clone();
            let cancel = CancellationToken::new();
            let (engine, handles) = tauri::async_runtime::block_on(async {
                let engine = Engine::open(&db_path, Arc::new(SystemClock), EngineConfig::default()).await?;
                engine.bootstrap().await?;
                let notifier: Arc<dyn Notifier> = Arc::new(TauriNotifier(handle.clone()));
                let mut handles = engine.spawn_workers(notifier, cancel.clone());
                handles.push(tokio::spawn(ui_push(engine.clone(), handle.clone(), cancel.clone())));
                let port = engine.ingest_port().await;
                handles.push(tokio::spawn(crate::activity::ingest::serve(engine.clone(), port, cancel.clone())));
                Ok::<_, crate::Error>((engine, handles))
            })?;

            app.manage(AppState { engine, open_started: parking_lot::Mutex::new(None) });
            app.manage(parking_lot::Mutex::new(Some(Workers { cancel, handles })));

            build_tray(app.handle())?;

            if let Some(w) = app.get_webview_window(MAIN) {
                let h = app.handle().clone();
                w.on_window_event(move |e| {
                    if let WindowEvent::Focused(false) = e {
                        if let Some(w) = h.get_webview_window(MAIN) {
                            let _ = w.hide();
                        }
                    }
                });
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building LeetLife");

    app.run(|app, event| {
        if let RunEvent::ExitRequested { .. } | RunEvent::Exit = event {
            shutdown(app);
        }
    });
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let icon = tauri::image::Image::from_bytes(include_bytes!("../../icons/tray.png"))?;
    let open = MenuItem::with_id(app, "open", "Open LeetLife", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit LeetLife", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &PredefinedMenuItem::separator(app)?, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("LeetLife")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id.as_ref() {
            "open" => show_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                toggle_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn toggle_window(app: &AppHandle) {
    let Some(w) = app.get_webview_window(MAIN) else { return };
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
    } else {
        show_window(app);
    }
}

fn show_window(app: &AppHandle) {
    let Some(w) = app.get_webview_window(MAIN) else { return };
    if let Some(state) = app.try_state::<AppState>() {
        state.mark_opening();
    }
    let _ = w.move_window(Position::TrayCenter);
    let _ = w.show();
    let _ = w.set_focus();
    let _ = app.emit("window-shown", ());
}

/// Bus subscriber that pushes committed state to the webview so it never polls.
async fn ui_push(engine: Arc<Engine>, app: AppHandle, cancel: CancellationToken) {
    let mut rx = engine.bus().subscribe();
    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            r = rx.recv() => match r {
                Ok(ev) => {
                    let _ = app.emit("app-event", &ev);
                    if ev.changes_state() {
                        if let Ok(snap) = engine.snapshot().await {
                            let _ = app.emit("state-changed", &*snap);
                        }
                    }
                    if let AppEvent::TaskDue { .. } = ev {
                        // Nudges are also worth surfacing in-window if it's open.
                        let _ = app.emit("nudge", &ev);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => tracing::warn!(missed = n, "ui push lagged"),
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    }
}

fn shutdown(app: &AppHandle) {
    let Some(workers) = app.try_state::<parking_lot::Mutex<Option<Workers>>>() else { return };
    let Some(Workers { cancel, handles }) = workers.lock().take() else { return };
    tracing::info!("shutting down");
    cancel.cancel();
    if let Some(state) = app.try_state::<AppState>() {
        let engine = state.engine.clone();
        tauri::async_runtime::block_on(async move {
            for h in handles {
                let _ = tokio::time::timeout(std::time::Duration::from_secs(3), h).await;
            }
            if let Err(e) = engine.flush_metrics().await {
                tracing::warn!(error = %e, "metrics flush on exit failed");
            }
            engine.db().close().await;
        });
    }
}
