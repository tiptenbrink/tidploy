use std::path::Path;

use color_eyre::eyre::Report;
use relative_path::RelativePath;
use tracing::{debug, span, Level};

use crate::{config::ConfigVar, state::{create_state, CliEnvRunState, CliEnvState, State}};

/// Parses the list of strings given and interprets them as each pair of two being a secret key and target
/// env name.
fn parse_cli_envs(envs: Vec<String>) -> Vec<ConfigVar> {
    // Our chunk size is 2 so we know first and second exist
    // Any final element that does not have something to pair with will be ommitted
    envs.chunks_exact(2)
        .map(|c| ConfigVar {
            key: c.first().unwrap().to_owned(),
            env_name: c.get(1).unwrap().to_owned(),
        })
        .collect()
}

/// Creates the state that is used to run the executable. Adds envs provided through CLI to `create_state`.
pub(crate) fn create_state_run(
    cli_state: CliEnvState,
    exe_name: Option<String>,
    envs: Vec<String>,
    path: Option<&Path>,
    deploy_path: Option<&RelativePath>,
    load_tag: bool,
) -> Result<State, Report> {
    // Exits when the function returns
    let run_state_span = span!(Level::DEBUG, "run_state");
    let _enter = run_state_span.enter();

    let cli_run_state = CliEnvRunState {
        exe_name,
        envs: parse_cli_envs(envs),
    };
    debug!("Parsed CLI envs as {:?}", cli_run_state);
    Ok(create_state(cli_state, Some(cli_run_state), path, deploy_path, load_tag)?)
}