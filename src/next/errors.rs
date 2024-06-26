use keyring::Error as KeyringError;
use std::io::Error as IOError;
use thiserror::Error as ThisError;
use tracing_error::TracedError;

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
#[error("{msg}\n{source}")]
pub(crate) struct StateError {
    pub(crate) msg: String,
    // This traced error means that traces up to the creation of the specific kind will also be tracked
    pub(crate) source: TracedError<StateErrorKind>,
}

#[derive(ThisError, Debug)]
pub(crate) enum StateErrorKind {
    #[error("Path is not valid UTF-8!")]
    InvalidPath,
    #[error("State manipulation failed due to IO error! {0}")]
    IO(#[from] IOError),
    // #[error("{0}")]
    // InvalidRoot(String),
    #[error("{0}")]
    Secret(#[from] SecretError),
    #[error("{0}")]
    Git(#[from] GitError),
    #[error("{0}")]
    Config(#[from] ConfigError),
    #[error("{0}")]
    Address(#[from] AddressError),
}

pub(crate) trait WrapStateErr<T, E> {
    fn to_state_err<S: Into<String>>(self, msg: S) -> Result<T, StateError>;
}

impl<T, E> WrapStateErr<T, E> for Result<T, E>
where
    E: Into<StateErrorKind> + Send + Sync + 'static,
{
    fn to_state_err<S: Into<String>>(self, msg: S) -> Result<T, StateError> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(StateError {
                msg: msg.into(),
                source: e.into().into(),
            }),
        }
    }
}

#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
pub(crate) struct ProcessIOError {
    pub(crate) msg: String,
    pub(crate) source: IOError,
}

#[derive(ThisError, Debug)]
pub(crate) enum ProcessError {
    #[error("Failed to decode process output! {0}")]
    Decode(String),
    #[error("Internal IO error when trying to run process! {0}")]
    IO(#[from] ProcessIOError),
}

#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
pub(crate) struct GitProcessError {
    pub(crate) msg: String,
    pub(crate) source: ProcessError,
}

#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
pub(crate) struct ContextIOError {
    pub(crate) msg: String,
    pub(crate) source: IOError,
}

#[derive(ThisError, Debug)]
pub(crate) enum GitError {
    #[error("Git command failed with following output: {0}")]
    Failed(String),
    #[error("Process error trying to run Git! {0}")]
    Process(#[from] GitProcessError),
    #[error("Error with the filesystem before running Git: {0}")]
    IO(#[from] ContextIOError),
}

#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
pub(crate) struct ConfigError {
    pub(crate) msg: String,
    pub(crate) source: ConfigErrorKind,
}

#[derive(Debug, ThisError)]
pub(crate) enum ConfigErrorKind {
    #[error("IO error during config load! {0}")]
    IO(#[from] IOError),
    #[error("Failed to parse config TOML! {0}")]
    TOMLDecode(#[from] toml::de::Error),
    #[error("Failed to parse config JSON! {0}")]
    JSONDecode(#[from] serde_json::Error),
}

pub(crate) trait WrapConfigErr<T, E> {
    fn to_config_err(self, msg: String) -> Result<T, ConfigError>;
}

impl<T, E> WrapConfigErr<T, E> for Result<T, E>
where
    E: Into<ConfigErrorKind> + Send + Sync + 'static,
{
    fn to_config_err(self, msg: String) -> Result<T, ConfigError> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(ConfigError {
                msg,
                source: e.into(),
            }),
        }
    }
}

#[derive(Debug, ThisError)]
pub(crate) enum ResolutionError {
    #[error("Failed to resolve configs! {0}")]
    Config(#[from] ConfigError),
}

#[derive(Debug, ThisError)]
pub(crate) enum AddressError {
    #[error("Repo URL '{0}' doesn't end with /<name>.git and cannot be parsed!")]
    RepoParse(String),
}
