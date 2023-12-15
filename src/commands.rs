use crate::auth::{auth_command, AuthError};
use crate::errors::ProcessError;
use crate::process::run_entrypoint;

use crate::state::{create_state_pre, create_state_run, CliEnvState, LoadError, StateContext};
use clap::{Parser, Subcommand};

// use std::time::Instant;
use thiserror::Error as ThisError;

pub(crate) const DEFAULT_INFER: &str = "default_infer";
pub(crate) const TIDPLOY_DEFAULT: &str = "_tidploy_default";
pub(crate) const DEFAULT: &str = "default";

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
    // /// Download tag or version with specific env, run automatically if using deploy
    // Download,

    // /// Deploy tag or version with specific env
    // Deploy {
    //     #[arg(long = "exe", default_value = "default")]
    //     executable: String,

    //     #[arg(short)]
    //     variables: Vec<String>
    // },
    /// Run an entrypoint using the password set for a specific repo and stage 'deploy', can be used after download
    Run {
        #[arg(short = 'x', long = "exe", default_value = "_tidploy_default")]
        executable: String,

        #[arg(short, num_args = 2)]
        variables: Vec<String>,
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
    #[error("Error unning executable! {0}")]
    Exe(#[from] ProcessError),
}

pub(crate) fn run_cli() -> Result<(), Error> {
    //let now = Instant::now();

    let args = Cli::parse();

    //println!("{:?}", args);

    let cli_state = CliEnvState {
        context: args.context,
        network: args.network,
        repo_url: args.repo,
        deploy_path: args.deploy_pth,
        tag: args.tag,
    };

    match args.command {
        Commands::Auth { key } => {
            let state = create_state_pre(cli_state).map_err(ErrorRepr::Load)?;
            //println!("{:?}", state);
            //println!("time {}", now.elapsed().as_secs_f64());

            Ok(auth_command(&state, key).map_err(ErrorRepr::Auth)?)
        }
        Commands::Run {
            executable,
            variables,
        } => {
            let state = create_state_run(cli_state, Some(executable), variables, false)
                .map_err(ErrorRepr::Load)?;
            //println!("{:?}", state);
            //println!("time {}", now.elapsed().as_secs_f64());

            run_entrypoint(state.current_dir, &state.exe_name, state.envs)
                .map_err(ErrorRepr::Exe)?;

            Ok(())
        }
    }
}
