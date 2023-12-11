use crate::config::load_dploy_config;
use crate::secret_store::{get_password, set_password};
use crate::secrets::SecretOutput;
use clap::{Parser, Subcommand, ValueEnum};
use keyring::Error as KeyringError;
use rpassword::prompt_password;
use spinoff::{spinners, Spinner};
use std::ffi::OsString;
use std::fs::{self};

use std::string::FromUtf8Error;
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

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Environment {
    /// Local development environment
    Localdev,
    /// Staging environment
    Staging,
    /// Production environment
    Production,
}

impl Environment {
    fn to_string(self) -> &'static str {
        match self {
            Self::Localdev => "localdev",
            Self::Staging => "staging",
            Self::Production => "production",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Stage {
    /// Download stage
    Download,
    /// Deploy stage
    Deploy,
}

impl Stage {
    fn to_string(self) -> &'static str {
        match self {
            Self::Download => "download",
            Self::Deploy => "deploy",
        }
    }
}

/// Deploy self-contained deploy units
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Download tag or version with specific env
    Download {
        /// Environment
        #[arg(value_enum)]
        env: Environment,

        /// Version or tag to download
        git_ref: Option<String>,

        /// Git repository URL, defaults to "origin" remote of current Git root, looks for TI_DPLOY_REPO_URL env variable if not set
        /// Set to 'git_root_origin' to ignore environment variable and only look for current repository origin
        #[arg(short, long, default_value = "default_git_root_origin")]
        repo: String,
    },

    /// Deploy tag or version with specific env
    Deploy {
        /// Environment
        #[arg(value_enum)]
        env: Environment,

        /// Version or tag to deploy. Omit to deploy latest for env
        git_ref: Option<String>,

        #[arg(short, long, default_value = "default_tidploy_git_root")]
        repo: String,

        /// Whether to get the latest version of the ref (default: true)
        #[arg(id = "latest", short, long, default_value_t = true)]
        latest_opt: bool,

        /// Whether to recreate the database (default: false)
        #[arg(short, long, default_value_t = false)]
        recreate: bool,
    },

    /// Save authentication details for specific stage until reboot
    Auth {
        #[arg(value_enum)]
        stage: Stage,

        #[arg(default_value = "default")]
        repo: String,
    },
}

static TMP_DIR: &str = "/tmp/ti_dploy";

fn env_tag_name(env: &str, tag: &str) -> String {
    format!("{}_{}", env, tag)
}

fn location(name: &str, env: &str, tag: &str) -> String {
    let env_tag_name = env_tag_name(env, tag);

    format!("{}/{}_{}", TMP_DIR, name, env_tag_name)
}

fn make_tmp_dir() -> Result<(), FileError> {
    let tmp_dir_path = Path::new(TMP_DIR);

    if tmp_dir_path.exists() {
        if tmp_dir_path.is_dir() {
            return Ok(());
        }

        fs::remove_file(tmp_dir_path)?;
    }

    fs::create_dir_all(tmp_dir_path)?;

    Ok(())
}

fn make_archive(
    source_dir_parent: &str,
    source_dir: &str,
    env: &str,
    tag: &str,
) -> Result<(), FileError> {
    let archives_dir = format!("{}/archives", TMP_DIR);
    let archives_path = Path::new(&archives_dir);
    if !archives_path.exists() {
        fs::create_dir_all(archives_path)?;
    }

    let archive_name = format!("{}.tar.gz", env_tag_name(env, tag));

    let archive_path = archives_path.join(&archive_name);
    let archive_loc = format!("{}/{}", &archives_dir, &archive_name);

    if archive_path.exists() {
        fs::remove_file(archive_path)?;
    }

    let mut output_archive_prog = Cmd::new("tar");
    let output_archive = output_archive_prog
        .current_dir(source_dir_parent)
        .arg("-czf")
        .arg(archive_loc)
        .arg(source_dir);

    output_archive.output()?;

    println!("Saved deploy archive in tmp.");

    Ok(())
}

fn checkout_tag(repo_loc: &str, git_ref: &str) -> Result<(), GitError> {
    let _checkout = Cmd::new("git")
        .current_dir(repo_loc)
        .arg("checkout")
        .arg("-f")
        .arg(git_ref)
        .output()?;

    Ok(())
}

#[derive(Debug, ThisError)]
enum FileError {
    #[error("IO failure for file!")]
    IO(#[from] IOError),
}

#[derive(Debug, ThisError)]
enum ProcessError {
    #[error("IO failure for external process!")]
    IO(#[from] IOError),
    #[error("Failure decoding process output!")]
    Decode(#[from] FromUtf8Error),
    #[error("Process had no output!")]
    NoOutput,
}

#[derive(Debug, ThisError)]
enum GitError {
    #[error("IO failure for external process!")]
    IO(#[from] IOError),
    #[error("Failure decoding Git output!")]
    Decode(#[from] FromUtf8Error),
}

#[derive(Debug, ThisError)]
enum RepoParseError {
    #[error("Failure getting origin name of current repository using Git!")]
    Git(#[from] GitError),
    #[error("Environment variable {0:?} cannot be parsed as Unicode string!")]
    BadEnvVar(OsString),
    #[error("Repo URL {0} doesn't end with /<name>.git and cannot be parsed!")]
    InvalidURL(String),
}

#[derive(Debug, ThisError)]
enum DownloadError {
    #[error("Failure parsing repo URL!")]
    RepoParse(#[from] RepoParseError),
    #[error("Failure preparing repo!")]
    Repo(#[from] RepoError),
    #[error("Failure during download dealing with files!")]
    File(#[from] FileError),
    #[error("Failure during download dealing with external process!")]
    Process(#[from] ProcessError),
}

#[derive(Debug, ThisError)]
enum DeployError {
    #[error("Failure parsing repo URL!")]
    RepoParse(#[from] RepoParseError),
    #[error("Failure preparing repo!")]
    Repo(#[from] RepoError),
    #[error("Failure downloading repo!")]
    Download(#[from] DownloadError),
    #[error("Failure getting or setting password!")]
    Auth(#[from] AuthError),
    #[error("Failure during download dealing with files!")]
    File(#[from] FileError),
    #[error("Failure during deploy dealing with external process!")]
    Process(#[from] ProcessError),
    #[error("Failed to load secrets!")]
    Secrets,
    #[error("Failed to parse secrets JSON!")]
    SecretsDecode(#[from] serde_json::Error),
}

#[derive(Debug)]
struct DeployObject {
    env: String,
    repo: String,
    git_ref: String,
}

#[derive(Debug, ThisError)]
enum RepoError {
    #[error("Failure during preparation dealing with files!")]
    File(#[from] FileError),
    #[error("Failure during preparation dealing with external process!")]
    Process(#[from] ProcessError),
    #[error("Failure during download dealing with Git!")]
    Git(#[from] GitError),
    #[error("Target repo {} does not contain deploy/use/{} at ref {}", .0.repo, .0.env, .0.git_ref)]
    DeployNotFound(DeployObject),
}

#[derive(ThisError, Debug)]
#[error(transparent)]
pub struct Error(#[from] ErrorRepr);

#[derive(ThisError, Debug)]
enum ErrorRepr {
    #[error("Auth failure.")]
    Auth(#[from] AuthError),

    #[error("Download failure.")]
    Download(#[from] DownloadError),

    #[error("Deploy failure.")]
    Deploy(#[from] DeployError),
}

fn prepare_repo(
    name: &str,
    repo_url: &str,
    env: &str,
    tag: &str,
    git_ref_opt: Option<String>,
    latest: bool,
) -> Result<(), RepoError> {
    let loc_str = location(name, env, tag);
    let repo_loc = format!("{}_repo", loc_str);

    let repo_path = Path::new(&repo_loc);
    let exists = repo_path.exists();

    if !exists {
        make_tmp_dir()?;
    }

    if !exists || git_ref_opt.is_none() {
        if exists {
            fs::remove_dir_all(repo_path).map_err(FileError::IO)?;
        }

        let mut sp = Spinner::new(spinners::Line, "Cloning repository...", None);

        let _repo_clone_stdout = Cmd::new("git")
            .arg("clone")
            .arg("--filter=tree:0")
            .arg(repo_url)
            .arg(&repo_loc)
            .stdout(Stdio::piped())
            .output()
            .map_err(GitError::IO)?;

        sp.success("Repository cloned!");

        if let Some(git_ref) = git_ref_opt.clone() {
            let mut sp = Spinner::new(spinners::Line, "Checking out ref...", None);

            checkout_tag(&repo_loc, &git_ref)?;

            sp.success("Checked out ref!");
        }
    } else if let Some(git_ref) = git_ref_opt.clone() {
        if exists && latest {
            let mut sp = Spinner::new(spinners::Line, "Checking out ref and updating...", None);

            checkout_tag(&repo_loc, &git_ref)?;

            // In case we were on a branch we now update to latest
            let _pull = Cmd::new("git")
                .current_dir(&repo_loc)
                .arg("pull")
                .output()
                .map_err(GitError::IO)?;

            checkout_tag(&repo_loc, &git_ref)?;

            sp.success("Checked out ref!");
        } else if exists {
            let mut sp = Spinner::new(spinners::Line, "Checking out ref...", None);

            checkout_tag(&repo_loc, &git_ref)?;

            sp.success("Checked out ref!");
        }
    }

    let use_dir = format!("{}/deploy/use", repo_loc);
    let use_path = Path::new(&use_dir);

    if !use_path.exists() {
        return Err(RepoError::DeployNotFound(DeployObject {
            env: env.to_owned(),
            repo: repo_url.to_owned(),
            git_ref: git_ref_opt.unwrap_or("none".to_owned()),
        }));
    }

    //make_archive(&use_dir, env, env, tag)?;

    Ok(())
}

fn extract(env: &str, tag: &str) -> Result<(), FileError> {
    let archives_dir = format!("{}/archives", TMP_DIR);
    let env_tag = env_tag_name(env, tag);
    let archive_name = format!("{}.tar.gz", &env_tag);

    let archive_loc = format!("{}/{}", &archives_dir, &archive_name);
    let target_dir = format!("{}/{}", TMP_DIR, env_tag);
    let target_path = Path::new(&target_dir);

    if target_path.exists() {
        fs::remove_dir_all(target_path)?;
    }

    make_tmp_dir()?;

    let mut tar_prog = Cmd::new("tar");

    // strip components might not work on every platform
    let tar_prog = tar_prog
        .arg("-xzf")
        .arg(archive_loc)
        .current_dir(TMP_DIR)
        .arg("-C")
        .arg(env_tag)
        .arg("--strip-components")
        .arg("1");

    tar_prog.output()?;

    println!("Extracted archive {}.", archive_name);

    Ok(())
}

fn get_password_env(env: Environment, name: &str, stage: Stage) -> Result<Option<String>, AuthError> {
    match env {
        Environment::Localdev => Ok(None),
        Environment::Staging | Environment::Production => match get_password(name, stage.to_string()) {
            Ok(None) => Err(AuthError::NoPassword),
            Ok(pw_some) => Ok(pw_some),
            Err(e) => Err(e.into()),
        },
    }
}

fn add_password_maybe<'a>(
    cmd: &'a mut Cmd,
    password_option: Option<String>,
    env_key: &str,
) -> &'a mut Cmd {
    match password_option {
        None => cmd,
        Some(password) => cmd.env(env_key, password),
    }
}

struct GitRepo {
    name: String,
    url: String,
}

fn git_root_origin_url() -> Result<String, GitError> {
    let git_origin_output = Cmd::new("git")
        .arg("config")
        .arg("--get")
        .arg("remote.origin.url")
        .output()
        .map_err(GitError::IO)?;

    if !git_origin_output.status.success() {
        panic!("Failed to get origin URL!")
    }

    Ok(String::from_utf8(git_origin_output.stdout)?
        .trim_end()
        .to_owned())
}

fn get_repo(repo_arg: String) -> Result<GitRepo, RepoParseError> {
    let repo_val = if repo_arg == "default_tidploy_git_root" {
        match env::var("TI_DPLOY_REPO_URL") {
            Ok(repo_var) => repo_var,
            Err(env::VarError::NotPresent) => "tidploy_git_root".to_owned(),
            Err(env::VarError::NotUnicode(env_str)) => {
                return Err(RepoParseError::BadEnvVar(env_str))
            }
        }
    } else {
        repo_arg
    };

    let url = if repo_val == "tidploy_git_root" {
        git_root_origin_url()?
    } else {
        repo_val
    };
    let split_parts: Vec<&str> = url.split('/').collect();
    let last_part = *split_parts
        .last()
        .ok_or(RepoParseError::InvalidURL(url.clone()))?;
    let split_parts_dot: Vec<&str> = last_part.split('.').collect();
    let name = (*split_parts_dot
        .first()
        .ok_or(RepoParseError::InvalidURL(url.clone()))?)
    .to_owned();
    Ok(GitRepo { name, url })
}

#[derive(ThisError, Debug)]
enum AuthError {
    #[error("Failed to get name from repo!")]
    RepoParse(#[from] RepoParseError),
    #[error("Failed to get password from prompt!")]
    Prompt(#[from] IOError),
    #[error("No password saved.")]
    NoPassword,
    #[error("Internal keyring failure.")]
    Keyring(#[from] KeyringError),
}

fn auth_command(stage: Stage, repo: String) -> Result<(), AuthError> {
    let git_repo = get_repo(repo)?;
    let password = prompt_password("Enter password:\n")?;
    set_password(&password, &git_repo.name, stage.to_string())?;
    Ok(println!("Set password!"))
}

fn download_command(
    env: Environment,
    git_ref: Option<String>,
    repo: String,
) -> Result<(), DownloadError> {
    let env_str = env.to_string();
    let tag = match &git_ref {
        Some(git_ref) => git_ref.clone(),
        None => "latest".to_owned(),
    };
    let git_repo = get_repo(repo)?;
    let loc_str = location(&git_repo.name, env_str, &tag);
    let repo_loc = format!("{}_repo", loc_str);
    let use_dir = format!("{}/deploy/use", &repo_loc);

    prepare_repo(&git_repo.name, &git_repo.url, env_str, &tag, git_ref, true)?;
    make_archive(&use_dir, env_str, env_str, &tag)?;
    extract(env_str, &tag)?;

    Ok(())
}

fn deploy_command(
    env: Environment,
    git_ref: Option<String>,
    latest_opt: bool,
    recreate: bool,
    repo: String,
) -> Result<(), DeployError> {
    let mut latest = latest_opt;
    if git_ref.is_none() && !latest {
        println!("No git ref is specified, setting latest to true!");
        latest = false;
    }
    let tag = match &git_ref {
        Some(git_ref) => git_ref.clone(),
        None => "latest".to_owned(),
    };

    let GitRepo {
        name,
        url: repo_url,
    } = get_repo(repo)?;

    let env_str = env.to_string();

    let archives_dir = format!("{}/archives", TMP_DIR);
    let env_tag = env_tag_name(env_str, &tag);
    let archive_name = format!("{}.tar.gz", &env_tag);
    let archive_loc = format!("{}/{}", &archives_dir, &archive_name);
    let loc_str = location(&name, env_str, &tag);
    let repo_loc = format!("{}_repo", loc_str);
    let use_dir = format!("{}/deploy/use", &repo_loc);
    let archive_path = Path::new(&archive_loc);

    // Always download if tag is latest
    let new_archive = !archive_path.exists() || latest;

    if new_archive {
        println!("Creating new archive...");
        prepare_repo(&name, &repo_url, env_str, &tag, git_ref, latest)?;
        make_archive(&use_dir, env_str, env_str, &tag)?;
    }

    extract(env_str, &tag)?;

    let loc_str = location(&name, env_str, &tag);

    let config_path = format!("{}/{}", &loc_str, "tidploy.toml");
    let mut dploy_config = load_dploy_config(&config_path);

    // in this case we are on the latest commit, but we need to go back to the correct commit of the latest release
    if latest && new_archive {
        // Redownload with correct tag
        prepare_repo(
            &name,
            &repo_url,
            env_str,
            &tag,
            Some(dploy_config.latest_ref()),
            true,
        )?;
        make_archive(&use_dir, env_str, env_str, &tag)?;
        // Reload config
        extract(env_str, &tag)?;
        dploy_config = load_dploy_config(&config_path);
    }

    println!("Running deploy.");

    let maybe_password = match get_password_env(env, &name, Stage::Deploy) {
        Err(AuthError::NoPassword) => {
            println!("Set password using `tidploy auth`!");
            return Ok(());
        }
        other => other,
    }?;

    let mut sp = Spinner::new(spinners::Line, "Loading secrets...", None);
    let mut secrets = HashMap::<String, String>::new();

    for id in dploy_config.get_secrets() {
        let mut run_secrets = Cmd::new("bws");
        let run_secrets =
            add_password_maybe(&mut run_secrets, maybe_password.clone(), "BWS_ACCESS_TOKEN")
                .arg("secret")
                .arg("get")
                .arg(&id);
        let output = run_secrets.output().map_err(ProcessError::IO)?;

        if !output.status.success() {
            if !output.stderr.is_empty() {
                println!(
                    "{}",
                    String::from_utf8(output.stderr).map_err(ProcessError::Decode)?
                );
            } else {
                println!("Error loading secrets: {:?}!", output.status)
            }
            return Err(DeployError::Secrets);
        }

        let secrets_output = String::from_utf8(output.stdout).map_err(ProcessError::Decode)?;

        let s_output: SecretOutput =
            serde_json::from_str(&secrets_output).map_err(DeployError::SecretsDecode)?;
        secrets.insert(s_output.key, s_output.value);
    }
    sp.success("Secrets loaded into environment!");

    let deploy_name = format!("{}-{}", env_str, &tag).replace('.', "_");

    let recreate_value = if recreate { "yes" } else { "no" };

    // TODO this is too specific logic
    let deploy_tag_suffix = if tag == "latest" {
        "".to_owned()
    } else {
        format!("-{}", &tag)
    };

    println!("Running entrypoint with deploy name {}...", &deploy_name);

    let mut entrypoint_output = Cmd::new(format!("{}/{}", &loc_str, "entrypoint.sh"))
        .current_dir(&loc_str)
        .envs(&secrets)
        .env("RECREATE", recreate_value)
        .env("DEPLOY_NAME", deploy_name)
        .env("DEPLOY_TAG_SUFFIX", &deploy_tag_suffix)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(ProcessError::IO)?;

    let entrypoint_stdout = entrypoint_output
        .stdout
        .take()
        .ok_or(ProcessError::NoOutput)?;

    let reader = BufReader::new(entrypoint_stdout);

    reader
        .lines()
        .map_while(Result::ok)
        .for_each(|line| println!("{}", line));

    let output_stderr = entrypoint_output
        .wait_with_output()
        .map_err(ProcessError::IO)?
        .stderr;
    if !output_stderr.is_empty() {
        println!(
            "{}",
            String::from_utf8(output_stderr).map_err(ProcessError::Decode)?
        );
    }
    Ok(())
}

pub(crate) fn run_cli() -> Result<(), Error> {
    let args = Args::parse();

    match args.command {
        Commands::Auth { stage, repo } => Ok(auth_command(stage, repo).map_err(ErrorRepr::from)?),
        Commands::Download { env, git_ref, repo } => {
            Ok(download_command(env, git_ref, repo).map_err(ErrorRepr::from)?)
        }
        Commands::Deploy {
            env,
            git_ref,
            latest_opt,
            recreate,
            repo,
        } => Ok(deploy_command(env, git_ref, latest_opt, recreate, repo).map_err(ErrorRepr::from)?),
    }
}
