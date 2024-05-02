use super::run::run_command_input as inner_run_command;
use super::secrets::secret_command as inner_secret_command;
use super::state::StateIn;
use crate::state::CliEnvState;
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
/// global_args.service = Some("tidpoy_test_service".to_owned());
/// ```
#[non_exhaustive]
#[derive(Default)]
pub struct GlobalArguments {
    //pub context: Option<StateContext>,
    // pub repo_url: Option<String>,
    // pub deploy_path: Option<String>,
    // pub tag: Option<String>,
    pub service: Option<String>,
}

impl From<GlobalArguments> for StateIn {
    fn from(value: GlobalArguments) -> Self {
        Self {
            service: value.service
        }
    }
}

/// These represent arguments that correspond to args of the CLI `run` subcommand. To limit breaking
/// changes, this struct is `non_exhaustive`. See [GlobalArguments] for details on how to instantiate.
#[non_exhaustive]
#[derive(Default)]
pub struct RunArguments {
    pub executable: Option<String>,
    pub variables: Vec<String>,
    // pub archive: Option<String>,
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
    inner_run_command(global_args.into(), args.executable, args.variables, args.input_bytes).map_err(|e| CommandError {
        msg: "An error occurred in the inner application layer.".to_owned(),
        source: e,
    })
}


#[non_exhaustive]
#[derive(Default)]
pub struct SecretArguments {
    pub key: String,
    pub prompt: Option<String>
}

pub fn secret_command(
    global_args: GlobalArguments,
    args: SecretArguments,
) -> Result<String, CommandError> {
    inner_secret_command(global_args.into(), &args.key, args.prompt).map_err(|e| CommandError {
        msg: "An error occurred in the inner application layer.".to_owned(),
        source: e,
    })
}