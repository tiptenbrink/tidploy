use keyring::{Entry, Error as KeyringError};
use rpassword::prompt_password;
use tracing::debug;

use crate::commands::TIDPLOY_DEFAULT;

use super::errors::{SecretError, SecretKeyringError};

fn get_keyring_secret(key: &str) -> Result<Option<String>, KeyringError> {
    debug!(
        "Trying to get keyring password with key {} for service tidploy",
        key
    );
    let entry = Entry::new("tidploy", key)?;
    match entry.get_password() {
        Ok(pw) => Ok(Some(pw)),
        Err(err) => match err {
            KeyringError::NoEntry => {
                debug!("No entry found!");
                    Ok(None)
            }, 
            _ => Err(err)
        },
    }
}

fn set_keyring_secret(secret: &str, key: &str) -> Result<(), KeyringError> {
    let entry = Entry::new("tidploy", key)?;
    entry.set_password(secret)?;
    debug!("Set keyring password with key {} for service tidploy", key);
    Ok(())
}

/// Prompts for secret and saves it at `<context_name>::<deploy path>::<hash>:<key>`.
pub(crate) fn secret_command(context_name: &str, deploy_path: Option<&str>, hash: &str, key: &str) -> Result<(), SecretError> {
    let password = prompt_password("Enter secret:\n")?;
    let deploy_path = deploy_path.unwrap_or(TIDPLOY_DEFAULT);

    let store_key = key_from_option(Some(context_name), Some(deploy_path), hash, key);

    set_keyring_secret(&password, &store_key).map_err(|e| {
        SecretKeyringError {
            msg: format!("Failed to get key {}", &store_key),
            source: e
        }
    })?;
    Ok(println!("Set secret with store key {}!", &store_key))
}


fn key_from_option(context_name: Option<&str>, state_name: Option<&str>, hash: &str, key: &str) -> String {
    let state_name = state_name.unwrap_or(TIDPLOY_DEFAULT);
    let context_name = context_name.unwrap_or(TIDPLOY_DEFAULT);

    format!("{}::{}::{}:{}", context_name, state_name, hash, key)
}

/// Gets secret using a key with format `<context_name>::<state_name>::<hash>:<key>`.
pub(crate) fn get_secret(context_name: Option<&str>, state_name: Option<&str>, hash: &str, key: &str) -> Result<String, SecretError> {
    debug!("Getting secret with key {}", key);
    let store_key = key_from_option(context_name, state_name, hash, key);

    match get_keyring_secret(&store_key).map_err(|e| SecretKeyringError {
        msg: format!("Failed to get key {}", &store_key),
        source: e
    })? {
        Some(password) => Ok(password),
        None => Err(SecretError::NoPassword(store_key)),
    }
}