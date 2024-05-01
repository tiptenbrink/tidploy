use keyring::Error as KeyringError;
use std::io::Error as IOError;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
pub(crate) struct SecretError {
    pub(crate) msg: String,
    pub(crate) source: SecretErrorKind,
}

#[derive(ThisError, Debug)]
pub(crate) enum SecretErrorKind {
    #[error("Failed to get password from prompt! {0}")]
    Prompt(#[from] IOError),
    #[error("No password saved.")]
    NoPassword,
    #[error("Internal keyring failure. {0}")]
    Keyring(#[from] KeyringError),
}
