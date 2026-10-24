//! Tauri IPC surface. Thin: parse arguments, call the engine, time the call.
use crate::cache::Snapshot;
use crate::engine::{AttemptOutcome, Engine, LogAttemptInput, SyncReport};
use crate::error::Result;
use crate::metrics::{names, MetricSummary};
use crate::models::*;
use crate::recommendation::{Filter, Recommendation};
use crate::scheduler::ScheduledJob;
use crate::storage::repo::MetricHistoryRow;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;

pub struct AppState {
    pub engine: Arc<Engine>,
    /// Set when the tray is clicked; cleared when the UI reports first paint.
    pub open_started: Mutex<Option<Instant>>,
}

impl AppState {
    pub fn mark_opening(&self) {
        *self.open_started.lock() = Some(Instant::now());
    }
}

#[tauri::command]
pub async fn get_snapshot(state: State<'_, AppState>) -> Result<Arc<Snapshot>> {
    let _t = state.engine.metrics().timer(names::IPC_SNAPSHOT);
    state.engine.snapshot().await
}

#[tauri::command]
pub async fn refresh_snapshot(state: State<'_, AppState>) -> Result<Arc<Snapshot>> {
    state.engine.refresh_snapshot().await
}

#[tauri::command]
pub async fn log_attempt(state: State<'_, AppState>, input: LogAttemptInput) -> Result<AttemptOutcome> {
    state.engine.log_attempt(input).await
}

#[tauri::command]
pub async fn purchase_item(state: State<'_, AppState>, item_id: i64) -> Result<Arc<Snapshot>> {
    state.engine.purchase_item(item_id).await
}

#[tauri::command]
pub async fn set_equipped(state: State<'_, AppState>, item_id: i64, equipped: bool) -> Result<Arc<Snapshot>> {
    state.engine.set_equipped(item_id, equipped).await
}

#[tauri::command]
pub async fn get_shop(state: State<'_, AppState>) -> Result<Vec<ShopItem>> {
    state.engine.shop().await
}

#[tauri::command]
pub async fn get_stats(state: State<'_, AppState>) -> Result<Stats> {
    state.engine.stats().await
}

#[derive(Debug, Default, Deserialize)]
pub struct RecommendArgs {
    pub difficulty: Option<Difficulty>,
    pub topic: Option<String>,
    #[serde(default)]
    pub only_due: bool,
    pub limit: Option<usize>,
}

#[tauri::command]
pub async fn get_recommendations(state: State<'_, AppState>, args: Option<RecommendArgs>) -> Result<Vec<Recommendation>> {
    let a = args.unwrap_or_default();
    let filter = Filter { difficulty: a.difficulty, topic: a.topic, only_due: a.only_due };
    state.engine.recommendations(filter, a.limit.unwrap_or(5).clamp(1, 50)).await
}

#[tauri::command]
pub async fn search_problems(state: State<'_, AppState>, query: String, limit: Option<i64>) -> Result<Vec<ProblemMeta>> {
    state.engine.search_problems(&query, limit.unwrap_or(12).clamp(1, 100)).await
}

#[tauri::command]
pub async fn list_topics(state: State<'_, AppState>) -> Result<Vec<String>> {
    state.engine.topics().await
}

#[tauri::command]
pub async fn get_recent_attempts(state: State<'_, AppState>, limit: Option<i64>) -> Result<Vec<AttemptRow>> {
    state.engine.recent_attempts(limit.unwrap_or(20).clamp(1, 200)).await
}

#[tauri::command]
pub async fn get_metrics(state: State<'_, AppState>) -> Result<Vec<MetricSummary>> {
    Ok(state.engine.metrics_summary())
}

#[tauri::command]
pub async fn get_metric_history(state: State<'_, AppState>, name: String, limit: Option<i64>) -> Result<Vec<MetricHistoryRow>> {
    state.engine.metric_history(&name, limit.unwrap_or(48).clamp(1, 1000)).await
}

#[derive(Debug, Serialize)]
pub struct SettingsView {
    pub values: HashMap<String, String>,
    pub autostart: bool,
    pub ingest_port: u16,
}

#[tauri::command]
pub async fn get_settings(app: AppHandle, state: State<'_, AppState>) -> Result<SettingsView> {
    Ok(SettingsView {
        values: state.engine.settings().await?,
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        ingest_port: state.engine.ingest_port().await,
    })
}

#[tauri::command]
pub async fn set_setting(state: State<'_, AppState>, key: String, value: String) -> Result<()> {
    state.engine.set_setting(&key, &value).await
}

#[tauri::command]
pub async fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool> {
    let al = app.autolaunch();
    let r = if enabled { al.enable() } else { al.disable() };
    r.map_err(|e| crate::Error::Invalid(format!("autostart: {e}")))?;
    Ok(al.is_enabled().unwrap_or(enabled))
}

#[tauri::command]
pub async fn sync_now(state: State<'_, AppState>) -> Result<SyncReport> {
    Ok(state.engine.sync_activity().await)
}

#[tauri::command]
pub async fn get_jobs(state: State<'_, AppState>) -> Result<Vec<ScheduledJob>> {
    Ok(state.engine.scheduler().pending())
}

/// The UI calls this after its first paint of a fresh snapshot, closing the
/// "tray click -> pixels" latency measurement.
#[tauri::command]
pub async fn ui_rendered(state: State<'_, AppState>, render_ms: Option<f64>) -> Result<()> {
    if let Some(start) = state.open_started.lock().take() {
        state.engine.metrics().record(names::UI_OPEN, start.elapsed());
    }
    if let Some(ms) = render_ms {
        state.engine.metrics().record_us(names::UI_RENDER, ms * 1000.0);
    }
    Ok(())
}

#[tauri::command]
pub async fn hide_window(app: AppHandle) -> Result<()> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    Ok(())
}

#[tauri::command]
pub async fn quit_app(app: AppHandle) -> Result<()> {
    app.exit(0);
    Ok(())
}
