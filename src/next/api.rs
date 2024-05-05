use super::run::run_command_input as inner_run_command;
use super::secrets::secret_command as inner_secret_command;
use super::state::{StateIn, StateOptions};

use camino::Utf8PathBuf;
use color_eyre::eyre::Report;
use thiserror::Error as ThisError;

pub use super::process::EntrypointOut;
pub use crate::state::StateContext;

/// These represent global arguments that correspond to global args of the CLI (i.e. valid for all
/// subcomannds). To limit breaking changes, this struct is `non_exhaustive`.
///
/// Instantiate GlobalArguments using:
/// ```
/// # use tidploy::GlobalArguments;
/// let mut global_args = GlobalArguments::default();
/// ```
/// Then you can set the arguments like:
/// ```
/// # use tidploy::GlobalArguments;
/// # let mut global_args = GlobalArguments::default();
/// global_args.cwd_context = false;
/// ```
#[non_exhaustive]
#[derive(Default, Clone)]
pub struct GlobalArguments {
    pub cwd_context: bool,
    pub resolve_root: Option<String>,
    pub state_root: Option<String>,
    pub state_path: Option<String>,
    pub store_dir: Option<Utf8PathBuf>, // pub repo_url: Option<String>,
                                        // pub deploy_path: Option<String>,
                                        // pub tag: Option<String>,
}

impl From<GlobalArguments> for StateIn {
    fn from(value: GlobalArguments) -> Self {
        Self::from_args(
            value.cwd_context,
            value.resolve_root,
            value.state_path,
            value.state_root,
        )
    }
}

impl From<GlobalArguments> for StateOptions {
    fn from(value: GlobalArguments) -> Self {
        let default = Self::default();

        Self {
            store_dir: value.store_dir.unwrap_or(default.store_dir),
        }
    }
}

/// These represent arguments that correspond to args of the CLI `run` subcommand. To limit breaking
/// changes, this struct is `non_exhaustive`. See [GlobalArguments] for details on how to instantiate.
#[non_exhaustive]
#[derive(Default)]
pub struct RunArguments {
    pub executable: Option<String>,
    pub execution_path: Option<String>,
    pub variables: Vec<String>,
    pub service: Option<String>,
    pub input_bytes: Option<Vec<u8>>,
}

/// Simple wrapper error that displays the inner `eyre` [Report]. However, it is not directly accessible. Do
/// not try to match on its potential errors, simply directly display it.
#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
pub struct CommandError {
    pub msg: String,
    source: Report,
}

pub fn run_command(
    global_args: GlobalArguments,
    args: RunArguments,
) -> Result<EntrypointOut, CommandError> {
    inner_run_command(
        global_args.clone().into(),
        Some(global_args.into()),
        args.service,
        args.executable,
        args.execution_path,
        args.variables,
        args.input_bytes,
    )
    .map_err(|e| CommandError {
        msg: "An error occurred in the inner application layer.".to_owned(),
        source: e,
    })
}

#[non_exhaustive]
#[derive(Default)]
pub struct SecretArguments {
    pub key: String,
    pub service: Option<String>,
    pub prompt: Option<String>,
}

pub fn secret_command(
    global_args: GlobalArguments,
    args: SecretArguments,
) -> Result<String, CommandError> {
    inner_secret_command(
        global_args.clone().into(),
        Some(global_args.into()),
        args.service,
        args.key,
        args.prompt,
    )
    .map_err(|e| CommandError {
        msg: "An error occurred in the inner application layer.".to_owned(),
        source: e,
    })
}
