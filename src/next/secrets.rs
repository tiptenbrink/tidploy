use std::{collections::HashMap};

use color_eyre::eyre::Report;
use keyring::{Entry, Error as KeyringError};
use rpassword::prompt_password;
use tracing::{debug, instrument};

use crate::commands::TIDPLOY_DEFAULT;

use super::{
    config::ConfigVar, errors::{SecretError, SecretKeyringError, StateError, WrapStateErr}, resolve::SecretScope, state::{create_state, StateIn}
};

fn get_keyring_secret(key: &str, service: &str) -> Result<Option<String>, KeyringError> {
    debug!(
        "Trying to get keyring password with key {} for service {}",
        key, service
    );
    let entry = Entry::new(service, key)?;
    match entry.get_password() {
        Ok(pw) => Ok(Some(pw)),
        Err(err) => match err {
            KeyringError::NoEntry => {
                debug!("No entry found!");
                Ok(None)
            }
            _ => Err(err),
        },
    }
}

fn set_keyring_secret(secret: &str, key: &str, service: &str) -> Result<(), KeyringError> {
    let entry = Entry::new(service, key)?;
    entry.set_password(secret)?;
    debug!(
        "Set keyring password with key {} for service {}",
        key, service
    );
    Ok(())
}

fn key_from_scope(
    scope: &SecretScope,
    key: &str,
) -> String {
    format!("{}::{}::{}:{}", scope.name, scope.sub, scope.hash, key)
}

/// Gets secret using a key with format `<context_name>::<state_name>::<hash>:<key>`.
#[instrument(name = "get_secret", level = "debug", skip_all)]
pub(crate) fn get_secret(
    scope: &SecretScope,
    key: &str,
) -> Result<String, SecretError> {
    debug!("Getting secret with key {}", key);
    let store_key = key_from_scope(scope, key);

    match get_keyring_secret(&store_key, &scope.service).map_err(|e| SecretKeyringError {
        msg: format!("Failed to get key {}", &store_key),
        source: e,
    })? {
        Some(password) => Ok(password),
        None => Err(SecretError::NoPassword(store_key)),
    }
}

/// Prompts for secret and saves it at `<context_name>::<state_name>::<hash>:<key>`.
/// If `prompt` is None it will prompt for a password, otherwise it will use the given prompt.
fn secret_prompt(
    scope: &SecretScope,
    key: &str,
    prompt: Option<String>,
) -> Result<String, SecretError> {
    let password = if let Some(prompt) = prompt {
        prompt
    } else {
        prompt_password("Enter secret:\n")?
    };

    let store_key = key_from_scope(scope, key);

    set_keyring_secret(&password, &store_key, &scope.service).map_err(|e| SecretKeyringError {
        msg: format!("Failed to get key {}", &store_key),
        source: e,
    })?;
    Ok(store_key)
}

// pub(crate) fn secret_command(
//     state_in: StateIn,
//     key: &str,
//     prompt: Option<String>,
// ) -> Result<String, Report> {
//     debug!(
//         "Secret command called with in_state {:?}, key {:?} and prompt {:?}",
//         state_in, key, prompt
//     );

//     let state = create_state(state_in)?;

//     let store_key = secret_prompt(&state, key, prompt)?;

//     println!("Set secret with store key {}!", &store_key);
//     Ok(store_key)
// }

pub(crate) fn secret_vars_to_envs(
    scope: &SecretScope,
    vars: Vec<ConfigVar>,
) -> Result<HashMap<String, String>, StateError> {
    let mut envs = HashMap::<String, String>::new();
    for e in vars {
        debug!("Getting pass for {:?}", e);
        let pass = get_secret(scope, &e.key)
        .to_state_err("Getting secret for config var to create env map.".to_owned())?;

        envs.insert(e.env_name, pass);
    }
    Ok(envs)
}
