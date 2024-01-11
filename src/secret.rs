use crate::commands::{DEFAULT, TIDPLOY_DEFAULT};
use crate::secret_store::{get_password, set_password};

use crate::state::State;

use keyring::Error as KeyringError;

use rpassword::prompt_password;

use std::io::Error as IOError;

use thiserror::Error as ThisError;

use tracing::{debug, span, Level};

#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
pub(crate) struct AuthError {
    pub(crate) msg: String,
    pub(crate) source: AuthErrorKind,
}

#[derive(ThisError, Debug)]
pub(crate) enum AuthErrorKind {
    #[error("Failed to get password from prompt! {0}")]
    Prompt(#[from] IOError),
    #[error("No password saved.")]
    NoPassword,
    #[error("Internal keyring failure. {0}")]
    Keyring(#[from] KeyringError),
}

/// Prompts for secret and saves it at <key>:<repo name>/<deploy path>/<commit sha>. The last three are all extracted
/// from the state. Forward slashes in the deploy path are replaced with \\.
pub(crate) fn secret_command(state: &State, key: String) -> Result<(), AuthError> {
    let password = prompt_password("Enter secret:\n").map_err(|e| AuthError {
        msg: "Failed to create password prompt!".to_owned(),
        source: e.into(),
    })?;
    let path_str = state.deploy_path.as_str().replace('/', "\\\\");
    let store_key: String = format!(
        "{}:{}/{}/{}",
        key, state.repo.name, path_str, state.commit_sha
    );
    set_password(&password, &store_key).map_err(|e| {
        let msg = format!(
            "Could not set password in auth command with store_key {}!",
            store_key
        );
        AuthError {
            msg,
            source: e.into(),
        }
    })?;
    Ok(println!("Set password with store_key {}!", &store_key))
}

/// Gets secret using a key with format <key>:<repo name>/<deploy path>/<commit sha>. If it cannot find an exact
/// match, it will first replace the commit sha with _tidploy_default, then it will try with that and with an empty
/// deploy path. Finally it will try with 'repo' set to default.
pub(crate) fn get_secret(state: &State, key: &str) -> Result<String, AuthErrorKind> {
    let secret_span = span!(Level::DEBUG, "get_secret");
    let _enter = secret_span.enter();
    debug!("Getting secret with key {}", key);
    let path_str = state.deploy_path.as_str().replace('/', "\\\\");
    let store_key: String = format!(
        "{}:{}/{}/{}",
        key, state.repo.name, path_str, state.commit_sha
    );
    if let Some(password) = get_password(&store_key)? {
        return Ok(password);
    }
    let store_key_default_commit = format!(
        "{}:{}/{}/{}",
        key, state.repo.name, path_str, TIDPLOY_DEFAULT
    );
    if let Some(password) = get_password(&store_key_default_commit)? {
        return Ok(password);
    }

    let store_key_default_commit_deploy =
        format!("{}:{}/{}/{}", key, state.repo.name, "", TIDPLOY_DEFAULT);
    if let Some(password) = get_password(&store_key_default_commit_deploy)? {
        return Ok(password);
    }
    let store_key_default = format!("{}:{}/{}/{}", key, DEFAULT, "", TIDPLOY_DEFAULT);
    match get_password(&store_key_default)? {
        Some(password) => Ok(password),
        None => Err(AuthErrorKind::NoPassword),
    }
}
