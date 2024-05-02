use color_eyre::eyre::{Context, Report};
use relative_path::RelativePathBuf;
use tracing::{debug, instrument};

use crate::{
    archives::extract_archive,
    filesystem::get_dirs,
    state::{create_state_create, create_state_run, CliEnvState},
};

use super::{
    process::{run_entrypoint, EntrypointOut},
    state::{create_state_run as create_state_run_next, resolve_paths, StateIn},
};

pub(crate) fn run_command(
    state_in: StateIn,
    executable: Option<String>,
    variables: Vec<String>,
) -> Result<EntrypointOut, Report> {
    run_command_input(state_in, executable, variables, None)
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

    run_entrypoint(state.deploy_dir(), &relative_path, state.envs, input_bytes)
}

#[instrument(name = "run", level = "debug", skip_all)]
pub(crate) fn run_command_input(
    state_in: StateIn,
    executable: Option<String>,
    variables: Vec<String>,
    input_bytes: Option<Vec<u8>>,
) -> Result<EntrypointOut, Report> {
    debug!("Run command called with in_state {:?}, executable {:?}, variables {:?} and input_bytes {:?}", state_in, executable, variables, input_bytes);
    
    let state = create_state_run_next(state_in, executable.as_deref(), variables)?;

    let resolved_paths = resolve_paths(state.paths);

    run_entrypoint(
        &resolved_paths.exe_dir,
        &resolved_paths.exe_path,
        state.envs,
        input_bytes,
    )
}
