use std::{collections::HashMap, env::current_dir, path::PathBuf};

use color_eyre::eyre::Report;

use tracing::{debug, span, Level};

use crate::config::ConfigVar;

use super::errors::SecretError;

/// Parses the list of strings given and interprets them as each pair of two being a secret key and target
/// env name.
fn parse_cli_vars(envs: Vec<String>) -> Vec<ConfigVar> {
    // Our chunk size is 2 so we know first and second exist
    // Any final element that does not have something to pair with will be ommitted
    envs.chunks_exact(2)
        .map(|c| ConfigVar {
            key: c.first().unwrap().to_owned(),
            env_name: c.get(1).unwrap().to_owned(),
        })
        .collect()
}

pub(crate) struct State {
    pub(crate) exe_name: String,
    pub(crate) path: PathBuf,
    pub(crate) envs: HashMap<String, String>,
}

fn secret_vars_to_envs(vars: Vec<ConfigVar>) -> Result<HashMap<String, String>, SecretError> {
    let mut envs = HashMap::<String, String>::new();
    for e in vars {
        debug!("NOT YET IMPLEMENTED Getting pass for {:?}", e);
        let pass = "notyet".to_owned();
        // let pass = get_secret(state, &e.key).map_err(|source| {
        //     let msg = format!("Failed to get password with key {} from passwords while loading envs into state!", e.key);
        //     AuthError { msg, source }
        // })?;

        envs.insert(e.env_name, pass);
    }
    Ok(envs)
}

/// Creates the state that is used to run the executable. Adds envs provided through CLI to `create_state`.
pub(crate) fn create_state_run(
    exe_name: Option<String>,
    envs: Vec<String>,
) -> Result<State, Report> {
    // Exits when the function returns
    let run_state_span = span!(Level::DEBUG, "run_state");
    let _enter = run_state_span.enter();

    let path = current_dir().unwrap();
    let exe_name = exe_name.unwrap();
    let secret_vars = parse_cli_vars(envs);
    let envs = secret_vars_to_envs(secret_vars).unwrap();
    Ok(State {
        exe_name,
        path,
        envs,
    })
}
