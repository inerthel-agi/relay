use std::{
    env,
    sync::{Mutex, OnceLock},
};

use anyhow::{Context, Result, bail};
use keyring_core::{Entry, Error as KeyringError, set_default_store};
use rand::{RngCore, rng};
use serde::{Deserialize, Serialize};
use windows_native_keyring_store::Store;

const SERVICE: &str = "eu.inerthel.relay";
/// Former Credential Manager services, newest first. A value found there is
/// moved to `SERVICE` the first time it is read.
const LEGACY_SERVICES: [&str; 2] = ["eu.stealthylabs.relay", "eu.stealthylabs.discord-obs-relay"];
const DISCORD_ACCOUNT: &str = "discord-credentials";
const YOUTUBE_API_KEY_ACCOUNT: &str = "youtube-api-key";
const RELAY_SECRET_ACCOUNT: &str = "relay-secret";
const OBS_WEBSOCKET_ACCOUNT: &str = "obs-websocket-password";
static KEYRING_READY: OnceLock<Result<(), String>> = OnceLock::new();
static RELAY_SECRET_CACHE: OnceLock<Mutex<Option<String>>> = OnceLock::new();

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordCredentials {
    pub client_id: String,
    pub token: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatus {
    pub configured: bool,
    pub source: Option<&'static str>,
    pub client_id: Option<String>,
    pub youtube_configured: bool,
    pub obs_password_configured: bool,
}

pub fn initialize_keyring() -> Result<()> {
    KEYRING_READY
        .get_or_init(|| {
            let store = Store::new().map_err(|error| error.to_string())?;
            set_default_store(store);
            Ok(())
        })
        .clone()
        .map_err(anyhow::Error::msg)
}

pub fn save_discord_credentials(credentials: &DiscordCredentials) -> Result<()> {
    validate_discord_credentials(credentials)?;
    initialize_keyring()?;
    let serialized = serde_json::to_string(credentials)?;
    Entry::new(SERVICE, DISCORD_ACCOUNT)?
        .set_password(&serialized)
        .context("failed to save Discord credentials in Windows Credential Manager")
}

pub fn load_discord_credentials() -> Result<Option<(DiscordCredentials, &'static str)>> {
    initialize_keyring()?;
    match Entry::new(SERVICE, DISCORD_ACCOUNT)?.get_password() {
        Ok(serialized) => {
            let credentials: DiscordCredentials = serde_json::from_str(&serialized)
                .context("stored Discord credentials are invalid")?;
            validate_discord_credentials(&credentials)?;
            Ok(Some((credentials, "windows")))
        }
        Err(KeyringError::NoEntry) => match migrate_legacy_password(DISCORD_ACCOUNT)? {
            Some(serialized) => {
                let credentials: DiscordCredentials = serde_json::from_str(&serialized)
                    .context("stored Discord credentials are invalid")?;
                validate_discord_credentials(&credentials)?;
                Ok(Some((credentials, "windows")))
            }
            None => load_environment_credentials(),
        },
        Err(error) => Err(error).context("failed to read Discord credentials"),
    }
}

pub fn save_youtube_api_key(api_key: &str) -> Result<()> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Ok(());
    }
    validate_youtube_api_key(api_key)?;
    initialize_keyring()?;
    Entry::new(SERVICE, YOUTUBE_API_KEY_ACCOUNT)?
        .set_password(api_key)
        .context("failed to save the YouTube API key in Windows Credential Manager")
}

pub fn load_youtube_api_key() -> Result<Option<String>> {
    initialize_keyring()?;
    match Entry::new(SERVICE, YOUTUBE_API_KEY_ACCOUNT)?.get_password() {
        Ok(api_key) => {
            validate_youtube_api_key(&api_key)?;
            Ok(Some(api_key))
        }
        Err(KeyringError::NoEntry) => match migrate_legacy_password(YOUTUBE_API_KEY_ACCOUNT)? {
            Some(api_key) => {
                validate_youtube_api_key(&api_key)?;
                Ok(Some(api_key))
            }
            None => Ok(None),
        },
        Err(error) => Err(error).context("failed to read the YouTube API key"),
    }
}

/// OBS WebSocket password; an empty value removes the saved one.
pub fn save_obs_password(password: &str) -> Result<()> {
    initialize_keyring()?;
    let entry = Entry::new(SERVICE, OBS_WEBSOCKET_ACCOUNT)?;
    if password.is_empty() {
        // A forgotten password must not come back from a former service.
        for service in LEGACY_SERVICES {
            match Entry::new(service, OBS_WEBSOCKET_ACCOUNT)?.delete_credential() {
                Ok(()) | Err(KeyringError::NoEntry) => {}
                Err(error) => {
                    return Err(error).context("failed to remove the OBS WebSocket password");
                }
            }
        }
        return match entry.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(error) => Err(error).context("failed to remove the OBS WebSocket password"),
        };
    }
    if password.len() > 256 || password.chars().any(char::is_control) {
        bail!("The OBS WebSocket password is invalid.");
    }
    entry
        .set_password(password)
        .context("failed to save the OBS WebSocket password in Windows Credential Manager")
}

pub fn load_obs_password() -> Result<Option<String>> {
    initialize_keyring()?;
    match Entry::new(SERVICE, OBS_WEBSOCKET_ACCOUNT)?.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(KeyringError::NoEntry) => migrate_legacy_password(OBS_WEBSOCKET_ACCOUNT),
        Err(error) => Err(error).context("failed to read the OBS WebSocket password"),
    }
}

pub fn credential_status() -> Result<CredentialStatus> {
    let credentials = load_discord_credentials()?;
    let youtube_configured = load_youtube_api_key()?.is_some();
    let obs_password_configured = load_obs_password().ok().flatten().is_some();
    Ok(match credentials {
        Some((credentials, source)) => CredentialStatus {
            configured: true,
            source: Some(source),
            client_id: Some(credentials.client_id),
            youtube_configured,
            obs_password_configured,
        },
        None => CredentialStatus {
            configured: false,
            source: None,
            client_id: None,
            youtube_configured,
            obs_password_configured,
        },
    })
}

pub fn load_or_create_relay_secret() -> Result<String> {
    let mut cached_secret = RELAY_SECRET_CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| anyhow::anyhow!("relay secret cache is unavailable"))?;
    if let Some(secret) = cached_secret.as_ref() {
        return Ok(secret.clone());
    }

    initialize_keyring()?;
    let entry = Entry::new(SERVICE, RELAY_SECRET_ACCOUNT)?;
    let secret = match entry.get_password() {
        Ok(secret) if secret.len() >= 32 => Ok(secret),
        Ok(_) => create_relay_secret(),
        Err(KeyringError::NoEntry) => match migrate_legacy_password(RELAY_SECRET_ACCOUNT)? {
            Some(secret) if secret.len() >= 32 => Ok(secret),
            Some(_) | None => create_relay_secret(),
        },
        Err(error) => Err(error).context("failed to read the relay secret"),
    }?;
    *cached_secret = Some(secret.clone());
    Ok(secret)
}

fn create_relay_secret() -> Result<String> {
    initialize_keyring()?;
    let mut bytes = [0_u8; 32];
    rng().fill_bytes(&mut bytes);
    let secret = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Entry::new(SERVICE, RELAY_SECRET_ACCOUNT)?
        .set_password(&secret)
        .context("failed to save the relay secret")?;
    Ok(secret)
}

fn migrate_legacy_password(account: &str) -> Result<Option<String>> {
    for service in LEGACY_SERVICES {
        let legacy_entry = Entry::new(service, account)?;
        let password = match legacy_entry.get_password() {
            Ok(password) => password,
            Err(KeyringError::NoEntry) => continue,
            Err(error) => return Err(error).context("failed to read legacy Relay credentials"),
        };
        Entry::new(SERVICE, account)?
            .set_password(&password)
            .context("failed to migrate Relay credentials")?;
        legacy_entry
            .delete_credential()
            .context("failed to remove migrated Relay credentials")?;
        return Ok(Some(password));
    }
    Ok(None)
}

fn load_environment_credentials() -> Result<Option<(DiscordCredentials, &'static str)>> {
    let client_id = env::var("DISCORD_CLIENT_ID")
        .ok()
        .filter(|value| !value.is_empty());
    let token = env::var("DISCORD_TOKEN")
        .ok()
        .filter(|value| !value.is_empty());
    match (client_id, token) {
        (Some(client_id), Some(token)) => {
            let credentials = DiscordCredentials { client_id, token };
            validate_discord_credentials(&credentials)?;
            Ok(Some((credentials, "environment")))
        }
        (None, None) => Ok(None),
        _ => bail!("DISCORD_CLIENT_ID and DISCORD_TOKEN must both be configured."),
    }
}

fn validate_discord_credentials(credentials: &DiscordCredentials) -> Result<()> {
    if !(17..=20).contains(&credentials.client_id.len())
        || !credentials
            .client_id
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        bail!("The Discord client ID is invalid.");
    }
    if credentials.token.trim().len() < 20 {
        bail!("The Discord token is invalid.");
    }
    Ok(())
}

fn validate_youtube_api_key(api_key: &str) -> Result<()> {
    if !(20..=256).contains(&api_key.len())
        || api_key
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        bail!("The YouTube API key is invalid.");
    }
    Ok(())
}
