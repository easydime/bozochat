//! Phase 1 — Configuration module
//!
//! Charge et sauvegarde la config depuis `~/.bozochat/config.json`.
//! Les valeurs par défaut sont identiques à celles de l'Electron client (main.js).

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Position de l'overlay, correspond aux options du <select> dans settings.html.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OverlayPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

impl Default for OverlayPosition {
    fn default() -> Self {
        Self::Center
    }
}

/// Configuration de l'application, stockée en JSON.
/// Tous les champs ont `#[serde(default)]` : un fichier partiel ou absent ne crash pas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "defaults::server_url")]
    pub server_url: String,

    #[serde(default)]
    pub user_id: String,

    #[serde(default)]
    pub overlay_position: OverlayPosition,

    /// Durée d'affichage en ms (Electron : defaultDuration = 5000).
    #[serde(default = "defaults::default_duration")]
    pub default_duration: u64,

    #[serde(default)]
    pub auto_start: bool,
}

mod defaults {
    pub fn server_url() -> String {
        "ws://localhost:3001".to_string()
    }

    pub fn default_duration() -> u64 {
        5000
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: defaults::server_url(),
            user_id: String::new(),
            overlay_position: OverlayPosition::default(),
            default_duration: defaults::default_duration(),
            auto_start: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Chemin du fichier
// ---------------------------------------------------------------------------

/// Retourne `~/.bozochat/config.json`.
pub fn config_path() -> PathBuf {
    let home = dirs::home_dir().expect("Impossible de déterminer le répertoire home");
    home.join(".bozochat").join("config.json")
}

// ---------------------------------------------------------------------------
// Chargement / Sauvegarde
// ---------------------------------------------------------------------------

/// Charge la config depuis `~/.bozochat/config.json`.
///
/// - Si le fichier n'existe pas : crée le dossier + fichier avec les défauts, retourne les défauts.
/// - Si le fichier existe mais est invalide : retourne une erreur.
/// - Si le fichier est valide mais incomplet : les champs manquants prennent leurs valeurs par défaut.
pub fn load() -> io::Result<Config> {
    let path = config_path();

    if !path.exists() {
        log::info!("Fichier config absent, création avec les défauts : {:?}", path);
        let config = Config::default();
        save(&config)?;
        return Ok(config);
    }

    log::info!("Chargement de la config depuis {:?}", path);
    let raw = fs::read_to_string(&path)?;
    let config: Config = serde_json::from_str(&raw).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Impossible de parser config.json : {}", e),
        )
    })?;
    Ok(config)
}

/// Sauvegarde la config dans `~/.bozochat/config.json` (format JSON indenté).
///
/// Crée le dossier `~/.bozochat/` s'il n'existe pas.
pub fn save(config: &Config) -> io::Result<()> {
    let path = config_path();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(config).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Impossible de sérialiser la config : {}", e),
        )
    })?;

    fs::write(&path, json)?;
    log::info!("Config sauvegardée dans {:?}", path);
    Ok(())
}
