//! Phase 4 — System tray icon et menu contextuel
//!
//! Crée l'icône dans le system tray Windows avec un menu contextuel.
//! Les événements (clics menu, double-clic icône) sont forwardés vers
//! l'event loop winit via EventLoopProxy<OverlayCommand>.
//!
//! IMPORTANT : build() doit être appelé APRÈS le démarrage de l'event loop
//! (dans new_events avec StartCause::Init).

use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};
use winit::event_loop::EventLoopProxy;

use crate::overlay::OverlayCommand;
use crate::websocket::NotificationPayload;

// ---------------------------------------------------------------------------
// Icône embarquée à la compilation
// ---------------------------------------------------------------------------

const ICON_BYTES: &[u8] = include_bytes!("../../../bozoicon.ico");

fn load_icon() -> tray_icon::Icon {
    let cursor = std::io::Cursor::new(ICON_BYTES);
    let icon_dir = ico::IconDir::read(cursor).expect("Impossible de lire bozoicon.ico");

    // Prendre la plus grande entrée disponible
    let entry = icon_dir
        .entries()
        .iter()
        .max_by_key(|e| e.width())
        .expect("ICO vide");

    let image = entry.decode().expect("Impossible de décoder l'icône");
    tray_icon::Icon::from_rgba(image.rgba_data().to_vec(), image.width(), image.height())
        .expect("Impossible de créer l'icône tray")
}

// ---------------------------------------------------------------------------
// État du tray
// ---------------------------------------------------------------------------

/// IDs des items du menu pour pouvoir les identifier dans MenuEvent.
#[derive(Debug, Clone)]
pub struct MenuIds {
    pub settings: muda::MenuId,
    pub connect_toggle: muda::MenuId,
    pub test: muda::MenuId,
    pub quit: muda::MenuId,
}

/// État du tray conservé dans OverlayApp.
pub struct TrayState {
    /// Le tray icon lui-même (doit rester en vie, dropping = disparaît).
    pub tray: TrayIcon,
    /// Item "BozoChat (Connecté/Déconnecté)" — label disabled.
    pub status_item: MenuItem,
    /// Item "Reconnecter" / "Déconnecter" — texte change selon l'état.
    pub connect_item: MenuItem,
    /// IDs des items pour le dispatch dans user_event.
    pub ids: MenuIds,
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

/// Crée le tray icon et enregistre les handlers d'événements.
/// Les événements sont forwardés vers l'event loop winit via proxy.
pub fn build(proxy: EventLoopProxy<OverlayCommand>) -> Result<TrayState, Box<dyn std::error::Error>> {
    let icon = load_icon();

    // ── Menu ────────────────────────────────────────────────────────────────
    let menu = Menu::new();

    let status_item = MenuItem::new("BozoChat (Déconnecté)", false, None);
    let settings_item = MenuItem::new("Settings", true, None);
    let connect_item = MenuItem::new("Reconnecter", true, None);
    let separator1 = PredefinedMenuItem::separator();
    let separator2 = PredefinedMenuItem::separator();
    let separator3 = PredefinedMenuItem::separator();
    let test_item = MenuItem::new("Test Notification", true, None);
    let quit_item = MenuItem::new("Quitter", true, None);

    let ids = MenuIds {
        settings: settings_item.id().clone(),
        connect_toggle: connect_item.id().clone(),
        test: test_item.id().clone(),
        quit: quit_item.id().clone(),
    };

    menu.append_items(&[
        &status_item,
        &separator1,
        &settings_item,
        &connect_item,
        &separator2,
        &test_item,
        &separator3,
        &quit_item,
    ])?;

    // ── TrayIcon ────────────────────────────────────────────────────────────
    let tray = TrayIconBuilder::new()
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .with_tooltip("BozoChat - Déconnecté")
        .build()?;

    // ── Handlers d'événements ───────────────────────────────────────────────
    // Menu events (clics sur les items)
    let proxy_menu = proxy.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let _ = proxy_menu.send_event(OverlayCommand::MenuAction(event.id.clone()));
    }));

    // Tray icon events (double-clic → ouvrir settings)
    let proxy_tray = proxy.clone();
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::DoubleClick { .. } = event {
            let _ = proxy_tray.send_event(OverlayCommand::OpenSettings);
        }
    }));

    log::info!("System tray initialisé");

    Ok(TrayState {
        tray,
        status_item,
        connect_item,
        ids,
    })
}

// ---------------------------------------------------------------------------
// Mise à jour de l'état de connexion
// ---------------------------------------------------------------------------

/// Met à jour le tooltip et les items du menu selon l'état de connexion.
pub fn set_connected(state: &TrayState, connected: bool) {
    if connected {
        let _ = state.tray.set_tooltip(Some("BozoChat - Connecté"));
        state.status_item.set_text("BozoChat (Connecté)");
        state.connect_item.set_text("Déconnecter");
    } else {
        let _ = state.tray.set_tooltip(Some("BozoChat - Déconnecté"));
        state.status_item.set_text("BozoChat (Déconnecté)");
        state.connect_item.set_text("Reconnecter");
    }
    log::info!("Tray mis à jour : connecté={}", connected);
}

// ---------------------------------------------------------------------------
// Payload de test
// ---------------------------------------------------------------------------

/// Crée un payload de notification de test (pour le menu "Test Notification").
pub fn test_payload() -> NotificationPayload {
    NotificationPayload {
        sender: "System".to_string(),
        message: "Test BozoChat !".to_string(),
        media_type: None,
        media_filename: None,
        media_data: None,
        duration: Some(3000),
    }
}
