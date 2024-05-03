use color_eyre::eyre::{Context, Report};
use relative_path::RelativePathBuf;
use tracing::{debug, instrument};

use crate::{
    archives::extract_archive,
    filesystem::get_dirs,
    next::{
        resolve::{merge_and_resolve, RunArguments, SecretScopeArguments},
        secrets::secret_vars_to_envs,
        state::{create_resolve_state, parse_cli_vars},
    },
    state::{create_state_create, create_state_run, CliEnvState},
};

use super::{
    process::{run_entrypoint, EntrypointOut},
    resolve::RunResolved,
    state::StateIn,
};

pub(crate) fn run_command(
    state_in: StateIn,
    service: Option<String>,
    executable: Option<String>,
    execution_path: Option<String>,
    variables: Vec<String>,
) -> Result<EntrypointOut, Report> {
    run_command_input(state_in, service, executable, execution_path, variables, None)
}

#[instrument(name = "run", level = "debug", skip_all)]
pub(crate) fn run_command_input_old_state(
    cli_state: CliEnvState,
    executable: Option<String>,
    variables: Vec<String>,
    archive: Option<String>,
    input_bytes: Option<Vec<u8>>,
) -> Result<EntrypointOut, Report> {
    // Only loads archive if it is given, otherwise path is None
    let state = if let Some(archive) = archive {
        let cache_dir = get_dirs().cache.as_path();
        let archive_path = cache_dir
            .join("archives")
            .join(&archive)
            .with_extension("tar.gz");

        let tmp_dir = get_dirs().tmp.as_path();
        let extracted_path =
            extract_archive(&archive_path, tmp_dir, &archive).wrap_err("Repo error.")?;
        debug!("Extracted and loaded archive at {:?}", &extracted_path);

        let state = create_state_create(cli_state.clone(), Some(&extracted_path), None, true)
            .wrap_err("Load error.")?;

        Some(state)
    } else {
        debug!("No archive provided to run command.");
        None
    };
    let root_dir = state.as_ref().map(|state| state.root_dir.as_path());
    let deploy_path = state
        .as_ref()
        .map(|state| state.deploy_path.as_relative_path());

    let state = create_state_run(
        cli_state,
        executable,
        variables,
        root_dir,
        deploy_path,
        true,
    )
    .wrap_err("Create state error.")?;

    // let state = extra_envs(state);

    let relative_path = RelativePathBuf::from(&state.exe_name);
    let exe_path = relative_path.to_path(state.deploy_dir());
    run_entrypoint(&state.deploy_dir(), &exe_path, state.envs, input_bytes)
}

#[instrument(name = "run", level = "debug", skip_all)]
pub(crate) fn run_command_input(
    state_in: StateIn,
    service: Option<String>,
    executable: Option<String>,
    execution_path: Option<String>,
    variables: Vec<String>,
    input_bytes: Option<Vec<u8>>,
) -> Result<EntrypointOut, Report> {
    debug!("Run command called with in_state {:?}, executable {:?}, variables {:?} and input_bytes {:?}", state_in, executable, variables, input_bytes);

    let scope_args = SecretScopeArguments {
        service,
        ..Default::default()
    };
    let run_args = RunArguments {
        executable,
        execution_path,
        envs: parse_cli_vars(variables),
        scope_args,
    };
    let resolve_state = create_resolve_state(state_in)?;

    let run_resolved = merge_and_resolve(run_args, resolve_state)?;

    run_unit_input(run_resolved, input_bytes)
}

pub(crate) fn run_unit_input(
    run_resolved: RunResolved,
    input_bytes: Option<Vec<u8>>,
) -> Result<EntrypointOut, Report> {
    //debug!("Run command called with in_state {:?}, executable {:?}, variables {:?} and input_bytes {:?}", state_in, executable, variables, input_bytes);

    let secret_vars = secret_vars_to_envs(&run_resolved.scope, run_resolved.envs)?;

    run_entrypoint(
        &run_resolved.execution_path,
        &run_resolved.executable,
        secret_vars,
        input_bytes,
    )
}
