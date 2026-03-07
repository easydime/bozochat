//! BozoChat Rust client — point d'entrée
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//!
//! Phase 1 : Chargement config (~/.bozochat/config.json)
//! Phase 2 : Client WebSocket (thread tokio dédié)
//! Phase 3 : Overlay WebView transparent (thread principal winit)

mod config;
mod overlay;
mod settings;
mod tray;
mod websocket;

use std::process;

use overlay::OverlayCommand;
use websocket::WsEvent;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // ── Phase 1 : Configuration ───────────────────────────────────────────
    let cfg = match config::load() {
        Ok(c) => {
            log::info!(
                "Config — server={} userId='{}' position={:?} duration={}ms autoStart={}",
                c.server_url, c.user_id, c.overlay_position, c.default_duration, c.auto_start
            );
            c
        }
        Err(e) => {
            eprintln!("Erreur fatale : impossible de charger la config : {}", e);
            process::exit(1);
        }
    };
    log::info!("Fichier config : {:?}", config::config_path());

    // ── Phase 3 : EventLoop winit sur le thread principal ─────────────────
    // L'EventLoop DOIT être créé avant de spawner le thread tokio
    // pour pouvoir créer les proxies nécessaires.
    let event_loop = winit::event_loop::EventLoop::<OverlayCommand>::with_user_event()
        .build()
        .unwrap_or_else(|e| {
            eprintln!("Impossible de créer l'event loop : {}", e);
            process::exit(1);
        });

    // proxy_ws : utilisé par le thread tokio pour envoyer Show/Quit
    // proxy_ipc : stocké dans OverlayApp, cloné dans la closure IPC du WebView
    let proxy_ws  = event_loop.create_proxy();
    let proxy_ipc = event_loop.create_proxy();

    // ── Phase 2 : Thread tokio dédié ─────────────────────────────────────
    let cfg_clone = cfg.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Impossible de créer le runtime tokio");

        rt.block_on(async move {
            let mut event_rx = websocket::spawn(cfg_clone.clone());

            loop {
                match event_rx.recv().await {
                    Some(WsEvent::Connected) => {
                        log::info!("[WS] Connecté au serveur.");
                        let _ = proxy_ws.send_event(OverlayCommand::ConnectionStatus(true));
                    }
                    Some(WsEvent::Disconnected) => {
                        log::info!("[WS] Déconnecté. Reconnexion en cours...");
                        let _ = proxy_ws.send_event(OverlayCommand::ConnectionStatus(false));
                    }
                    Some(WsEvent::Notification(payload)) => {
                        let duration = payload.duration.unwrap_or(cfg_clone.default_duration);
                        log::info!(
                            "[NOTIFICATION] De: {} | '{}' | media={} | {}ms",
                            payload.sender,
                            payload.message,
                            payload.media_type.as_deref().unwrap_or("aucun"),
                            duration
                        );

                        // Le timer auto-hide est géré dans OverlayApp::show_notification
                        if proxy_ws.send_event(OverlayCommand::Show(payload, duration)).is_err() {
                            log::warn!("Event loop fermée, arrêt du thread WS.");
                            break;
                        }
                    }
                    None => {
                        log::info!("[WS] Tâche WebSocket terminée.");
                        let _ = proxy_ws.send_event(OverlayCommand::Quit);
                        break;
                    }
                }
            }
        });
    });

    // ── Phase 3 : Lancer l'overlay sur le thread principal ────────────────
    println!("[BozoChat] Démarré. Ferme la fenêtre ou Ctrl-C pour quitter.");
    let mut app = overlay::OverlayApp::new(cfg, proxy_ipc);

    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("Erreur event loop : {}", e);
        process::exit(1);
    }
}
