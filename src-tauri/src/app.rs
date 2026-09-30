use crate::{
    lyrics::{
        providers::{LocalProvider, LrclibProvider},
        resolver::Resolver,
    },
    model::{Lyrics, Playback, SessionInfo},
    playback::{arbitrate, Clock},
    settings::{self, InteractionMode, Settings, Shortcuts, Theme},
};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant,
};
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ViewState {
    revision: u64,
    settings: Settings,
    playback: Option<Playback>,
    sessions: Vec<SessionInfo>,
    lyrics: Lyrics,
    loading: bool,
    track_offset_ms: i64,
    diagnostic: Option<String>,
}
struct Runtime {
    view: ViewState,
    clock: Option<Clock>,
    generation: u64,
}
struct Request {
    track: Playback,
    generation: u64,
    refresh: bool,
}
struct Shared {
    runtime: Mutex<Runtime>,
    settings_gate: Mutex<()>,
    root: PathBuf,
    resolver_tx: tokio::sync::mpsc::UnboundedSender<Request>,
    _log_guard: tracing_appender::non_blocking::WorkerGuard,
}
type AppState = Arc<Shared>;
fn snapshot(shared: &Shared) -> ViewState {
    let mut runtime = shared.runtime.lock().unwrap();
    runtime.view.revision += 1;
    let mut state = runtime.view.clone();
    state.playback = runtime.clock.as_ref().map(Clock::snapshot);
    state
}
fn publish(app: &tauri::AppHandle, shared: &Shared) {
    let _ = app.emit("lyricglass://state", snapshot(shared));
}
fn show_settings(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
fn apply_window(
    app: &tauri::AppHandle,
    settings: &Settings,
    old: Option<&Settings>,
) -> Result<(), String> {
    let w = app
        .get_webview_window("overlay")
        .ok_or("Overlay unavailable")?;
    w.set_ignore_cursor_events(settings.mode == InteractionMode::Ghost)
        .map_err(|e| e.to_string())?;
    w.set_resizable(settings.mode == InteractionMode::Edit)
        .map_err(|e| e.to_string())?;
    if settings.overlay_visible {
        w.show().map_err(|e| e.to_string())?;
    } else {
        w.hide().map_err(|e| e.to_string())?;
    }
    #[cfg(windows)]
    if old.is_none_or(|s| (s.theme.blur > 0.) != (settings.theme.blur > 0.)) {
        let result = if settings.theme.blur > 0. {
            window_vibrancy::apply_blur(&w, Some((10, 20, 18, 25)))
        } else {
            window_vibrancy::clear_blur(&w)
        };
        if let Err(error) = result {
            tracing::warn!(event = "native_blur_unavailable", %error);
        }
    }
    Ok(())
}
fn shortcut_list(s: &Shortcuts) -> Result<Vec<Shortcut>, String> {
    let mut keys = vec![];
    for value in [&s.toggle, &s.earlier, &s.later] {
        let key: Shortcut = value
            .parse()
            .map_err(|e| format!("无效快捷键 {value}: {e}"))?;
        if keys.iter().any(|k: &Shortcut| k.id() == key.id()) {
            return Err("快捷键不能重复".into());
        }
        keys.push(key);
    }
    Ok(keys)
}
fn register_shortcuts(
    app: &tauri::AppHandle,
    s: &Shortcuts,
    old: Option<&Shortcuts>,
) -> Result<(), String> {
    let keys = shortcut_list(s)?;
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;
    for key in keys {
        if let Err(error) = app.global_shortcut().register(key) {
            let _ = app.global_shortcut().unregister_all();
            if let Some(old) = old {
                for key in shortcut_list(old)? {
                    let _ = app.global_shortcut().register(key);
                }
            }
            return Err(format!("无法注册快捷键（可能已被占用）: {error}"));
        }
    }
    Ok(())
}
fn change_settings(
    app: &tauri::AppHandle,
    shared: &Shared,
    change: impl FnOnce(&mut Settings),
) -> Result<(), String> {
    let _gate = shared.settings_gate.lock().unwrap();
    let old = shared.runtime.lock().unwrap().view.settings.clone();
    let mut new = old.clone();
    change(&mut new);
    new.validate()?;
    if new.shortcuts != old.shortcuts {
        register_shortcuts(app, &new.shortcuts, Some(&old.shortcuts))?;
    }
    if let Err(e) = apply_window(app, &new, Some(&old))
        .and_then(|_| settings::write_json(&shared.root.join("settings.json"), &new))
    {
        let _ = apply_window(app, &old, Some(&new));
        if new.shortcuts != old.shortcuts {
            let _ = register_shortcuts(app, &old.shortcuts, None);
        }
        return Err(e);
    }
    {
        let mut runtime = shared.runtime.lock().unwrap();
        if new.shortcuts != old.shortcuts {
            runtime.view.diagnostic = None;
        }
        runtime.view.settings = new;
    }
    publish(app, shared);
    Ok(())
}
fn cycle_mode(app: &tauri::AppHandle, shared: &Shared) -> Result<(), String> {
    change_settings(app, shared, |s| {
        s.mode = match s.mode {
            InteractionMode::Edit => InteractionMode::Locked,
            InteractionMode::Locked => InteractionMode::Ghost,
            InteractionMode::Ghost => InteractionMode::Edit,
        }
    })
}
fn queue_resolution(shared: &Shared, refresh: bool) {
    let mut r = shared.runtime.lock().unwrap();
    if let Some(track) = r.clock.as_ref().map(Clock::snapshot) {
        r.generation += 1;
        r.view.loading = true;
        let _ = shared.resolver_tx.send(Request {
            track,
            generation: r.generation,
            refresh,
        });
    }
}
#[tauri::command]
fn get_state(shared: tauri::State<'_, AppState>) -> ViewState {
    snapshot(&shared)
}
#[tauri::command]
fn save_settings(
    app: tauri::AppHandle,
    shared: tauri::State<'_, AppState>,
    settings: Settings,
) -> Result<(), String> {
    change_settings(&app, &shared, |s| {
        let geometry = s.geometry.clone();
        *s = settings;
        s.geometry = geometry;
    })
}
#[tauri::command]
fn set_mode(
    app: tauri::AppHandle,
    shared: tauri::State<'_, AppState>,
    mode: InteractionMode,
) -> Result<(), String> {
    change_settings(&app, &shared, |s| s.mode = mode)
}
#[tauri::command]
fn open_settings(app: tauri::AppHandle) {
    show_settings(&app);
}
#[tauri::command]
fn refresh_lyrics(app: tauri::AppHandle, shared: tauri::State<'_, AppState>) {
    queue_resolution(&shared, true);
    publish(&app, &shared);
}
#[tauri::command]
fn set_track_offset(app: tauri::AppHandle, shared: tauri::State<'_, AppState>, offset: i64) {
    shared.runtime.lock().unwrap().view.track_offset_ms = offset.clamp(-600000, 600000);
    publish(&app, &shared);
}
#[tauri::command]
fn import_lyrics(
    app: tauri::AppHandle,
    shared: tauri::State<'_, AppState>,
    track_key: String,
    text: String,
) -> Result<(), String> {
    let track = shared
        .runtime
        .lock()
        .unwrap()
        .clock
        .as_ref()
        .map(Clock::snapshot)
        .ok_or("请先播放一首歌")?;
    if track.track_key != track_key {
        return Err("歌曲已切换，请重新选择歌词文件".into());
    }
    LocalProvider {
        root: shared.root.join("local-lyrics"),
    }
    .save(&track, &text)?;
    queue_resolution(&shared, true);
    publish(&app, &shared);
    Ok(())
}
#[tauri::command]
fn remove_local_lyrics(
    app: tauri::AppHandle,
    shared: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let track = shared
        .runtime
        .lock()
        .unwrap()
        .clock
        .as_ref()
        .map(Clock::snapshot)
        .ok_or("请先播放一首歌")?;
    let path = LocalProvider {
        root: shared.root.join("local-lyrics"),
    }
    .path(&track.track_key)?;
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    queue_resolution(&shared, true);
    publish(&app, &shared);
    Ok(())
}
#[tauri::command]
fn list_fonts() -> Vec<String> {
    #[cfg(windows)]
    {
        crate::platform::windows::fonts()
    }
    #[cfg(not(windows))]
    {
        vec!["sans-serif".into()]
    }
}
#[tauri::command]
fn list_themes(shared: tauri::State<'_, AppState>) -> Vec<Theme> {
    let defaults = [
        include_str!("../../themes/glass-mint.json"),
        include_str!("../../themes/paper.json"),
        include_str!("../../themes/midnight.json"),
    ];
    let mut themes: Vec<Theme> = defaults
        .iter()
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect();
    if let Ok(files) = std::fs::read_dir(shared.root.join("themes")) {
        for file in files.flatten() {
            if let Ok(text) = std::fs::read_to_string(file.path()) {
                if let Ok(theme) = serde_json::from_str::<Theme>(&text) {
                    if theme.validate().is_ok() {
                        themes.push(theme);
                    }
                }
            }
        }
    }
    themes
}
#[tauri::command]
fn save_theme(shared: tauri::State<'_, AppState>, theme: Theme) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    theme.validate()?;
    let id = format!("{:x}", Sha256::digest(theme.name.as_bytes()));
    settings::write_json(
        &shared.root.join("themes").join(format!("{id}.json")),
        &theme,
    )
}
#[tauri::command]
async fn export_theme(theme: Theme) -> Result<(), String> {
    theme.validate()?;
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("导出 LyricGlass 主题")
            .add_filter("JSON theme", &["json"])
            .set_file_name("lyricglass-theme.json")
            .save_file()
        {
            std::fs::write(
                path,
                serde_json::to_vec_pretty(&theme).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn reset_overlay_position(
    app: tauri::AppHandle,
    shared: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let w = app
        .get_webview_window("overlay")
        .ok_or("Overlay unavailable")?;
    w.set_size(tauri::LogicalSize::new(860., 230.))
        .map_err(|e| e.to_string())?;
    w.center().map_err(|e| e.to_string())?;
    change_settings(&app, &shared, |s| {
        s.overlay_visible = true;
        s.mode = InteractionMode::Edit;
    })
}
fn tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::TrayIconBuilder,
    };
    let mut items = vec![];
    for (id, label) in [
        ("settings", "打开设置"),
        ("visible", "显示 / 隐藏歌词"),
        ("edit", "编辑 · 拖动与缩放"),
        ("locked", "锁定位置"),
        ("ghost", "穿透鼠标"),
        ("reset", "找回悬浮窗"),
        ("quit", "退出 LyricGlass"),
    ] {
        items.push(MenuItem::with_id(app, id, label, true, None::<&str>)?);
    }
    let refs: Vec<&dyn tauri::menu::IsMenuItem<_>> = items
        .iter()
        .map(|i| i as &dyn tauri::menu::IsMenuItem<_>)
        .collect();
    let menu = Menu::with_items(app, &refs)?;
    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("LyricGlass · 桌面歌词")
        .menu(&menu)
        .on_menu_event(|app, event| {
            let shared = app.state::<AppState>();
            let result = match event.id.as_ref() {
                "settings" => {
                    show_settings(app);
                    Ok(())
                }
                "visible" => {
                    change_settings(app, &shared, |s| s.overlay_visible = !s.overlay_visible)
                }
                "edit" => change_settings(app, &shared, |s| s.mode = InteractionMode::Edit),
                "locked" => change_settings(app, &shared, |s| s.mode = InteractionMode::Locked),
                "ghost" => change_settings(app, &shared, |s| s.mode = InteractionMode::Ghost),
                "reset" => reset_overlay_position(app.clone(), shared.clone()),
                "quit" => {
                    app.exit(0);
                    Ok(())
                }
                _ => Ok(()),
            };
            if let Err(error) = result {
                shared.runtime.lock().unwrap().view.diagnostic = Some(error);
                publish(app, &shared);
            }
        })
        .build(app)?;
    Ok(())
}
fn start_workers(
    app: tauri::AppHandle,
    shared: AppState,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<Request>,
    resolver: Resolver,
) {
    let handle = app.clone();
    let state = shared.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(mut request) = rx.recv().await {
            while let Ok(newer) = rx.try_recv() {
                request = newer;
            }
            let result = resolver.resolve(&request.track, request.refresh).await;
            {
                let mut r = state.runtime.lock().unwrap();
                if r.generation != request.generation {
                    continue;
                }
                r.view.lyrics = result;
                r.view.loading = false;
            }
            publish(&handle, &state);
        }
    });
    #[cfg(windows)]
    std::thread::spawn(move || loop {
        let app_callback = app.clone();
        let shared_callback = shared.clone();
        let result = crate::platform::windows::watch(move |sessions, current| {
            let mut r = shared_callback.runtime.lock().unwrap();
            let active = r.clock.as_ref().map(|c| c.playback.source_id.as_str());
            let chosen = arbitrate(
                &sessions,
                active,
                &r.view.settings.preferred_player,
                current.as_deref(),
            )
            .map(|i| sessions[i].clone());
            let old_source = r.clock.as_ref().map(|c| c.playback.source_id.clone());
            if old_source.as_deref() != chosen.as_ref().map(|p| p.source_id.as_str()) {
                tracing::info!(event = "active_session_changed", source = ?chosen.as_ref().map(|p| &p.source_id));
            }
            let old_key = r.clock.as_ref().map(|c| c.playback.track_key.clone());
            let new_key = chosen.as_ref().map(|p| p.track_key.clone());
            r.view.sessions = sessions
                .iter()
                .map(|s| SessionInfo {
                    id: s.source_id.clone(),
                    app: s.source_app.clone(),
                    title: s.title.clone(),
                    status: s.status.clone(),
                })
                .collect();
            if old_key != new_key {
                r.generation += 1;
                r.view.track_offset_ms = 0;
                r.view.lyrics = Lyrics::unavailable("等待歌词");
                r.view.loading = chosen.is_some();
                if let Some(track) = &chosen {
                    let _ = shared_callback.resolver_tx.send(Request {
                        track: track.clone(),
                        generation: r.generation,
                        refresh: false,
                    });
                }
            }
            r.clock = chosen.map(|p| Clock {
                playback: p,
                received: Instant::now(),
            });
            drop(r);
            publish(&app_callback, &shared_callback);
        });
        if let Err(error) = result {
            tracing::error!(event = "smtc_disconnected", %error);
            {
                let mut r = shared.runtime.lock().unwrap();
                r.generation += 1;
                r.clock = None;
                r.view.sessions.clear();
                r.view.loading = false;
                r.view.lyrics = Lyrics::unavailable("Windows 媒体会话暂不可用，正在重连");
            }
            publish(&app, &shared);
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
    });
}
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_settings(app)
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let Some(shared) = app.try_state::<AppState>() else {
                        return;
                    };
                    let shortcuts = shared
                        .runtime
                        .lock()
                        .unwrap()
                        .view
                        .settings
                        .shortcuts
                        .clone();
                    let Ok(keys) = shortcut_list(&shortcuts) else {
                        return;
                    };
                    if shortcut.id() == keys[0].id() {
                        let _ = cycle_mode(app, &shared);
                    } else {
                        let delta = if shortcut.id() == keys[1].id() {
                            100
                        } else {
                            -100
                        };
                        {
                            let mut r = shared.runtime.lock().unwrap();
                            r.view.track_offset_ms =
                                (r.view.track_offset_ms + delta).clamp(-600000, 600000);
                        }
                        publish(app, &shared);
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_state,
            save_settings,
            set_mode,
            open_settings,
            refresh_lyrics,
            set_track_offset,
            import_lyrics,
            remove_local_lyrics,
            list_fonts,
            list_themes,
            save_theme,
            export_theme,
            reset_overlay_position
        ])
        .setup(|app| {
            let root = app.path().app_data_dir()?;
            std::fs::create_dir_all(&root)?;
            std::fs::create_dir_all(root.join("logs"))?;
            let log = tracing_appender::rolling::Builder::new()
                .rotation(tracing_appender::rolling::Rotation::DAILY)
                .filename_prefix("diagnostic")
                .filename_suffix("jsonl")
                .max_log_files(7)
                .build(root.join("logs"))?;
            let (writer, guard) = tracing_appender::non_blocking(log);
            let _ = tracing_subscriber::fmt()
                .json()
                .with_env_filter("lyricglass_lib=info")
                .with_writer(writer)
                .try_init();
            let settings = settings::load(&root.join("settings.json"));
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            let shared = Arc::new(Shared {
                runtime: Mutex::new(Runtime {
                    view: ViewState {
                        revision: 0,
                        settings: settings.clone(),
                        playback: None,
                        sessions: vec![],
                        lyrics: Lyrics::unavailable("播放音乐后，歌词会出现在这里"),
                        loading: false,
                        track_offset_ms: 0,
                        diagnostic: None,
                    },
                    clock: None,
                    generation: 0,
                }),
                settings_gate: Mutex::new(()),
                resolver_tx: tx,
                root: root.clone(),
                _log_guard: guard,
            });
            app.manage(shared.clone());
            // Webviews can invoke commands as soon as they load. State must exist
            // before creating either window, including during a cold first launch.
            for config in &app.config().app.windows {
                tauri::WebviewWindowBuilder::from_config(app, config)?.build()?;
            }
            if let Err(error) = register_shortcuts(app.handle(), &settings.shortcuts, None) {
                shared.runtime.lock().unwrap().view.diagnostic = Some(error);
            }
            let w = app.get_webview_window("overlay").unwrap();
            w.set_size(tauri::PhysicalSize::new(
                settings.geometry.width,
                settings.geometry.height,
            ))?;
            w.set_position(tauri::PhysicalPosition::new(
                settings.geometry.x,
                settings.geometry.y,
            ))?;
            let g = &settings.geometry;
            let visible = w.available_monitors()?.iter().any(|m| {
                let p = m.position();
                let s = m.size();
                g.x + g.width as i32 > p.x + 80
                    && g.y + g.height as i32 > p.y + 40
                    && g.x < p.x + s.width as i32 - 80
                    && g.y < p.y + s.height as i32 - 40
            });
            if !visible {
                w.center()?;
            }
            apply_window(app.handle(), &settings, None).map_err(std::io::Error::other)?;
            tray(app.handle())?;
            let resolver = Resolver::new(
                root,
                vec![Box::new(
                    LrclibProvider::new().map_err(std::io::Error::other)?,
                )],
            );
            start_workers(app.handle().clone(), shared, rx, resolver);
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
            if window.label() != "overlay" {
                return;
            }
            if matches!(
                event,
                tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)
            ) {
                if let Some(shared) = app.try_state::<AppState>() {
                    if let (Ok(p), Ok(s)) = (window.outer_position(), window.inner_size()) {
                        if s.width >= 300 && s.height >= 100 {
                            let mut r = shared.runtime.lock().unwrap();
                            r.view.settings.geometry = settings::Geometry {
                                x: p.x,
                                y: p.y,
                                width: s.width,
                                height: s.height,
                            };
                            let shared = shared.inner().clone();
                            tauri::async_runtime::spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                let _gate = shared.settings_gate.lock().unwrap();
                                let r = shared.runtime.lock().unwrap();
                                let g = &r.view.settings.geometry;
                                if g.x == p.x
                                    && g.y == p.y
                                    && g.width == s.width
                                    && g.height == s.height
                                {
                                    let _ = settings::write_json(
                                        &shared.root.join("settings.json"),
                                        &r.view.settings,
                                    );
                                }
                            });
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("LyricGlass could not start");
}
