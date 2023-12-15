use crate::config::{load_dploy_config, ConfigError};
use crate::errors::{FileError, GitError, ProcessError};
use crate::secret_store::{get_password, set_password};
use crate::secrets::SecretOutput;
use clap::{Parser, Subcommand, ValueEnum};
use keyring::Error as KeyringError;
use rpassword::prompt_password;
use spinoff::{spinners, Spinner};
use std::ffi::OsString;
use std::fs::{self};
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

fn possible_env_strings() -> Vec<Environment> {
    vec![Environment::Localdev, Environment::Staging, Environment::Production]
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
    /// Save authentication details for specific stage until reboot
    Auth {
        /// 'download' stage is for downloading repository, 'deploy' stage (default) is for running deploy unit entrypoint
        #[arg(value_enum, default_value_t = Stage::Deploy)]
        stage: Stage,

        /// Git repository URL, defaults to "origin" remote of current Git root, looks for TI_DPLOY_REPO_URL env variable if not set.
        /// Set to 'git_root_origin' to ignore environment variable and only look for current repository origin
        #[arg(short, long, default_value = "default_git_root_origin")]
        repo: String,
    },
    
    /// Download tag or version with specific env, run automatically if using deploy
    Download {
        /// Environment
        #[arg(value_enum)]
        env: Environment,

        /// Version or tag to download. Omit to deploy latest for env
        git_ref: Option<String>,

        /// Git repository URL, defaults to "origin" remote of current Git root, looks for TI_DPLOY_REPO_URL env variable if not set.
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

        /// Git repository URL, defaults to "origin" remote of current Git root, looks for TI_DPLOY_REPO_URL env variable if not set.
        /// Set to 'git_root_origin' to ignore environment variable and only look for current repository origin
        #[arg(short, long, default_value = "default_git_root_origin")]
        repo: String,

        /// Whether to get the latest version of the ref (default: true)
        #[arg(id = "latest", short, long, default_value_t = true)]
        latest_opt: bool,

        /// Whether to recreate the database (default: false)
        #[arg(short = 'c', long, default_value_t = false)]
        recreate: bool,
    },


    /// Run an entrypoint using the password set for a specific repo and stage 'deploy', can be used after download
    Run {
        /// Run executable in the current directory with this name, conflicts with 'git_ref'
        #[arg(short, long, conflicts_with_all = ["git_ref"])]
        program: Option<String>,

        /// Environment.
        #[arg(value_enum)]
        env: Option<Environment>,
        
        /// Version or tag to download
        git_ref: Option<String>,

        /// Name of environment variable to set password to, defaults to looking at tidploy for 'env_var'
        #[arg(short, long)]
        env_var: Option<String>,

        /// Git repository URL, defaults to "origin" remote of current Git root, looks for TI_DPLOY_REPO_URL env variable if not set.
        /// Set to 'git_root_origin' to ignore environment variable and only look for current repository origin
        #[arg(short, long, default_value = "default_git_root_origin")]
        repo: String,
    }
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
    name: &str,
    env: &str,
    tag: &str,
) -> Result<(), FileError> {
    let archives_dir = format!("{}/archives", TMP_DIR);
    let archives_path = Path::new(&archives_dir);
    if !archives_path.exists() {
        fs::create_dir_all(archives_path)?;
    }

    let archive_name = format!("{}_{}.tar.gz", name, env_tag_name(env, tag));

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

#[derive(Debug)]
struct DeployObject {
    env: String,
    repo: String,
    git_ref: String,
}

#[derive(Debug, ThisError)]
enum RepoError {
    #[error("Failure during preparation dealing with files! {0}")]
    File(#[from] FileError),
    #[error("Failure during preparation dealing with external process! {0}")]
    Process(#[from] ProcessError),
    #[error("Failure during download dealing with Git! {0}")]
    Git(#[from] GitError),
    #[error("Target repo {} does not contain deploy/use/{} at ref {}", .0.repo, .0.env, .0.git_ref)]
    DeployNotFound(DeployObject),
}

#[derive(Debug, ThisError)]
enum RepoParseError {
    #[error("Failure getting origin name of current repository using Git! {0}")]
    Git(#[from] GitError),
    #[error("Environment variable {0:?} cannot be parsed as Unicode string!")]
    BadEnvVar(OsString),
    #[error("Repo URL {0} doesn't end with /<name>.git and cannot be parsed!")]
    InvalidURL(String),
}

#[derive(ThisError, Debug)]
enum AuthError {
    #[error("Failed to get name from repo! {0}")]
    RepoParse(#[from] RepoParseError),
    #[error("Failed to get password from prompt! {0}")]
    Prompt(#[from] IOError),
    #[error("No password saved.")]
    NoPassword,
    #[error("Internal keyring failure. {0}")]
    Keyring(#[from] KeyringError),
}

#[derive(Debug, ThisError)]
enum DownloadError {
    #[error("Failure parsing repo URL! {0}")]
    RepoParse(#[from] RepoParseError),
    #[error("Failure preparing repo! {0}")]
    Repo(#[from] RepoError),
    #[error("Failure during download dealing with files! {0}")]
    File(#[from] FileError),
    #[error("Failure during download dealing with external process! {0}")]
    Process(#[from] ProcessError),
}

#[derive(Debug, ThisError)]
enum DeployError {
    #[error("Failure parsing repo URL! {0}")]
    RepoParse(#[from] RepoParseError),
    #[error("Failure preparing repo! {0}")]
    Repo(#[from] RepoError),
    #[error("Failure downloading repo! {0}")]
    Download(#[from] DownloadError),
    #[error("Failure getting or setting password! {0}")]
    Auth(#[from] AuthError),
    #[error("Failure reading config! {0}")]
    Config(#[from] ConfigError),
    #[error("Failure during download dealing with files! {0}")]
    File(#[from] FileError),
    #[error("Failure during deploy dealing with external process! {0}")]
    Process(#[from] ProcessError),
    #[error("Failed to parse secrets JSON! {0}")]
    SecretsDecode(#[from] serde_json::Error),
    #[error("Current directory cannot be interpreted as an environment!")]
    EnvError
}

#[derive(ThisError, Debug)]
#[error(transparent)]
pub struct Error(#[from] ErrorRepr);

#[derive(ThisError, Debug)]
enum ErrorRepr {
    #[error("Auth failure. {0}")]
    Auth(#[from] AuthError),

    #[error("Download failure. {0}")]
    Download(#[from] DownloadError),

    #[error("Deploy failure. {0}")]
    Deploy(#[from] DeployError),
}

fn output_check_success(output: &Output) -> Result<(), ProcessError> {
    if !output.status.success() {
        if !output.stderr.is_empty() {
            println!(
                "{}",
                String::from_utf8(output.stderr.clone()).map_err(ProcessError::Decode)?
            );
        } else {
            println!("Stderr is empty!");
        }
        return Err(ProcessError::Failed(output.status));
    }
    Ok(())
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

fn extract(name: &str, env: &str, tag: &str) -> Result<(), FileError> {
    let archives_dir = format!("{}/archives", TMP_DIR);
    let env_tag = env_tag_name(env, tag);
    let name_env_tag = format!("{}_{}", name, &env_tag);
    let archive_name = format!("{}.tar.gz", &name_env_tag);

    let archive_loc = format!("{}/{}", &archives_dir, &archive_name);
    let target_dir = format!("{}/{}", TMP_DIR, &name_env_tag);
    let target_path = Path::new(&target_dir);

    if target_path.exists() {
        fs::remove_dir_all(target_path)?;
    }

    fs::create_dir_all(target_path)?;

    let mut tar_prog = Cmd::new("tar");

    // strip components might not work on every platform
    let tar_prog = tar_prog
        .arg("-xzf")
        .arg(archive_loc)
        .current_dir(TMP_DIR)
        .arg("-C")
        .arg(name_env_tag)
        .arg("--strip-components")
        .arg("1");

    let output = tar_prog.output()?;

    output_check_success(&output)?;

    println!("Extracted archive {}.", archive_name);

    Ok(())
}

fn get_password_env(
    env: Environment,
    name: &str,
    stage: Stage,
) -> Result<Option<String>, AuthError> {
    match env {
        Environment::Localdev => Ok(None),
        Environment::Staging | Environment::Production => {
            match get_password(name, stage.to_string()) {
                Ok(None) => Err(AuthError::NoPassword),
                Ok(pw_some) => Ok(pw_some),
                Err(e) => Err(e.into()),
            }
        }
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
    let repo_val = if repo_arg == "default_git_root_origin" {
        match env::var("TI_DPLOY_REPO_URL") {
            Ok(repo_var) => repo_var,
            Err(env::VarError::NotPresent) => "git_root_origin".to_owned(),
            Err(env::VarError::NotUnicode(env_str)) => {
                return Err(RepoParseError::BadEnvVar(env_str))
            }
        }
    } else {
        repo_arg
    };

    let url = if repo_val == "git_root_origin" {
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

fn auth_command(stage: Stage, repo: String) -> Result<(), AuthError> {
    let git_repo = get_repo(repo)?;
    let password = prompt_password("Enter password:\n")?;
    set_password(&password, &git_repo.name, stage.to_string())?;
    Ok(println!(
        "Set password for stage {} and repo {}!",
        &stage.to_string(),
        &git_repo.name
    ))
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
    make_archive(&use_dir, env_str, &git_repo.name, env_str, &tag)?;
    extract(&git_repo.name, env_str, &tag)?;

    Ok(())
}

fn guess_env(dir_name: &str) -> Option<Environment> {
    let possible = possible_env_strings();
    for s in possible {
        if dir_name.contains(s.to_string()) {
            return Some(s)
        }
    }

    None
}

fn tag_from_git_ref(git_ref: Option<String>) -> String {
    match &git_ref {
        Some(git_ref) => git_ref.clone(),
        None => "latest".to_owned(),
    }
}

fn run_entrypoint<P: AsRef<Path>>(
    entrypoint_dir: P,
    entrypoint: &str,
    envs: HashMap<String, String>,
) -> Result<(), ProcessError> {
    println!("Running {}!", &entrypoint);
    let program_path = entrypoint_dir.as_ref().join(entrypoint);
    let mut entrypoint_output = Cmd::new(program_path)
        .current_dir(&entrypoint_dir)
        .envs(&envs)
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


fn run_command(program: Option<String>, env: Option<Environment>, git_ref: Option<String>, repo: String, env_var: Option<String>) -> Result<(), DeployError> {
    let mut envs_map = HashMap::<String, String>::new();
    let GitRepo {
        name,
        url: repo_url,
    } = get_repo(repo)?;

    let current_dir = env::current_dir().map_err(FileError::IO)?;
    let current_dir_name = current_dir.file_name().unwrap().to_string_lossy().to_string();

    let env = env.map_or_else(|| guess_env(&current_dir_name), Some).ok_or(DeployError::EnvError)?;
    
    let mut dploy_config = None;

    let env_var = env_var.map_or_else(|| {
        let loaded_dploy_config = load_dploy_config(&current_dir)?;
        
        let res = loaded_dploy_config.get_env_var().ok_or(ConfigError::NoEnvVar)?;

        dploy_config = Some(loaded_dploy_config);

        Ok::<String, ConfigError>(res)
    }, Ok)?;

    let maybe_password = match get_password_env(env, &name, Stage::Deploy) {
        Err(AuthError::NoPassword) => {
            println!("No password found for stage {} and repo URL {}. Set password using `tidploy auth`!", "deploy", &repo_url);
            return Ok(());
        }
        other => other,
    }?;
    let password = maybe_password.clone().ok_or(AuthError::NoPassword)?;
    envs_map.insert(env_var, password);

    if let Some(program) = program {
        return Ok(run_entrypoint(&current_dir, &program, envs_map)?);
    }

    let env_str = env.to_string();

    let loc_str = location(&name, env_str, &tag_from_git_ref(git_ref));

    let dploy_config = dploy_config.map_or_else(|| load_dploy_config(&current_dir) , Ok)?;

    let entrypoint_name = dploy_config.get_entrypoint();

    run_entrypoint(&loc_str, &entrypoint_name, envs_map)?;
    
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
    let tag = match &git_ref {
        Some(git_ref) => git_ref.clone(),
        None => "latest".to_owned(),
    };

    if tag == "latest" && !latest {
        println!("Tag is latest, setting latest to true!");
        latest = true;
    }

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
        make_archive(&use_dir, env_str, &name, env_str, &tag)?;
    }

    extract(&name, env_str, &tag)?;

    let loc_str = location(&name, env_str, &tag);

    let config_path_dir = loc_str.clone();
    let mut dploy_config = load_dploy_config(&config_path_dir)?;

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
        make_archive(&use_dir, env_str, &name, env_str, &tag)?;
        // Reload config
        extract(&name, env_str, &tag)?;
        dploy_config = load_dploy_config(&config_path_dir)?;
    }

    println!("Running deploy.");

    let maybe_password = match get_password_env(env, &name, Stage::Deploy) {
        Err(AuthError::NoPassword) => {
            println!("No password found for stage {} and repo URL {}. Set password using `tidploy auth`!", "deploy", &repo_url);
            return Ok(());
        }
        other => other,
    }?;

    let mut envs_map = HashMap::<String, String>::new();

    let deploy_name = format!("{}-{}", env_str, &tag).replace('.', "_");

    let recreate_value = if recreate { "yes" } else { "no" };

    // TODO this is too specific logic
    let deploy_tag_suffix = if tag == "latest" {
        "".to_owned()
    } else {
        format!("-{}", &tag)
    };

    let entrypoint_name = dploy_config.get_entrypoint();

    println!("Running entrypoint with deploy name {}...", &deploy_name);
    envs_map.insert("RECREATE".to_owned(), recreate_value.to_owned());
    envs_map.insert("DEPLOY_NAME".to_owned(), deploy_name);
    envs_map.insert("DEPLOY_TAG_SUFFIX".to_owned(), deploy_tag_suffix);

    if dploy_config.uses_dployer() {
        let dployer_env = dploy_config.get_env_var().ok_or(ConfigError::NoEnvVar)?;
        if let Some(password) = maybe_password.clone() {
            envs_map.insert(dployer_env, password);
        }

        run_entrypoint(&loc_str, &entrypoint_name, envs_map)?;

        return Ok(());
    }

    let mut sp = Spinner::new(spinners::Line, "Loading secrets...", None);

    for id in dploy_config.get_secrets() {
        let mut run_secrets = Cmd::new("bws");
        let run_secrets =
            add_password_maybe(&mut run_secrets, maybe_password.clone(), "BWS_ACCESS_TOKEN")
                .arg("secret")
                .arg("get")
                .arg(&id);
        let output = run_secrets.output().map_err(ProcessError::IO)?;

        output_check_success(&output)?;

        let secrets_output = String::from_utf8(output.stdout).map_err(ProcessError::Decode)?;

        let s_output: SecretOutput =
            serde_json::from_str(&secrets_output).map_err(DeployError::SecretsDecode)?;
        envs_map.insert(s_output.key, s_output.value);
    }
    sp.success("Secrets loaded into environment!");

    run_entrypoint(&loc_str, &entrypoint_name, envs_map)?;

    Ok(())
}

pub(crate) fn run_cli() -> Result<(), Error> {
    let args = Args::parse();

    match args.command {
        Commands::Auth { stage, repo } => Ok(auth_command(stage, repo).map_err(ErrorRepr::from)?),
        Commands::Download { env, git_ref, repo } => {
            Ok(download_command(env, git_ref, repo).map_err(ErrorRepr::from)?)
        },
        Commands::Run { program, env, git_ref, repo, env_var } => {
            Ok(run_command(program, env, git_ref, repo, env_var).map_err(ErrorRepr::from)?)
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
