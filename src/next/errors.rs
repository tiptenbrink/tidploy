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