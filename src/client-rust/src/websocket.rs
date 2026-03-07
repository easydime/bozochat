//! Phase 2 — Client WebSocket
//!
//! Gère la connexion, la reconnexion automatique, le parsing des messages
//! et le forwarding des notifications vers le thread UI via un channel mpsc.
//!
//! Protocole (src/bot/index.js) :
//!   Serveur → Client : connected | notification | ping | server-shutdown
//!   Client → Serveur : auth | pong

use crate::config::Config;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};

// ---------------------------------------------------------------------------
// Types de messages
// ---------------------------------------------------------------------------

/// Messages reçus du serveur. Le champ "type" est le discriminant serde.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ServerMessage {
    Connected {
        message: String,
        #[serde(rename = "clientId")]
        client_id: String,
    },
    Notification {
        data: NotificationPayload,
    },
    Ping,
    ServerShutdown,
    /// Catch-all pour les types futurs inconnus.
    #[serde(other)]
    Unknown,
}

/// Payload de la notification (champ "data" du message notification).
/// Les champs media sont Option car absents pour les notifications texte-only.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotificationPayload {
    pub sender: String,
    pub message: String,
    /// Ex : "image/png", "video/mp4". Absent si pas de pièce jointe.
    #[serde(rename = "mediaType")]
    pub media_type: Option<String>,
    /// Nom original du fichier, ex : "chat.gif".
    #[serde(rename = "mediaFilename")]
    pub media_filename: Option<String>,
    /// Données binaires encodées en base64.
    #[serde(rename = "mediaData")]
    pub media_data: Option<String>,
    /// Durée d'affichage en ms. Utiliser config.default_duration si absent.
    pub duration: Option<u64>,
}

/// Événements envoyés du thread WS vers le thread UI.
#[derive(Debug, Clone)]
pub enum WsEvent {
    /// Payload prête à afficher.
    Notification(NotificationPayload),
    /// Connexion établie (pour l'icône tray en Phase 3).
    Connected,
    /// Déconnexion (pour l'icône tray en Phase 3).
    Disconnected,
}

// ---------------------------------------------------------------------------
// Builders de messages Client → Serveur
// ---------------------------------------------------------------------------

fn auth_message(user_id: &str) -> String {
    serde_json::json!({ "type": "auth", "userId": user_id }).to_string()
}

fn pong_message() -> String {
    serde_json::json!({ "type": "pong" }).to_string()
}

// ---------------------------------------------------------------------------
// Boucle de reconnexion
// ---------------------------------------------------------------------------

/// Démarre le client WebSocket dans une tâche tokio.
///
/// Retourne le receiver du channel d'événements.
/// Quand la tâche se termine (server-shutdown ou receiver droppé), le channel se ferme.
pub fn spawn(config: Config) -> mpsc::Receiver<WsEvent> {
    // Capacité 32 : le thread UI est bloqué si trop lent plutôt que d'accumuler sans limite.
    let (tx, rx) = mpsc::channel::<WsEvent>(32);

    tokio::spawn(async move {
        run_reconnect_loop(config, tx).await;
    });

    rx
}

/// Boucle principale. Tourne jusqu'à server-shutdown ou fermeture du receiver.
async fn run_reconnect_loop(config: Config, tx: mpsc::Sender<WsEvent>) {
    loop {
        log::info!("Connexion à {}", config.server_url);

        match connect_async(&config.server_url).await {
            Ok((ws_stream, _response)) => {
                log::info!("Connecté au serveur BozoChat");

                if tx.send(WsEvent::Connected).await.is_err() {
                    return; // receiver droppé, on arrête silencieusement
                }

                let should_reconnect = handle_connection(ws_stream, &config, &tx).await;

                let _ = tx.send(WsEvent::Disconnected).await;

                if !should_reconnect {
                    log::info!("Serveur arrêté. Pas de reconnexion.");
                    return;
                }
            }
            Err(e) => {
                log::warn!("Erreur de connexion WebSocket : {}. Retry dans 5s...", e);
            }
        }

        // Délai avant reconnexion — identique au setTimeout 5000ms de l'Electron client.
        sleep(Duration::from_millis(5000)).await;
    }
}

/// Gère une connexion WebSocket active.
///
/// Retourne `true` si on doit se reconnecter, `false` sur server-shutdown.
async fn handle_connection<S>(
    ws_stream: tokio_tungstenite::WebSocketStream<S>,
    config: &Config,
    tx: &mpsc::Sender<WsEvent>,
) -> bool
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (mut writer, mut reader) = ws_stream.split();

    // Envoi du message auth si userId configuré (réplique le comportement JS).
    if !config.user_id.is_empty() {
        let auth = auth_message(&config.user_id);
        if let Err(e) = writer.send(Message::Text(auth.into())).await {
            log::error!("Erreur envoi auth : {}", e);
            return true;
        }
        log::info!("Auth envoyé pour l'utilisateur '{}'", config.user_id);
    }

    // Boucle de lecture
    while let Some(msg_result) = reader.next().await {
        match msg_result {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<ServerMessage>(&text) {
                    Ok(server_msg) => {
                        let keep_going = dispatch(server_msg, &mut writer, tx).await;
                        if !keep_going {
                            return false; // server-shutdown
                        }
                    }
                    Err(e) => {
                        log::warn!("Message non parsable : {} — brut : {}", e, text);
                    }
                }
            }
            Ok(Message::Close(_)) => {
                log::info!("Serveur a fermé la connexion WebSocket");
                break;
            }
            Ok(_) => {
                // Frames binaires / ping WS / pong WS — ignorées
            }
            Err(e) => {
                log::error!("Erreur lecture WebSocket : {}", e);
                break;
            }
        }
    }

    true // reconnect
}

/// Dispatche un message parsé. Retourne `false` uniquement sur server-shutdown.
async fn dispatch<S>(
    msg: ServerMessage,
    writer: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<S>,
        Message,
    >,
    tx: &mpsc::Sender<WsEvent>,
) -> bool
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    match msg {
        ServerMessage::Connected { message, client_id } => {
            log::info!(
                "Connexion confirmée — clientId={} message='{}'",
                client_id,
                message
            );
        }

        ServerMessage::Notification { data } => {
            log::info!(
                "Notification de '{}' : '{}' (media={})",
                data.sender,
                data.message,
                data.media_type.as_deref().unwrap_or("aucun")
            );
            if tx.send(WsEvent::Notification(data)).await.is_err() {
                return false; // receiver droppé
            }
        }

        ServerMessage::Ping => {
            log::debug!("Ping reçu, envoi pong");
            let pong = pong_message();
            if let Err(e) = writer.send(Message::Text(pong.into())).await {
                log::error!("Erreur envoi pong : {}", e);
            }
        }

        ServerMessage::ServerShutdown => {
            log::info!("server-shutdown reçu. Déconnexion propre.");
            return false;
        }

        ServerMessage::Unknown => {
            log::debug!("Type de message inconnu, ignoré");
        }
    }

    true
}
