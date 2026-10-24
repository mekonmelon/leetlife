//! Tauri shell: tray icon, popup window, native notifications, UI push,
//! autostart, and orderly shutdown of the engine's workers.
mod tray_art;
#[cfg(target_os = "macos")]
mod status_button;

use crate::clock::SystemClock;
use crate::commands::{self, AppState};
use crate::engine::{Engine, EngineConfig};
use crate::events::AppEvent;
use crate::notifications::{Notification, Notifier};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tokio_util::sync::CancellationToken;

const MAIN: &str = "main";

#[derive(Default)]
struct WindowPresentation {
    opened_once: AtomicBool,
}

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
            commands::place_item,
            commands::set_wallpaper,
            commands::garden_action,
            commands::minimize_window,
            commands::open_home,
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
            commands::backup_database,
            commands::show_save_folder,
            commands::open_problem,
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
                handles.push(tokio::spawn(animate_tray(engine.clone(), handle.clone(), cancel.clone())));
                let port = engine.ingest_port().await;
                handles.push(tokio::spawn(crate::activity::ingest::serve(engine.clone(), port, cancel.clone())));
                Ok::<_, crate::Error>((engine, handles))
            })?;

            app.manage(AppState { engine, open_started: parking_lot::Mutex::new(None) });
            app.manage(WindowPresentation::default());
            app.manage(parking_lot::Mutex::new(Some(Workers { cancel, handles })));

            build_tray(app.handle())?;

            if let Some(window) = app.get_webview_window("summary") {
                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if matches!(event, WindowEvent::Focused(false)) {
                        if let Some(window) = handle.get_webview_window("summary") { let _ = window.hide(); }
                    }
                });
            }
            if let Some(window) = app.get_webview_window(MAIN) {
                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(window) = handle.get_webview_window(MAIN) { let _ = window.hide(); }
                    }
                });
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building LeetLife");

    app.run(|app, event| {
        match event {
            RunEvent::Ready => {
                if !std::env::args().any(|arg| arg == "--autostart") {
                    open_home(app);
                }
            }
            // Finder and `open LeetLife.app` send this to an existing process.
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => open_home(app),
            RunEvent::ExitRequested { .. } | RunEvent::Exit => shutdown(app),
            _ => {}
        }
    });
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let icon = tauri::image::Image::new_owned(tray_art::pixels(0, true), tray_art::WIDTH, tray_art::HEIGHT);
    let open = MenuItem::with_id(app, "open", "Open LeetLife", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit LeetLife", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &PredefinedMenuItem::separator(app)?, &quit])?;

    let tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Susuwatari is dreaming · click for a little check-in")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id.as_ref() {
            "open" => open_home(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                toggle_summary(tray.app_handle());
            }
        })
        .build(app)?;
    #[cfg(target_os = "macos")]
    status_button::install(&tray, app.clone())?;
    Ok(())
}

fn toggle_summary(app: &AppHandle) {
    let Some(window) = app.get_webview_window("summary") else { return };
    if window.is_visible().unwrap_or(false) { let _ = window.hide(); return; }
    if let Some(state) = app.try_state::<AppState>() { state.mark_opening(); }
    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(Some(rect)) = tray.rect() {
            let position = rect.position.to_physical::<f64>(1.0);
            let size = rect.size.to_physical::<f64>(1.0);
            let monitor = window.available_monitors().ok().and_then(|monitors| monitors.into_iter().find(|monitor| {
                let origin = monitor.position();
                let extent = monitor.size();
                position.x >= origin.x as f64 && position.x < origin.x as f64 + extent.width as f64 &&
                position.y >= origin.y as f64 && position.y < origin.y as f64 + extent.height as f64
            })).or_else(|| window.primary_monitor().ok().flatten());
            if let Some(monitor) = monitor {
                let bounds = monitor.position();
                let monitor_size = monitor.size();
                let popup = window.outer_size().unwrap_or(tauri::PhysicalSize::new(340, 440));
                let left = bounds.x as f64 + 8.0;
                let right = (bounds.x as f64 + monitor_size.width as f64 - popup.width as f64 - 8.0).max(left);
                let x = (position.x + size.width / 2.0 - popup.width as f64 / 2.0).clamp(left, right);
                let y = position.y + size.height + 6.0;
                let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
            }
        }
    }
    #[cfg(target_os = "macos")]
    let _ = app.show();
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit_to("summary", "window-shown", ());
}

/// A full visit keeps its last position and can be dragged or minimized.
pub fn open_home(app: &AppHandle) {
    if let Some(summary) = app.get_webview_window("summary") { let _ = summary.hide(); }
    let Some(window) = app.get_webview_window(MAIN) else { return };
    if let Some(state) = app.try_state::<AppState>() { state.mark_opening(); }
    if !app.state::<WindowPresentation>().opened_once.swap(true, Ordering::Relaxed) {
        if let Ok(Some(monitor)) = window.current_monitor() {
            let width = (monitor.size().width as f64 / monitor.scale_factor() - 80.0).clamp(640.0, 900.0);
            let height = (monitor.size().height as f64 / monitor.scale_factor() - 80.0).clamp(600.0, 760.0);
            let _ = window.set_size(tauri::LogicalSize::new(width, height));
        }
        let _ = window.center();
    }
    #[cfg(target_os = "macos")]
    let _ = app.show();
    let _ = window.unminimize();
    if let Err(error) = window.show().and_then(|()| window.set_focus()) {
        tracing::error!(%error, "could not show LeetLife window");
    }
    let _ = app.emit_to(MAIN, "window-shown", ());
    let _ = app.emit_to(MAIN, "home-visited", ());
}

async fn animate_tray(engine: Arc<Engine>, app: AppHandle, cancel: CancellationToken) {
    use chrono::Timelike;
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(1400));
    let mut frame = 0u8;
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {
                let hour = chrono::Local::now().hour();
                let sleepy = engine.snapshot().await.map(|s| s.avatar.energy < 35 || hour >= 20 || hour < 7).unwrap_or(true);
                if let Some(tray) = app.tray_by_id("main") {
                    let image = tauri::image::Image::new_owned(tray_art::pixels(frame, sleepy), tray_art::WIDTH, tray_art::HEIGHT);
                    let _ = tray.set_icon_with_as_template(Some(image), true);
                    let _ = tray.set_tooltip(Some(if sleepy { "Susuwatari is dreaming · click for a little check-in" } else { "Susuwatari is keeping you company · click for a little check-in" }));
                }
                frame = (frame + 1) % 4;
            }
        }
    }
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
