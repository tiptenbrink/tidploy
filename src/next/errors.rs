use keyring::Error as KeyringError;
use std::io::Error as IOError;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub(crate) enum SecretError {
    #[error("Failed to get password from prompt! {0}")]
    Prompt(#[from] IOError),
    #[error("No secret saved for key {0}.")]
    NoPassword(String),
    #[error("Internal keyring failure. {0}")]
    Keyring(#[from] SecretKeyringError),
}

#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
pub(crate) struct SecretKeyringError {
    pub(crate) msg: String,
    pub(crate) source: KeyringError,
}

#[derive(ThisError, Debug)]
pub(crate) enum StateError {
    #[error("Could not create state due to IO error! {0}")]
    IO(#[from] IOError),
    #[error("Context root is invalid! {0}")]
    InvalidRoot(String),
    #[error("Could not create state as secrets failed to load! {0}")]
    Secret(#[from] SecretError)
}