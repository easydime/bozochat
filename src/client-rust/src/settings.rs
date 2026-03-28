//! Phase 5 — Fenêtre Settings
//!
//! Fenêtre native (avec décorations Windows) chargeant settings.html via WebView2.
//! Singleton : une seule instance à la fois.
//!
//! Flux :
//!   1. open() → crée fenêtre + WebView
//!   2. JS envoie 'ready' → Rust appelle inject_config()
//!   3. JS envoie {action:'save', config:{...}} → Rust sauvegarde → notify_save_success()
//!   4. JS envoie 'close' → Rust drop SettingsWindow

use winit::{
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::{Window, Icon},
};
use wry::WebViewBuilder;

use crate::config::Config;
use crate::overlay::OverlayCommand;

// ---------------------------------------------------------------------------
// HTML embarqué
// ---------------------------------------------------------------------------

const SETTINGS_HTML: &str = include_str!("../../client/renderer/settings.html");

// ---------------------------------------------------------------------------
// Icône de fenêtre (même ico que le tray)
// ---------------------------------------------------------------------------

const ICON_BYTES: &[u8] = include_bytes!("../../../bozoicon.ico");

fn load_window_icon() -> Option<Icon> {
    let cursor = std::io::Cursor::new(ICON_BYTES);
    let icon_dir = ico::IconDir::read(cursor).ok()?;
    let entry = icon_dir.entries().iter().max_by_key(|e| e.width())?;
    let image = entry.decode().ok()?;
    Icon::from_rgba(image.rgba_data().to_vec(), image.width(), image.height()).ok()
}

// ---------------------------------------------------------------------------
// SettingsWindow
// ---------------------------------------------------------------------------

/// Etat de la fenêtre settings (conservé dans OverlayApp).
/// Drop = fenêtre détruite automatiquement.
pub struct SettingsWindow {
    pub window: Window,
    pub webview: wry::WebView,
}

/// Ouvre la fenêtre settings.
/// Retourne `None` en cas d'erreur de création.
pub fn open(
    event_loop: &ActiveEventLoop,
    proxy: EventLoopProxy<OverlayCommand>,
) -> Option<SettingsWindow> {
    use winit::dpi::LogicalSize;

    let mut attrs = Window::default_attributes()
        .with_inner_size(LogicalSize::new(640u32, 520u32))
        .with_resizable(false)
        .with_decorations(true)
        .with_title("BozoChat Settings");

    if let Some(icon) = load_window_icon() {
        attrs = attrs.with_window_icon(Some(icon));
    }

    let window = match event_loop.create_window(attrs) {
        Ok(w) => w,
        Err(e) => {
            log::error!("Création fenêtre settings échouée : {}", e);
            return None;
        }
    };

    // Supprimer le bouton maximize via Win32 (with_resizable(false) ne le retire pas toujours)
    #[cfg(target_os = "windows")]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongW, SetWindowLongW, GWL_STYLE, WS_MAXIMIZEBOX,
        };
        if let Ok(handle) = window.window_handle() {
            if let RawWindowHandle::Win32(h) = handle.as_raw() {
                let hwnd = HWND(h.hwnd.get() as *mut core::ffi::c_void);
                unsafe {
                    let style = GetWindowLongW(hwnd, GWL_STYLE);
                    SetWindowLongW(hwnd, GWL_STYLE, style & !(WS_MAXIMIZEBOX.0 as i32));
                }
            }
        }
    }

    // Centrer sur l'écran
    if let Some(monitor) = window.current_monitor() {
        let screen = monitor.size();
        let win = window.outer_size();
        let x = (screen.width.saturating_sub(win.width)) / 2;
        let y = (screen.height.saturating_sub(win.height)) / 2;
        window.set_outer_position(winit::dpi::PhysicalPosition::new(x as i32, y as i32));
    }

    let win_size = window.inner_size();
    let webview = match WebViewBuilder::new()
        .with_bounds(wry::Rect {
            position: winit::dpi::LogicalPosition::new(0, 0).into(),
            size: winit::dpi::PhysicalSize::new(win_size.width, win_size.height).into(),
        })
        .with_html(SETTINGS_HTML)
        .with_ipc_handler(move |request: wry::http::Request<String>| {
            let body = request.body().trim().to_string();
            log::debug!("Settings IPC : '{}'", &body[..body.len().min(80)]);

            if body == "ready" {
                let _ = proxy.send_event(OverlayCommand::SettingsReady);
            } else if body == "close" {
                let _ = proxy.send_event(OverlayCommand::SettingsClose);
            } else if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&body) {
                if msg.get("action").and_then(|v| v.as_str()) == Some("save") {
                    let _ = proxy.send_event(OverlayCommand::SettingsSave(body));
                }
            }
        })
        .build_as_child(&window)
    {
        Ok(wv) => wv,
        Err(e) => {
            log::error!("Création WebView settings échouée : {}", e);
            return None;
        }
    };

    log::info!("Fenêtre settings ouverte");
    Some(SettingsWindow { window, webview })
}

/// Injecte la liste des moniteurs dans le WebView.
/// Appelé avant inject_config sur 'ready'.
pub fn inject_monitors(sw: &SettingsWindow, names: &[String], selected: usize) {
    let json = serde_json::to_string(names).unwrap_or_else(|_| "[]".to_string());
    let script = format!("loadMonitors({}, {})", json, selected);
    if let Err(e) = sw.webview.evaluate_script(&script) {
        log::error!("inject_monitors evaluate_script échoué : {}", e);
    }
}

/// Injecte la config courante dans le WebView via evaluate_script.
/// Appelé quand le JS envoie 'ready'.
pub fn inject_config(sw: &SettingsWindow, config: &Config) {
    let json = match serde_json::to_string(config) {
        Ok(j) => j,
        Err(e) => {
            log::error!("Sérialisation config échouée : {}", e);
            return;
        }
    };
    let script = format!("loadConfig({})", json);
    if let Err(e) = sw.webview.evaluate_script(&script) {
        log::error!("inject_config evaluate_script échoué : {}", e);
    }
}

/// Notifie le WebView que la sauvegarde a réussi.
pub fn notify_save_success(sw: &SettingsWindow) {
    if let Err(e) = sw.webview.evaluate_script("onSaveSuccess()") {
        log::error!("notify_save_success échoué : {}", e);
    }
}
