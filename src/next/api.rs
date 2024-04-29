use super::run::{run_command as inner_run_command};
use crate::state::CliEnvState;
pub use crate::state::StateContext;
use color_eyre::eyre::Report;
use thiserror::Error as ThisError;

#[non_exhaustive]
pub struct GlobalArguments {
    pub context: Option<StateContext>,
    pub repo_url: Option<String>,
    pub deploy_path: Option<String>,
    pub tag: Option<String>,
}

impl Default for GlobalArguments {
    fn default() -> Self {
        GlobalArguments {
            context: None,
            repo_url: None,
            deploy_path: None,
            tag: None
        }
    }
}

impl GlobalArguments {
    pub fn cli_env(context: Option<StateContext>, repo_url: Option<String>, deploy_path: Option<String>, tag: Option<String>) -> Self {
        GlobalArguments {
            context,
            repo_url,
            deploy_path,
            tag
        }
    }
}

#[non_exhaustive]
pub struct RunArguments {
    pub executable: Option<String>,
    pub variables: Vec<String>,
    pub archive: Option<String>
}

impl Default for RunArguments {
    fn default() -> Self {
        RunArguments {
            executable: None,
            variables: Vec::new(),
            archive: None
        }
    }
}

impl RunArguments {
    pub fn with(executable: Option<String>, variables: Vec<String>, archive: Option<String>) -> Self {
        RunArguments {
            executable,
            variables,
            archive
        }
    }
}

impl From<GlobalArguments> for CliEnvState {
    fn from(args: GlobalArguments) -> Self {
        CliEnvState {
            context: args.context,
            repo_url: args.repo_url,
            deploy_path: args.deploy_path,
            tag: args.tag,
        }
    }
}

#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
pub struct CommandError {
    msg: String,
    source: Report
}

pub fn run_command(global_args: GlobalArguments, args: RunArguments) -> Result<(), CommandError> {
    inner_run_command(global_args.into(), args.executable, args.variables, args.archive).map_err(|e|
    CommandError {
        msg: "An error occurred in the inner application layer.".to_owned(),
        source: e
    })
}