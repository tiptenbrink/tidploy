
use crate::config::{load_dploy_config, DployConfig, traverse_configs, ConfigError};
use crate::errors::{GitError, ProcessError, RelPathError};
use crate::filesystem::{FileError, get_current_dir};
use crate::git::{git_root_origin_url, relative_to_git_root};
use crate::secret_store::{get_password, set_password};
use crate::secrets::SecretOutput;
use crate::state::{StateContext, LoadError};
use clap::{Parser, Subcommand, ValueEnum};
use keyring::Error as KeyringError;
use rpassword::prompt_password;
use spinoff::{spinners, Spinner};
use std::env::VarError;
use std::ffi::OsString;
use std::fs::{self};
use std::path::PathBuf;
use std::process::Output;
use std::{
    collections::HashMap,
    env,
    io::BufRead,
    io::BufReader,
    io::Error as IOError,
    path::Path,
    process::{Command as Cmd, Stdio},
};
use thiserror::Error as ThisError;
use relative_path::RelativePathBuf;

pub(crate) const DEFAULT_INFER: &str = "default_infer";
pub(crate) const TIDPLOY_DEFAULT: &str = "_tidploy_default";
pub(crate) const DEFAULT: &str = "default";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, value_enum)]
    context: Option<StateContext>,

    #[arg(long)]
    no_network: Option<bool>,

    #[arg(short, long)]
    repo: Option<String>,

    #[arg(short, long)]
    tag: Option<String>,

    #[arg(short, long)]
    deploy_pth: Option<String>,

    
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Save authentication details for specific stage until reboot
    Auth {
        key: String
    },
    
    // /// Download tag or version with specific env, run automatically if using deploy
    // Download,

    // /// Deploy tag or version with specific env
    // Deploy {
    //     #[arg(long = "exe", default_value = "default")]
    //     executable: String,

    //     #[arg(short)]
    //     variables: Vec<String>
    // },


    // /// Run an entrypoint using the password set for a specific repo and stage 'deploy', can be used after download
    // Run {
    //     #[arg(long = "exe", default_value = "_tidploy_default")]
    //     executable: String,

    //     #[arg(short)]
    //     variables: Vec<String>
    // },
}

#[derive(ThisError, Debug)]
#[error(transparent)]
pub struct Error(#[from] ErrorRepr);

#[derive(ThisError, Debug)]
enum ErrorRepr {
    #[error("Load error failure! {0}")]
    Load(#[from] LoadError)
}





pub(crate) fn run_cli() -> Result<(), Error> {
    let args = Cli::parse();

    let state = create_state(args.context, args.use_network, args.repo, args.tag, args.deploy_pth);

    match args.command {
        Commands::Auth { key } => Ok(),
    }
}