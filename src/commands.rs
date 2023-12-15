use std::path::{Path, PathBuf};

use crate::archives::{extract_archive, make_archive};
use crate::auth::{auth_command, AuthError};
use crate::errors::{ProcessError, RepoError};
use crate::git::{checkout, checkout_path, repo_clone};

use crate::process::run_entrypoint;
use crate::state::{
    create_state_create, create_state_run, CliEnvState, LoadError, State, StateContext,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64USNP;
use base64::Engine;
use clap::{Parser, Subcommand};
use std::time::Instant;
use thiserror::Error as ThisError;

pub(crate) const DEFAULT_INFER: &str = "default_infer";
pub(crate) const TIDPLOY_DEFAULT: &str = "_tidploy_default";
pub(crate) const DEFAULT: &str = "default";
pub(crate) const TMP_DIR: &str = "/tmp/tidploy";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, value_enum, global = true)]
    context: Option<StateContext>,

    #[arg(long, global = true)]
    network: Option<bool>,

    /// Set the repository URL, defaults to 'default_infer', in which case it is inferred from the current repository. Set to 'default' to not set it.
    /// Falls back to environment variable using TIDPLOY_REPO and then to config with key 'repo_url'
    /// For infering, it looks at the URL set to the 'origin' remote
    #[arg(short, long, global = true)]
    repo: Option<String>,

    #[arg(short, long, global = true)]
    tag: Option<String>,

    #[arg(short, long, global = true)]
    deploy_pth: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Save authentication details for specific stage until reboot
    Auth { key: String },
    /// Download tag or version with specific env, run automatically if using deploy
    Download {
        #[arg(long)]
        repo_only: bool,
    },

    /// Deploy tag or version with specific env
    Deploy {
        #[arg(short = 'x', long = "exe", default_value = "_tidploy_default")]
        executable: String,

        #[arg(short, num_args = 2)]
        variables: Vec<String>,
    },
    /// Run an entrypoint using the password set for a specific repo and stage 'deploy', can be used after download
    Run {
        #[arg(short = 'x', long = "exe", default_value = "_tidploy_default")]
        executable: String,

        #[arg(short, num_args = 2)]
        variables: Vec<String>,

        #[arg(long)]
        archive: Option<String>,
    },
}

#[derive(ThisError, Debug)]
#[error(transparent)]
pub struct Error(#[from] ErrorRepr);

#[derive(ThisError, Debug)]
enum ErrorRepr {
    #[error("Load error failure! {0}")]
    Load(#[from] LoadError),
    #[error("Auth failure! {0}")]
    Auth(#[from] AuthError),
    #[error("Error running executable! {0}")]
    Exe(#[from] ProcessError),
    #[error("Error creating repository! {0}")]
    Repo(#[from] RepoError),
}

fn create_repo(state: State) -> Result<PathBuf, RepoError> {
    if !state.network {
        return Err(RepoError::NeedsNetwork);
    }
    let repo_name = format!("{}_{}", state.repo.name, state.repo.encoded_url);
    let tmp_dir = Path::new(TMP_DIR);
    let repo_path = tmp_dir.join(&repo_name);
    repo_clone(tmp_dir, &repo_name, &state.repo.url)?;

    Ok(repo_path)
}

fn download_command(
    cli_state: CliEnvState,
    state: State,
    repo_only: bool,
) -> Result<Option<State>, ErrorRepr> {
    let repo_path = create_repo(state).map_err(ErrorRepr::Repo)?;

    if repo_only {
        return Ok(None);
    }

    let state =
        create_state_create(cli_state.clone(), Some(&repo_path), true).map_err(ErrorRepr::Load)?;

    checkout(&repo_path, &state.commit_sha).map_err(ErrorRepr::Repo)?;

    checkout_path(&repo_path, &state.deploy_path).map_err(ErrorRepr::Repo)?;

    let deploy_path = state.deploy_path.to_path(&repo_path);

    let state =
        create_state_create(cli_state, Some(&deploy_path), true).map_err(ErrorRepr::Load)?;

    let tmp_dir = Path::new(TMP_DIR);
    let archives = tmp_dir.join("archives");
    let deploy_encoded = B64USNP.encode(state.deploy_path.as_str());
    let archive_name = format!(
        "{}_{}_{}",
        state.repo.name, state.commit_sha, deploy_encoded
    );

    make_archive(
        &archives,
        tmp_dir,
        repo_path.file_name().unwrap().to_string_lossy().as_ref(),
        &archive_name,
    )
    .map_err(ErrorRepr::Repo)?;

    Ok(Some(state))
}

fn extra_envs(mut state: State) -> State {
    let commit_long = state.commit_sha.clone();
    let commit_short = state.commit_sha[0..7].to_owned();

    state.envs.insert("TIDPLOY_SHA".to_owned(), commit_short);
    state
        .envs
        .insert("TIDPLOY_SHA_LONG".to_owned(), commit_long);
    state
        .envs
        .insert("TIDPLOY_TAG".to_owned(), state.tag.clone());

    state
}

pub(crate) fn run_cli() -> Result<(), Error> {
    let _now = Instant::now();

    let args = Cli::parse();

    let cli_state = CliEnvState {
        context: args.context,
        network: args.network,
        repo_url: args.repo,
        deploy_path: args.deploy_pth,
        tag: args.tag,
    };

    match args.command {
        Commands::Auth { key } => {
            let state = create_state_create(cli_state, None, false).map_err(ErrorRepr::Load)?;

            auth_command(&state, key).map_err(ErrorRepr::Auth)?;

            Ok(())
        }
        Commands::Download { repo_only } => {
            let state =
                create_state_create(cli_state.clone(), None, false).map_err(ErrorRepr::Load)?;
            download_command(cli_state, state, repo_only)?;

            Ok(())
        }
        Commands::Deploy {
            executable,
            variables,
        } => {
            let state =
                create_state_create(cli_state.clone(), None, false).map_err(ErrorRepr::Load)?;
            let state = download_command(cli_state.clone(), state, false)?.unwrap();
            let tmp_dir = Path::new(TMP_DIR);
            let deploy_encoded = B64USNP.encode(state.deploy_path.as_str());
            let archive_name = format!(
                "{}_{}_{}",
                state.repo.name, state.commit_sha, deploy_encoded
            );
            let archive_path = tmp_dir
                .join("archives")
                .join(&archive_name)
                .with_extension("tar.gz");
            extract_archive(&archive_path, tmp_dir, &archive_name).map_err(ErrorRepr::Repo)?;
            let target_path_root = tmp_dir.join(archive_name);
            let target_path = state.deploy_path.to_path(target_path_root);
            let state = create_state_run(
                cli_state,
                Some(executable),
                variables,
                Some(&target_path),
                true,
            )
            .map_err(ErrorRepr::Load)?;

            let state = extra_envs(state);

            run_entrypoint(state.current_dir, &state.exe_name, state.envs)
                .map_err(ErrorRepr::Exe)?;

            Ok(())
        }
        Commands::Run {
            executable,
            variables,
            archive,
        } => {
            let path = if let Some(archive) = archive {
                let tmp_dir = Path::new(TMP_DIR);
                let archive_path = tmp_dir
                    .join("archives")
                    .join(&archive)
                    .with_extension("tar.gz");
                // Running archives doesn't fully work as it doesn't yet know how to retrieve the deploy path
                // Splitting by '_' doesn't work easily as base64url includes '_' chars

                extract_archive(&archive_path, tmp_dir, &archive).map_err(ErrorRepr::Repo)?;
                Some(tmp_dir.join(&archive))
            } else {
                None
            };

            let path_ref = path.as_deref();

            let state = create_state_run(cli_state, Some(executable), variables, path_ref, true)
                .map_err(ErrorRepr::Load)?;

            let state = extra_envs(state);

            run_entrypoint(state.current_dir, &state.exe_name, state.envs)
                .map_err(ErrorRepr::Exe)?;

            Ok(())
        }
    }
}
