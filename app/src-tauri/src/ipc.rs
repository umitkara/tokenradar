//! The typed bridge to the frontend: commands, events and the generated
//! `app/src/lib/ipc/bindings.ts` (tauri-specta).
//!
//! Debug builds rewrite the bindings on start. `bindings_are_fresh` fails when the committed
//! file differs from what the Rust types produce, so CI catches a forgotten regeneration.

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, EventTarget, Manager};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_specta::{Builder, Event, collect_commands, collect_events};
use tokenradar_core::calibration::CalibrationStatus;
use tokenradar_core::config::{
    DataConfig, LocalePref, LogLevel, PanelView, ThemePref, WidgetPrefs,
};
use tokenradar_core::events::CoreEvent;
use tokenradar_core::limits::{PanelDetail, PanelSnapshot};
use tokenradar_core::notifier::{self, NotifyPrefs};
use tokenradar_core::statusline::line::{IconSet, Imported, SampleState, StatuslinePrefs};
use tokenradar_core::terminal_font::TargetId;
use tokenradar_core::terminal_font::terminals::FontChange;

use crate::main_window::{self, MainRouteRequested};
use crate::prefs::{
    self, Prefs, PrefsChanged, PrefsView, SettingsChanged, SettingsView, StatuslineKind,
    StatuslinePreset, StatuslinePreview,
};
use crate::privacy::{self, AboutView, Link, StorageView};
use crate::settings::{self, SettingsSection, SettingsSectionRequested};
use crate::sources::{
    self, ClaudeModAction, ClaudeModPlanView, DataSourcesView, PickError, StatuslineAction,
    StatuslineError, StatuslinePlanView,
};
use crate::terminal_font::{
    self, FontError, FontPlanView, RestoreDone, RestorePlanView, TerminalFontView,
};
use crate::tray_pin::{self, TrayPinState};
use crate::updater::{self, UpdateChanged, UpdateView};
use crate::{core_bridge, history, notify, onboarding, panel, tray};
use tokenradar_core::deeplink::MainRoute;
use tokenradar_core::domain::{LimitRef, ProjectId, SessionId};
use tokenradar_core::queries::{
    self, HeatMetric, Heatmap, ImpactRow, Overview, ProjectDetail, ProjectFilter, ProjectPage,
    ProjectSort, SessionDetail, SessionFilter, SessionPage, SessionSort, TimeRange, TimeSeries,
    WindowDetail, WindowFilter, WindowPage,
};

/// Relative to the `app/src-tauri` crate. Release builds never write the bindings.
#[cfg(debug_assertions)]
pub const BINDINGS_PATH: &str = "../src/lib/ipc/bindings.ts";

/// A core event, forwarded to the windows that are visible.
#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct CoreEventEmitted(pub CoreEvent);

/// Notifications were muted or unmuted (tray menu, panel, or the mute running out).
#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct MuteChanged {
    pub muted: bool,
}

/// The commands and events the frontend can use.
pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            get_panel_snapshot,
            get_panel_detail,
            get_calibration_status,
            get_panel_view,
            set_panel_view,
            get_settings,
            set_locale,
            set_theme,
            get_prefs,
            set_notification_prefs,
            set_data_prefs,
            set_log_level,
            set_panel_warm_ttl,
            set_widget_prefs,
            set_statusline_prefs,
            preview_statusline,
            statusline_catalog,
            list_statusline_presets,
            export_statusline,
            import_statusline,
            get_autostart,
            set_autostart,
            send_test_notification,
            get_data_sources,
            plan_statusline,
            plan_statusline_refresh,
            apply_statusline,
            discard_statusline_plan,
            plan_claude_mod,
            apply_claude_mod,
            get_terminal_font,
            install_nerd_font,
            cancel_nerd_font,
            plan_terminal_font,
            apply_terminal_font,
            plan_font_restore,
            apply_font_restore,
            set_api_consent,
            set_plan_fee,
            pick_claude_dir,
            reset_claude_dir,
            get_storage,
            delete_all_data,
            create_diagnostics,
            get_about,
            open_link,
            get_update,
            check_for_update,
            install_update,
            panel_ready,
            panel_resize,
            panel_painted,
            panel_hide,
            refresh_limits,
            list_windows,
            get_window,
            list_sessions,
            get_session,
            get_session_impact,
            list_projects,
            get_project,
            get_timeseries,
            get_heatmap,
            get_overview,
            get_muted,
            toggle_mute,
            open_settings,
            open_main,
            main_ready,
            get_tray_pin,
            pin_tray_icon,
            open_taskbar_settings,
            finish_onboarding
        ])
        .events(collect_events![
            CoreEventEmitted,
            SettingsChanged,
            PrefsChanged,
            MuteChanged,
            SettingsSectionRequested,
            MainRouteRequested,
            UpdateChanged,
            terminal_font::FontInstallProgress,
            panel::PanelShown
        ])
        // Ids, timestamps and money are `i64` but stay far below 2^53 (Step 03).
        .dangerously_cast_bigints_to_number()
        // The frontend only receives: one shape per type instead of serialize/deserialize twins.
        .disable_serde_phases()
}

/// Writes the TypeScript bindings to `path`.
#[cfg(debug_assertions)]
pub fn export(builder: &Builder<tauri::Wry>, path: &str) -> Result<(), String> {
    builder
        .export(typescript(), path)
        .map_err(|e| e.to_string())
}

#[cfg(debug_assertions)]
fn typescript() -> specta_typescript::Typescript {
    specta_typescript::Typescript::default()
        .header("// Generated by `tokenradar-app` (src/ipc.rs) in debug builds. Do not edit.\n")
}

/// Sends `event` to visible windows only. Without one, nothing is serialized: a hidden panel
/// fetches a fresh snapshot when it is shown again.
pub fn emit_core_event(app: &AppHandle, event: CoreEvent) {
    emit_to_visible(app, &CoreEventEmitted(event));
}

/// Sends `event` to the visible windows; a hidden page reloads its state when shown.
pub fn emit_to_visible<E: Event + Serialize + Clone>(app: &AppHandle, event: &E) {
    let visible: Vec<String> = app
        .webview_windows()
        .into_iter()
        .filter(|(_, w)| w.is_visible().unwrap_or(false))
        .map(|(label, _)| label)
        .collect();
    if visible.is_empty() {
        return;
    }
    let _ = event.emit_filter(app, |target| match target {
        EventTarget::WebviewWindow { label } | EventTarget::Webview { label } => {
            visible.iter().any(|v| v == label)
        }
        _ => false,
    });
}

/// The current limits, sessions and data-source state; `None` until the core is running.
#[tauri::command]
#[specta::specta]
async fn get_panel_snapshot(app: AppHandle) -> Option<PanelSnapshot> {
    core_bridge::panel_snapshot(&app)
}

/// The detail view's pace, time strip and chart (ADR-048); `None` until the core is running.
#[tauri::command]
#[specta::specta]
async fn get_panel_detail(app: AppHandle) -> Option<PanelDetail> {
    core_bridge::panel_detail(&app)
}

/// Every account's latest calibration (Step 13); empty until the core is running or before
/// the first one.
#[tauri::command]
#[specta::specta]
async fn get_calibration_status(app: AppHandle) -> Vec<CalibrationStatus> {
    core_bridge::calibration_status(&app)
}

/// The panel view to open on: the one last chosen.
#[tauri::command]
#[specta::specta]
async fn get_panel_view(app: AppHandle) -> PanelView {
    app.state::<Prefs>().panel_view()
}

#[tauri::command]
#[specta::specta]
async fn set_panel_view(app: AppHandle, view: PanelView) -> Result<(), String> {
    prefs::set_panel_view(&app, view)
}

#[tauri::command]
#[specta::specta]
async fn get_settings(app: AppHandle) -> SettingsView {
    app.state::<Prefs>().view()
}

#[tauri::command]
#[specta::specta]
async fn set_locale(app: AppHandle, locale: LocalePref) -> Result<SettingsView, String> {
    prefs::set_locale(&app, locale)
}

#[tauri::command]
#[specta::specta]
async fn set_theme(app: AppHandle, theme: ThemePref) -> Result<SettingsView, String> {
    prefs::set_theme(&app, theme)
}

/// Notifications, data, log level and panel timing, for the settings window.
#[tauri::command]
#[specta::specta]
async fn get_prefs(app: AppHandle) -> PrefsView {
    app.state::<Prefs>().prefs_view()
}

#[tauri::command]
#[specta::specta]
async fn set_notification_prefs(app: AppHandle, prefs: NotifyPrefs) -> Result<PrefsView, String> {
    prefs::set_notifications(&app, prefs)
}

#[tauri::command]
#[specta::specta]
async fn set_data_prefs(app: AppHandle, data: DataConfig) -> Result<PrefsView, String> {
    prefs::set_data(&app, data)
}

#[tauri::command]
#[specta::specta]
async fn set_log_level(app: AppHandle, level: LogLevel) -> Result<PrefsView, String> {
    prefs::set_log_level(&app, level)
}

#[tauri::command]
#[specta::specta]
async fn set_panel_warm_ttl(app: AppHandle, seconds: u32) -> Result<PrefsView, String> {
    prefs::set_panel_warm_ttl(&app, seconds)
}

/// Stores the widget settings and starts, updates or stops it.
#[tauri::command]
#[specta::specta]
async fn set_widget_prefs(app: AppHandle, prefs: WidgetPrefs) -> Result<PrefsView, String> {
    prefs::set_widget(&app, prefs)
}

#[tauri::command]
#[specta::specta]
async fn set_statusline_prefs(app: AppHandle, prefs: StatuslinePrefs) -> Result<PrefsView, String> {
    prefs::set_statusline(&app, prefs)
}

/// The statusline drawn with `prefs` on sample values in `state`, `width` columns wide, as
/// painted spans per line.
#[tauri::command]
#[specta::specta]
async fn preview_statusline(
    app: AppHandle,
    prefs: StatuslinePrefs,
    state: SampleState,
    width: Option<u16>,
) -> StatuslinePreview {
    prefs::statusline_preview(&app, &prefs, state, width)
}

/// Every kind of statusline item, for the add-item palette and the item editor; labels and
/// samples in icon set `icons` (Step 12p).
#[tauri::command]
#[specta::specta]
async fn statusline_catalog(app: AppHandle, icons: IconSet) -> Vec<StatuslineKind> {
    prefs::statusline_catalog(&app, icons)
}

/// The built-in statusline presets.
#[tauri::command]
#[specta::specta]
async fn list_statusline_presets() -> Vec<StatuslinePreset> {
    prefs::statusline_presets()
}

/// `prefs` as a `[statusline]` TOML table to share.
#[tauri::command]
#[specta::specta]
async fn export_statusline(prefs: StatuslinePrefs) -> String {
    prefs.normalized().export_toml()
}

/// Reads a shared `[statusline]` table without saving it; the error is the TOML parser's.
#[tauri::command]
#[specta::specta]
async fn import_statusline(text: String) -> Result<Imported, String> {
    StatuslinePrefs::import_toml(&text).map_err(|e| e.message().to_owned())
}

/// Whether the app starts with Windows. The OS entry is the truth, not `config.toml`: the
/// user can also turn it off in Task Manager.
#[tauri::command]
#[specta::specta]
async fn get_autostart(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

/// Turns starting with Windows on or off; answers with the state the OS now reports.
#[tauri::command]
#[specta::specta]
async fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    if let Err(error) = result {
        tracing::warn!(%error, enabled, "could not change autostart");
        return Err(error.to_string());
    }
    manager.is_enabled().map_err(|e| e.to_string())
}

/// "Send test notification": shows one toast at once, ignoring mute and quiet hours.
#[tauri::command]
#[specta::specta]
async fn send_test_notification(app: AppHandle) {
    let locale = app.state::<Prefs>().locale();
    notify::show(&app, &notifier::test_notice(locale));
}

/// Limit windows, newest first, one keyset page after `cursor` (Step 14b).
#[tauri::command]
#[specta::specta]
async fn list_windows(
    app: AppHandle,
    filter: WindowFilter,
    cursor: Option<String>,
    limit: Option<u32>,
) -> Result<WindowPage, String> {
    blocking(move || {
        history::run(&app, |q, ctx| {
            queries::list_windows(&q.store, ctx, &filter, cursor.as_deref(), limit)
        })
    })
    .await
}

/// One window with its session shares and readings; attributes it first when stale.
#[tauri::command]
#[specta::specta]
async fn get_window(app: AppHandle, id: i64) -> Result<Option<WindowDetail>, String> {
    blocking(move || history::run(&app, |q, ctx| queries::get_window(&q.store, ctx, id))).await
}

/// Session trees with usage in the filter's range, one keyset page after `cursor`.
#[tauri::command]
#[specta::specta]
async fn list_sessions(
    app: AppHandle,
    filter: SessionFilter,
    sort: SessionSort,
    cursor: Option<String>,
    limit: Option<u32>,
) -> Result<SessionPage, String> {
    blocking(move || {
        history::run(&app, |q, ctx| {
            queries::list_sessions(
                &q.store,
                ctx,
                &filter,
                sort,
                cursor.as_deref(),
                limit,
                &q.live,
            )
        })
    })
    .await
}

/// One session tree over its whole life; a subagent id opens its main session.
#[tauri::command]
#[specta::specta]
async fn get_session(app: AppHandle, id: SessionId) -> Result<Option<SessionDetail>, String> {
    blocking(move || {
        history::run(&app, |q, ctx| {
            queries::get_session(&q.store, ctx, id, &q.live)
        })
    })
    .await
}

/// Every window a session tree has a share in, newest first.
#[tauri::command]
#[specta::specta]
async fn get_session_impact(app: AppHandle, id: SessionId) -> Result<Vec<ImpactRow>, String> {
    blocking(move || {
        history::run(&app, |q, ctx| {
            queries::get_session_impact(&q.store, ctx, id)
        })
    })
    .await
}

/// Projects with usage in the filter's range, one keyset page after `cursor`.
#[tauri::command]
#[specta::specta]
async fn list_projects(
    app: AppHandle,
    filter: ProjectFilter,
    sort: ProjectSort,
    cursor: Option<String>,
    limit: Option<u32>,
) -> Result<ProjectPage, String> {
    blocking(move || {
        history::run(&app, |q, ctx| {
            queries::list_projects(&q.store, ctx, &filter, sort, cursor.as_deref(), limit)
        })
    })
    .await
}

/// One project over all time: weekly trend, model mix, most expensive sessions.
#[tauri::command]
#[specta::specta]
async fn get_project(app: AppHandle, id: ProjectId) -> Result<Option<ProjectDetail>, String> {
    blocking(move || history::run(&app, |q, ctx| queries::get_project(&q.store, ctx, id))).await
}

/// A limit's fused readings in `range`, thinned to `max_points` (at most 1,500), with its
/// windows and session markers.
#[tauri::command]
#[specta::specta]
async fn get_timeseries(
    app: AppHandle,
    limit: LimitRef,
    range: TimeRange,
    max_points: Option<u32>,
) -> Result<TimeSeries, String> {
    blocking(move || {
        history::run(&app, |q, ctx| {
            queries::get_timeseries(&q.store, ctx, &limit, range, max_points)
        })
    })
    .await
}

/// Usage per local week-day and hour in `range`.
#[tauri::command]
#[specta::specta]
async fn get_heatmap(
    app: AppHandle,
    range: TimeRange,
    metric: HeatMetric,
) -> Result<Heatmap, String> {
    blocking(move || {
        history::run(&app, |q, ctx| {
            queries::get_heatmap(&q.store, ctx, range, metric)
        })
    })
    .await
}

/// The overview page: this week against the last, top sessions and projects, plan value.
#[tauri::command]
#[specta::specta]
async fn get_overview(app: AppHandle) -> Result<Overview, String> {
    blocking(move || {
        history::run(&app, |q, ctx| {
            queries::get_overview(&q.store, ctx, q.limits.clone(), q.absent.clone(), &q.plan())
        })
    })
    .await
}

/// Runs blocking work (disk, dialogs, restarting the core) off the async workers.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .unwrap_or_else(|e| panic!("blocking command failed: {e}"))
}

/// Claude Code directories, the statusline bridge and the API consent.
#[tauri::command]
#[specta::specta]
async fn get_data_sources(app: AppHandle) -> Result<DataSourcesView, String> {
    blocking(move || sources::view(&app)).await
}

/// Plans installing or removing the statusline bridge; nothing is written until
/// `apply_statusline` gets the returned token.
#[tauri::command]
#[specta::specta]
async fn plan_statusline(
    app: AppHandle,
    action: StatuslineAction,
) -> Result<StatuslinePlanView, StatuslineError> {
    blocking(move || sources::plan(&app, action)).await
}

/// Plans the statusline's live refresh, in seconds, `None` for off (Step 12r). A `ready`
/// plan goes to `apply_statusline` like an install.
#[tauri::command]
#[specta::specta]
async fn plan_statusline_refresh(
    app: AppHandle,
    seconds: Option<u16>,
) -> Result<StatuslinePlanView, StatuslineError> {
    blocking(move || sources::plan_refresh_change(&app, seconds)).await
}

/// Applies the plan the user confirmed; returns the backup's path.
#[tauri::command]
#[specta::specta]
async fn apply_statusline(app: AppHandle, token: u32) -> Result<Option<String>, StatuslineError> {
    blocking(move || sources::apply(&app, token)).await
}

#[tauri::command]
#[specta::specta]
async fn discard_statusline_plan(app: AppHandle) {
    sources::discard(&app);
}

/// Plans installing or removing the Claude Code mod (ADR-047); nothing is written until
/// `apply_claude_mod` gets the returned token. `discard_statusline_plan` forgets it too.
#[tauri::command]
#[specta::specta]
async fn plan_claude_mod(
    app: AppHandle,
    action: ClaudeModAction,
) -> Result<ClaudeModPlanView, StatuslineError> {
    blocking(move || sources::plan_mod(&app, action)).await
}

/// Applies the mod plan the user confirmed; returns the backup's path.
#[tauri::command]
#[specta::specta]
async fn apply_claude_mod(app: AppHandle, token: u32) -> Result<Option<String>, StatuslineError> {
    blocking(move || sources::apply_mod(&app, token)).await
}

/// The terminal font card (Step 12o): installed Nerd Font, terminals seen, terminal settings.
#[tauri::command]
#[specta::specta]
async fn get_terminal_font(app: AppHandle) -> TerminalFontView {
    blocking(move || terminal_font::view(&app)).await
}

/// Downloads and installs the pinned Nerd Font; the user agreed in the window first.
/// Progress arrives as `FontInstallProgress` events.
#[tauri::command]
#[specta::specta]
async fn install_nerd_font(app: AppHandle) -> Result<TerminalFontView, FontError> {
    blocking(move || terminal_font::install(&app)).await
}

#[tauri::command]
#[specta::specta]
async fn cancel_nerd_font(app: AppHandle) {
    terminal_font::cancel(&app);
}

/// Plans selecting the installed Nerd Font in one terminal, or undoing TokenRadar's last
/// change there; nothing is written until `apply_terminal_font` gets the returned token.
/// `discard_statusline_plan` forgets it.
#[tauri::command]
#[specta::specta]
async fn plan_terminal_font(
    app: AppHandle,
    target: TargetId,
    change: FontChange,
) -> Result<FontPlanView, FontError> {
    blocking(move || terminal_font::plan(&app, target, change)).await
}

/// Applies the font plan the user confirmed; returns the backup's path.
#[tauri::command]
#[specta::specta]
async fn apply_terminal_font(app: AppHandle, token: u32) -> Result<String, FontError> {
    blocking(move || terminal_font::apply(&app, token)).await
}

/// Plans putting everything back as before TokenRadar: the terminal settings it still holds
/// and the font files it installed. Nothing changes until `apply_font_restore` gets the token.
#[tauri::command]
#[specta::specta]
async fn plan_font_restore(app: AppHandle) -> RestorePlanView {
    blocking(move || terminal_font::plan_restore(&app)).await
}

/// Applies the restore the user confirmed.
#[tauri::command]
#[specta::specta]
async fn apply_font_restore(app: AppHandle, token: u32) -> Result<RestoreDone, FontError> {
    blocking(move || terminal_font::apply_restore(&app, token)).await
}

/// The user's explicit yes (or withdrawal) for the usage API, stamped with the text version.
#[tauri::command]
#[specta::specta]
async fn set_api_consent(app: AppHandle, granted: bool) -> Result<DataSourcesView, String> {
    blocking(move || {
        sources::set_api_consent(&app, granted)?;
        sources::view(&app)
    })
    .await
}

/// The plan fee the plan value compares with; `None` or a non-positive value goes back to the
/// embedded plan table.
#[tauri::command]
#[specta::specta]
async fn set_plan_fee(app: AppHandle, fee_usd: Option<f64>) -> Result<DataSourcesView, String> {
    blocking(move || {
        sources::set_plan_fee(fee_usd)?;
        sources::view(&app)
    })
    .await
}

/// Folder picker for the Claude Code directory; `false` when cancelled.
#[tauri::command]
#[specta::specta]
async fn pick_claude_dir(app: AppHandle) -> Result<bool, PickError> {
    blocking(move || sources::pick_claude_dir(&app)).await
}

#[tauri::command]
#[specta::specta]
async fn reset_claude_dir(app: AppHandle) -> Result<(), String> {
    blocking(move || sources::reset_claude_dir(&app)).await
}

#[tauri::command]
#[specta::specta]
async fn get_storage() -> Result<StorageView, String> {
    blocking(privacy::storage).await
}

/// Deletes TokenRadar's own data (never Claude Code's) and starts over.
#[tauri::command]
#[specta::specta]
async fn delete_all_data(app: AppHandle) -> Result<(), String> {
    blocking(move || privacy::delete_all_data(&app)).await
}

/// Save dialog, then the bundle; its path, or `None` when cancelled.
#[tauri::command]
#[specta::specta]
async fn create_diagnostics(app: AppHandle) -> Result<Option<String>, String> {
    blocking(move || privacy::create_diagnostics(&app)).await
}

#[tauri::command]
#[specta::specta]
async fn get_about() -> AboutView {
    privacy::about()
}

#[tauri::command]
#[specta::specta]
async fn open_link(app: AppHandle, link: Link) -> Result<(), String> {
    privacy::open_link(&app, link)
}

#[tauri::command]
#[specta::specta]
async fn get_update(app: AppHandle) -> UpdateView {
    updater::view(&app)
}

/// "Check now": asks GitHub Releases at once, whatever the daily setting says.
#[tauri::command]
#[specta::specta]
async fn check_for_update(app: AppHandle) -> UpdateView {
    updater::check(&app).await
}

/// "Install and restart": on success the app exits and the installer takes over.
#[tauri::command]
#[specta::specta]
async fn install_update(app: AppHandle) -> Result<(), String> {
    updater::install(&app).await
}

/// The panel rendered its first snapshot; `height` is its content height in CSS pixels.
#[tauri::command]
#[specta::specta]
async fn panel_ready(app: AppHandle, height: f64) {
    panel::ready(&app, Some(height));
}

/// The panel's content height changed.
#[tauri::command]
#[specta::specta]
async fn panel_resize(app: AppHandle, height: f64) {
    panel::resize(&app, height);
}

/// The panel painted after becoming visible (latency log only).
#[tauri::command]
#[specta::specta]
async fn panel_painted(app: AppHandle) {
    panel::painted(&app);
}

/// `Esc` in the panel.
#[tauri::command]
#[specta::specta]
async fn panel_hide(app: AppHandle) {
    panel::hide(&app);
}

/// "Refresh": asks the usage API for data older than 60 s (with consent).
#[tauri::command]
#[specta::specta]
async fn refresh_limits(app: AppHandle) {
    core_bridge::panel_opened(&app);
}

#[tauri::command]
#[specta::specta]
async fn get_muted(app: AppHandle) -> bool {
    tray::is_muted(&app)
}

/// Mutes notifications for an hour, or unmutes them; answered with [`MuteChanged`].
#[tauri::command]
#[specta::specta]
async fn toggle_mute(app: AppHandle) {
    tray::send(&app, tray::TrayMsg::ToggleMute);
}

/// Opens the settings window on `section`.
#[tauri::command]
#[specta::specta]
async fn open_settings(app: AppHandle, section: SettingsSection) -> Result<(), String> {
    settings::show(&app, Some(section)).map_err(|e| e.to_string())
}

/// Opens the main window, on `route` when given (the panel's button).
#[tauri::command]
#[specta::specta]
async fn open_main(app: AppHandle, route: Option<MainRoute>) -> Result<(), String> {
    main_window::show(&app, route).map_err(|e| e.to_string())
}

/// The main window's page rendered: show the window.
#[tauri::command]
#[specta::specta]
async fn main_ready(app: AppHandle) {
    main_window::ready(&app);
}

/// Whether the tray icon is on the taskbar or in the overflow flyout (Windows 11).
#[tauri::command]
#[specta::specta]
async fn get_tray_pin() -> TrayPinState {
    blocking(tray_pin::state).await
}

/// "Always show the icon": the user clicked it; answers with the state read back.
#[tauri::command]
#[specta::specta]
async fn pin_tray_icon() -> TrayPinState {
    blocking(tray_pin::pin).await
}

/// The fallback when the icon could not be promoted: the taskbar settings page.
#[tauri::command]
#[specta::specta]
async fn open_taskbar_settings(app: AppHandle) -> Result<(), String> {
    tray_pin::open_taskbar_settings(&app)
}

/// "Finish" in onboarding: closes the window, which records it as done.
#[tauri::command]
#[specta::specta]
async fn finish_onboarding(app: AppHandle) {
    onboarding::finish(&app);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed bindings match the Rust types. Regenerate with `pnpm dev` (debug start) or
    /// `UPDATE_BINDINGS=1 cargo test -p tokenradar-app bindings_are_fresh`.
    #[test]
    fn bindings_are_fresh() {
        let dir = tempfile::tempdir().expect("tempdir");
        let fresh = dir.path().join("bindings.ts");
        export(&builder(), fresh.to_str().expect("utf-8 path")).expect("export");
        let fresh = std::fs::read_to_string(&fresh).expect("read fresh");
        let committed_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(BINDINGS_PATH);
        if std::env::var_os("UPDATE_BINDINGS").is_some() {
            std::fs::write(&committed_path, &fresh).expect("write bindings");
            return;
        }
        let committed = std::fs::read_to_string(&committed_path)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        assert!(
            committed == fresh,
            "app/src/lib/ipc/bindings.ts is stale; run \
             `UPDATE_BINDINGS=1 cargo test -p tokenradar-app bindings_are_fresh`"
        );
    }
}
