//! Phase 3 — Fenêtre overlay transparente always-on-top
//! Phase 4 — Intégration system tray
//!
//! Architecture :
//!   - Tourne sur le thread principal (winit event loop)
//!   - Reçoit les commandes via EventLoopProxy<OverlayCommand> depuis le thread tokio
//!   - Affiche overlay.html dans un WebView2 transparent
//!   - Le JS communique retour via window.ipc.postMessage("hide")

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::{Window, WindowId},
};
use wry::WebViewBuilder;

use crate::config::{self, Config, OverlayPosition};
use crate::settings::{self, SettingsWindow};
use crate::tray::{self, TrayState};
use crate::websocket::NotificationPayload;

// ---------------------------------------------------------------------------
// HTML embarqué à la compilation
// ---------------------------------------------------------------------------

const OVERLAY_HTML: &str = include_str!("../../client/renderer/overlay.html");

// ---------------------------------------------------------------------------
// Commandes tokio → winit (et tray → winit)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum OverlayCommand {
    /// Afficher une notification pendant duration_ms millisecondes.
    Show(NotificationPayload, u64),
    /// Cacher immédiatement.
    Hide,
    /// Quitter l'application.
    Quit,
    /// Mise à jour de l'état de connexion WS (pour le tray).
    ConnectionStatus(bool),
    /// Clic sur un item du menu tray.
    MenuAction(muda::MenuId),
    /// Ouvrir la fenêtre settings (double-clic tray ou menu Settings).
    OpenSettings,
    /// Le JS de la fenêtre settings a fini de charger.
    SettingsReady,
    /// Le JS envoie la config à sauvegarder (JSON brut).
    SettingsSave(String),
    /// Fermer la fenêtre settings.
    SettingsClose,
    /// La vidéo a chargé ses métadonnées — override la durée du timer auto-hide.
    SetDuration(u64),
}

// ---------------------------------------------------------------------------
// Calcul de position
// ---------------------------------------------------------------------------

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const MARGIN: i32 = 40;

/// Calcule la position en pixels logiques (winit gère la conversion DPI via LogicalPosition).
/// `screen_w/h` sont en pixels physiques, `scale` est le scale factor du monitor.
fn compute_position(position: &OverlayPosition, screen_w: u32, screen_h: u32, scale: f64) -> (i32, i32) {
    // Convertit la taille écran en logique pour comparer avec WINDOW_WIDTH/HEIGHT (logique)
    let sw = (screen_w as f64 / scale) as i32;
    let sh = (screen_h as f64 / scale) as i32;
    let ww = WINDOW_WIDTH as i32;
    let wh = WINDOW_HEIGHT as i32;
    match position {
        OverlayPosition::TopLeft     => (MARGIN, MARGIN),
        OverlayPosition::TopRight    => (sw - ww - MARGIN, MARGIN),
        OverlayPosition::BottomLeft  => (MARGIN, sh - wh - MARGIN),
        OverlayPosition::BottomRight => (sw - ww - MARGIN, sh - wh - MARGIN),
        OverlayPosition::Center      => ((sw - ww) / 2, (sh - wh) / 2),
    }
}

// ---------------------------------------------------------------------------
// OverlayApp — ApplicationHandler
// ---------------------------------------------------------------------------

pub struct OverlayApp {
    config: Config,
    /// Proxy partagé : IPC JS→Rust, tray events, et fourni à tray::build().
    ipc_proxy: EventLoopProxy<OverlayCommand>,
    window: Option<Window>,
    webview: Option<wry::WebView>,
    /// État du tray, initialisé dans new_events(StartCause::Init).
    tray: Option<TrayState>,
    /// Fenêtre settings (singleton — None si fermée).
    settings_window: Option<SettingsWindow>,
    /// Génération du timer auto-hide. Incrémenter pour annuler le timer précédent.
    hide_generation: Arc<AtomicU64>,
}

impl OverlayApp {
    pub fn new(config: Config, ipc_proxy: EventLoopProxy<OverlayCommand>) -> Self {
        Self {
            config,
            ipc_proxy,
            window: None,
            webview: None,
            tray: None,
            settings_window: None,
            hide_generation: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Spawn un timer auto-hide tokio avec annulation par génération.
    fn spawn_hide_timer(&self, duration_ms: u64) {
        let gen = self.hide_generation.fetch_add(1, Ordering::Relaxed) + 1;
        let gen_arc = self.hide_generation.clone();
        let proxy = self.ipc_proxy.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(duration_ms));
            // N'envoie Hide que si la génération n'a pas changé
            if gen_arc.load(Ordering::Relaxed) == gen {
                let _ = proxy.send_event(OverlayCommand::Hide);
            }
        });
    }

    /// Applique SetWindowPos HWND_TOPMOST (Windows uniquement).
    fn set_topmost(&self) {
        #[cfg(target_os = "windows")]
        {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::{
                SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
            };

            if let Some(window) = &self.window {
                if let Ok(handle) = window.window_handle() {
                    if let RawWindowHandle::Win32(h) = handle.as_raw() {
                        let hwnd = HWND(h.hwnd.get() as *mut core::ffi::c_void);
                        unsafe {
                            let _ = SetWindowPos(
                                hwnd,
                                Some(HWND_TOPMOST),
                                0, 0, 0, 0,
                                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                            );
                        }
                    }
                }
            }
        }
    }

    /// Envoie showNotification(json) au WebView, affiche la fenêtre et démarre le timer auto-hide.
    fn show_notification(&mut self, payload: &NotificationPayload, duration_ms: u64) {
        let json = match serde_json::to_string(payload) {
            Ok(j) => j,
            Err(e) => {
                log::error!("Sérialisation notification échouée : {}", e);
                return;
            }
        };

        if let Some(webview) = &self.webview {
            let script = format!("showNotification({})", json);
            if let Err(e) = webview.evaluate_script(&script) {
                log::error!("evaluate_script échoué : {}", e);
            }
        }

        // Positionner juste avant set_visible — window.current_monitor() est fiable ici.
        if let Some(window) = &self.window {
            if let Some(monitor) = window.current_monitor() {
                let s = monitor.size();
                let scale = monitor.scale_factor();
                let (x, y) = compute_position(&self.config.overlay_position, s.width, s.height, scale);
                log::info!("Position overlay : x={} y={} (scale={} screen={}x{})", x, y, scale, s.width, s.height);
                window.set_outer_position(winit::dpi::LogicalPosition::new(x, y));
            }
            window.set_visible(true);
        }

        self.set_topmost();
        self.spawn_hide_timer(duration_ms);
        log::info!("Overlay affiché (timer {}ms)", duration_ms);
    }

    /// Cache la fenêtre.
    fn hide_window(&self) {
        if let Some(window) = &self.window {
            window.set_visible(false);
        }
        log::info!("Overlay caché");
    }


}

impl ApplicationHandler<OverlayCommand> for OverlayApp {
    /// Appelé au démarrage de l'event loop (StartCause::Init).
    /// C'est le seul endroit sûr pour créer le tray icon.
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::Init = cause {
            match tray::build(self.ipc_proxy.clone()) {
                Ok(t) => {
                    self.tray = Some(t);
                }
                Err(e) => {
                    log::error!("Initialisation tray échouée : {}", e);
                }
            }
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        use winit::dpi::LogicalSize;

        // ── Attributs de la fenêtre ────────────────────────────────────────
        let mut attrs = Window::default_attributes()
            .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .with_decorations(false)
            .with_transparent(true)
            .with_visible(false)
            .with_resizable(false)
            .with_title("BozoChat Overlay");

        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::WindowAttributesExtWindows;
            attrs = attrs
                .with_skip_taskbar(true)
                // Indispensable pour la transparence WebView2 sur Windows
                .with_no_redirection_bitmap(true);
        }

        let window = match event_loop.create_window(attrs) {
            Ok(w) => w,
            Err(e) => {
                log::error!("Création fenêtre échouée : {}", e);
                event_loop.exit();
                return;
            }
        };

        // Positionnement en pixels logiques — winit convertit selon le DPI du monitor.
        // scale_factor() est fiable ici car la fenêtre est rattachée au monitor principal.
        // Ne pas positionner ici — la fenêtre est invisible et Windows peut ignorer
        // set_outer_position. On positionne dans show_notification() juste avant set_visible(true).

        // ── WebView ────────────────────────────────────────────────────────
        let ipc_proxy = self.ipc_proxy.clone();

        let webview = match WebViewBuilder::new()
            .with_bounds(wry::Rect {
                position: winit::dpi::LogicalPosition::new(0, 0).into(),
                size: winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT).into(),
            })
            .with_html(OVERLAY_HTML)
            .with_transparent(true)
            .with_ipc_handler(move |request: wry::http::Request<String>| {
                let body = request.body().trim().to_string();
                log::debug!("IPC reçu depuis JS : '{}'", body);
                if body == "hide" {
                    let _ = ipc_proxy.send_event(OverlayCommand::Hide);
                } else if let Some(ms_str) = body.strip_prefix("set-duration:") {
                    if let Ok(ms) = ms_str.parse::<u64>() {
                        let _ = ipc_proxy.send_event(OverlayCommand::SetDuration(ms));
                    }
                }
            })
            .build_as_child(&window)
        {
            Ok(wv) => wv,
            Err(e) => {
                log::error!("Création WebView échouée : {}", e);
                event_loop.exit();
                return;
            }
        };

        self.window = Some(window);
        self.webview = Some(webview);

        log::info!("Overlay initialisé (caché, prêt à recevoir des notifications)");
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let WindowEvent::CloseRequested = event {
            // Si c'est la fenêtre settings → la fermer sans quitter l'app
            if let Some(sw) = &self.settings_window {
                if sw.window.id() == window_id {
                    self.settings_window = None;
                    return;
                }
            }
            // Sinon (overlay ou autre) → quitter l'app
            event_loop.exit();
        }
    }

    /// Reçoit toutes les commandes (tokio WS + tray + IPC JS).
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: OverlayCommand) {
        match event {
            OverlayCommand::Show(payload, duration) => {
                self.show_notification(&payload, duration);
            }
            OverlayCommand::Hide => {
                self.hide_window();
            }
            OverlayCommand::Quit => {
                log::info!("Commande Quit reçue, fermeture...");
                event_loop.exit();
            }

            // ── Phase 4 : tray ───────────────────────────────────────────
            OverlayCommand::ConnectionStatus(connected) => {
                if let Some(t) = &self.tray {
                    tray::set_connected(t, connected);
                }
            }
            OverlayCommand::MenuAction(id) => {
                self.handle_menu_action(id, event_loop);
            }
            OverlayCommand::OpenSettings => {
                if self.settings_window.is_none() {
                    self.settings_window = settings::open(event_loop, self.ipc_proxy.clone());
                } else {
                    // Amener au premier plan si déjà ouverte
                    if let Some(sw) = &self.settings_window {
                        sw.window.focus_window();
                    }
                }
            }
            OverlayCommand::SettingsReady => {
                if let Some(sw) = &self.settings_window {
                    settings::inject_config(sw, &self.config);
                }
            }
            OverlayCommand::SettingsSave(json) => {
                self.handle_settings_save(json);
            }
            OverlayCommand::SettingsClose => {
                self.settings_window = None;
            }
            OverlayCommand::SetDuration(ms) => {
                // La vidéo a fourni sa vraie durée — on relance le timer avec cette valeur.
                log::info!("Durée vidéo reçue : {}ms — relance du timer auto-hide", ms);
                self.spawn_hide_timer(ms);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Timer auto-hide géré côté tokio — rien à faire ici.
    }
}

impl OverlayApp {
    fn handle_menu_action(&mut self, id: muda::MenuId, event_loop: &ActiveEventLoop) {
        let ids = match &self.tray {
            Some(t) => t.ids.clone(),
            None => return,
        };

        if id == ids.quit {
            log::info!("Menu : Quitter");
            event_loop.exit();
        } else if id == ids.test {
            log::info!("Menu : Test Notification");
            let payload = tray::test_payload();
            let duration = payload.duration.unwrap_or(self.config.default_duration);
            self.show_notification(&payload, duration);
        } else if id == ids.settings {
            log::info!("Menu : Settings");
            let proxy = self.ipc_proxy.clone();
            let _ = proxy.send_event(OverlayCommand::OpenSettings);
        } else if id == ids.connect_toggle {
            log::info!("Menu : Connect/Disconnect → TODO Phase 6");
        }
    }

    fn handle_settings_save(&mut self, json: String) {
        // Le JSON reçu est {action:'save', config:{...}} — on extrait config
        let parsed = match serde_json::from_str::<serde_json::Value>(&json) {
            Ok(v) => v,
            Err(e) => {
                log::error!("JSON settings invalide : {}", e);
                return;
            }
        };

        let config_value = match parsed.get("config") {
            Some(v) => v.clone(),
            None => {
                log::error!("Champ 'config' manquant dans le JSON settings");
                return;
            }
        };

        let new_cfg = match serde_json::from_value::<Config>(config_value) {
            Ok(c) => c,
            Err(e) => {
                log::error!("Désérialisation config échouée : {}", e);
                return;
            }
        };

        if let Err(e) = config::save(&new_cfg) {
            log::error!("Sauvegarde config échouée : {}", e);
            return;
        }

        log::info!("Config sauvegardée : server={} position={:?}", new_cfg.server_url, new_cfg.overlay_position);
        self.config = new_cfg;

        if let Some(sw) = &self.settings_window {
            settings::notify_save_success(sw);
        }
        // Phase 6 : relancer le WS si serverUrl a changé
    }
}
