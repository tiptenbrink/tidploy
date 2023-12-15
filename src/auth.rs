use crate::commands::{DEFAULT, TIDPLOY_DEFAULT};
use crate::secret_store::{get_password, set_password};

use crate::state::State;

use keyring::Error as KeyringError;

use rpassword::prompt_password;

use std::io::Error as IOError;

use thiserror::Error as ThisError;

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

pub(crate) fn auth_command(state: &State, key: String) -> Result<(), AuthError> {
    let password = prompt_password("Enter password:\n").map_err(|e| AuthError {
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

pub(crate) fn auth_get_password(state: &State, key: &str) -> Result<String, AuthErrorKind> {
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
