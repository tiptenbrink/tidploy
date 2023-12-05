use clap::{Parser, Subcommand, ValueEnum};
use keyring::{Entry, Error as KeyringError, Error::NoEntry, Result as KeyringResult};
use rpassword::prompt_password;
use serde::Deserialize;
use spinoff::{spinners, Spinner};
use std::{
    collections::HashMap,
    fs,
    io::BufRead,
    io::BufReader,
    path::Path,
    process::{Command as Cmd, Stdio},
};

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
    },

    /// Deploy tag or version with specific env
    Deploy {
        /// Environment
        #[arg(value_enum)]
        env: Environment,

        /// Version or tag to deploy. Omit to deploy latest for env
        git_ref: Option<String>,

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
    },
}

static TMP_DIR: &str = "/tmp/ti_dploy";

fn get_password(stage: &str) -> KeyringResult<Option<String>> {
    let entry = Entry::new("ti_dploy", stage)?;
    match entry.get_password() {
        Ok(pw) => Ok(Some(pw)),
        Err(NoEntry) => Ok(None),
        Err(err) => Err(err),
    }
}

fn set_password(password: &str, stage: &str) -> KeyringResult<()> {
    let entry = Entry::new("ti_dploy", stage)?;
    entry.set_password(password)?;
    Ok(())
}

fn env_tag_name(env: &str, tag: &str) -> String {
    format!("{}_{}", env, tag)
}

fn location(env: &str, tag: &str) -> String {
    let env_tag_name = env_tag_name(env, tag);

    format!("{}/{}", TMP_DIR, env_tag_name)
}

// fn download_release(repo: &str, env: &str, tag: &str) -> () {
//     let output = Cmd::new("gh")
//         .arg("release")
//         .arg("download")
//         .arg(tag)
//         .arg("-R")
//         .arg(repo)
//         .arg("-p")
//         .arg(env)
//         .output().unwrap();
// }

fn make_archive(source_dir_parent: &str, source_dir: &str, env: &str, tag: &str) {
    mk_tmp_dir();

    let archives_dir = format!("{}/archives", TMP_DIR);
    let _mk_tmp_dir = Cmd::new("mkdir")
        .arg("-p")
        .arg(&archives_dir)
        .output()
        .unwrap();
    let archive_name = format!("{}.tar.gz", env_tag_name(env, tag));

    let archive_loc = format!("{}/{}", &archives_dir, &archive_name);

    let _remove_existing = Cmd::new("rm").arg(&archive_loc).output().unwrap();

    let mut output_archive_prog = Cmd::new("tar");
    let output_archive = output_archive_prog
        .current_dir(source_dir_parent)
        .arg("-czf")
        .arg(archive_loc)
        .arg(source_dir);

    output_archive.output().unwrap();

    println!("Saved deploy archive in tmp.");
}

fn checkout_tag(repo_loc: &str, git_ref: &str) {
    let _checkout = Cmd::new("git")
        .current_dir(repo_loc)
        .arg("checkout")
        .arg("-f")
        .arg(git_ref)
        .output()
        .unwrap();
}

fn create_archive(repo_url: &str, env: &str, tag: &str, git_ref_opt: Option<String>, latest: bool) {
    let loc_str = location(env, tag);
    let repo_loc = format!("{}_repo", loc_str);

    mk_tmp_dir();

    let exists = Path::new(&repo_loc).exists();

    if !exists || git_ref_opt.is_none() {
        let _remove_existing = Cmd::new("rm").arg("-rf").arg(&repo_loc).output().unwrap();

        let mut sp = Spinner::new(spinners::Line, "Cloning repository...", None);

        let _repo_clone_stdout = Cmd::new("git")
            .arg("clone")
            .arg("--filter=tree:0")
            .arg(repo_url)
            .arg(&repo_loc)
            .stdout(Stdio::piped())
            .output()
            .unwrap();

        sp.success("Repository cloned!");

        if let Some(git_ref) = git_ref_opt {
            let mut sp = Spinner::new(spinners::Line, "Checking out ref...", None);

            checkout_tag(&repo_loc, &git_ref);

            sp.success("Checked out ref!");
        }
    } else if let Some(git_ref) = git_ref_opt {
        if exists && latest {
            let mut sp = Spinner::new(spinners::Line, "Checking out ref and updating...", None);

            checkout_tag(&repo_loc, &git_ref);

            // In case we were on a branch we now update to latest
            let _pull = Cmd::new("git")
                .current_dir(&repo_loc)
                .arg("pull")
                .output()
                .unwrap();

            checkout_tag(&repo_loc, &git_ref);

            sp.success("Checked out ref!");
        } else if exists {
            let mut sp = Spinner::new(spinners::Line, "Checking out ref...", None);

            checkout_tag(&repo_loc, &git_ref);

            sp.success("Checked out ref!");
        }
    }

    let use_dir = format!("{}/use", repo_loc);

    make_archive(&use_dir, env, env, tag);
}

// fn copy_to_archives(env: &str, tag: &str) {
//     mk_tmp_dir();

//     let archives_dir = format!("{}/archives", TMP_DIR);
//     let _mk_tmp_dir = Cmd::new("mkdir")
//         .arg("-p")
//         .arg(&archives_dir)
//         .output().unwrap();

//     let env_tag = env_tag_name(env, tag);
//     let archive_name = format!("{}.tar.gz", &env_tag);

//     let archive_loc = format!("{}/{}", &archives_dir, &archive_name);

//     let _remove_existing = Cmd::new("rm")
//         .arg(&archive_loc)
//         .output().unwrap();

//     let _copy_archive = Cmd::new("cp")
//         .arg(format!("./{}", &archive_name))
//         .arg(&archive_loc)
//         .output().unwrap();

//     println!("Copied archive {} to tmp archives.", archive_name);
// }

fn mk_tmp_dir() {
    let _mk_tmp_dir = Cmd::new("mkdir").arg("-p").arg(TMP_DIR).output().unwrap();
}

fn extract(env: &str, tag: &str) {
    let archives_dir = format!("{}/archives", TMP_DIR);
    let env_tag = env_tag_name(env, tag);
    let archive_name = format!("{}.tar.gz", &env_tag);

    let archive_loc = format!("{}/{}", &archives_dir, &archive_name);
    let target_dir = format!("{}/{}", TMP_DIR, env_tag);

    let _remove_existing = Cmd::new("rm").arg("-rf").arg(&target_dir).output().unwrap();

    let _mk_tmp_dir = Cmd::new("mkdir")
        .arg("-p")
        .arg(&target_dir)
        .output()
        .unwrap();

    let mut tar_prog = Cmd::new("tar");
    //tar -xzf /tmp/ti_dploy/archives/staging_b648930.tar.gz -C ./staging_b648930 --strip-components 1
    // strip components might not work on every platform
    let tar_prog = tar_prog
        .arg("-xzf")
        .arg(archive_loc)
        .current_dir(TMP_DIR)
        .arg("-C")
        .arg(env_tag)
        .arg("--strip-components")
        .arg("1");

    tar_prog.output().unwrap();

    println!("Extracted archive {}.", archive_name);
}

#[derive(Debug)]
enum Error {
    NoPassword,
    KeyringError(KeyringError),
}

fn get_password_env(env: Environment, stage: Stage) -> Result<Option<String>, Error> {
    match env {
        Environment::Localdev => Ok(None),
        Environment::Staging | Environment::Production => match get_password(stage.to_string()) {
            Ok(None) => Err(Error::NoPassword),
            Ok(pw_some) => Ok(pw_some),
            Err(e) => Err(Error::KeyringError(e)),
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

#[derive(Deserialize)]
struct SecretOutput {
    key: String,
    value: String,
}

#[derive(Deserialize)]
struct DployConfig {
    secrets: DploySecrets,
    info: DployInfo,
}

#[derive(Deserialize)]
struct DployInfo {
    latest: String,
}

#[derive(Deserialize)]
struct DploySecrets {
    ids: Vec<String>,
}

fn load_dploy_config(file_path: &str) -> DployConfig {
    let toml_file = fs::read_to_string(file_path).unwrap();

    let dploy_config: DployConfig = toml::from_str(&toml_file).unwrap();

    dploy_config
}

fn main() {
    let args = Args::parse();

    let repo_url = "https://github.com/DSAV-Dodeka/dodeka.git";

    //println!("{:?}", args);

    match args.command {
        Commands::Auth { stage } => {
            let password = prompt_password("Enter password:\n").unwrap();
            set_password(&password, stage.to_string()).unwrap();
            println!("Set password!");
        }
        Commands::Download { env, git_ref } => {
            let env_str = env.to_string();
            let tag = match &git_ref {
                Some(git_ref) => git_ref.clone(),
                None => "latest".to_owned(),
            };
            create_archive(repo_url, env_str, &tag, git_ref, true);
            extract(env_str, &tag);
        }
        Commands::Deploy {
            env,
            git_ref,
            latest_opt,
            recreate,
        } => {
            let mut latest = latest_opt;
            if git_ref.is_none() && !latest {
                println!("No git ref is specified, setting latest to true!");
                latest = false;
            }
            let tag = match &git_ref {
                Some(git_ref) => git_ref.clone(),
                None => "latest".to_owned(),
            };

            let env_str = env.to_string();

            let archives_dir = format!("{}/archives", TMP_DIR);
            let env_tag = env_tag_name(env_str, &tag);
            let archive_name = format!("{}.tar.gz", &env_tag);

            let archive_loc = format!("{}/{}", &archives_dir, &archive_name);

            let archive_path = Path::new(&archive_loc);

            // Always download if tag is latest
            let new_archive = !archive_path.exists() || latest;

            if new_archive {
                println!("Creating new archive...");
                create_archive(repo_url, env_str, &tag, git_ref, latest);
            }

            extract(env_str, &tag);

            let loc_str = location(env_str, &tag);

            let config_path = format!("{}/{}", &loc_str, "tidploy.toml");
            let mut dploy_config = load_dploy_config(&config_path);

            // in this case we are on the latest commit, but we need to go back to the correct commit of the latest release
            if latest && new_archive {
                // Redownload with correct tag
                create_archive(
                    repo_url,
                    env_str,
                    &tag,
                    Some(dploy_config.info.latest.clone()),
                    true,
                );
                // Reload config
                extract(env_str, &tag);
                dploy_config = load_dploy_config(&config_path);
            }

            println!("Running deploy.");

            let maybe_password = match get_password_env(env, Stage::Deploy) {
                Err(Error::NoPassword) => {
                    println!("Set password using `tidploy auth`!");
                    return;
                }
                other => other,
            }
            .unwrap();

            // if true {
            //     return;
            // }

            let mut sp = Spinner::new(spinners::Line, "Loading secrets...", None);
            let mut secrets = HashMap::<String, String>::new();
            for id in dploy_config.secrets.ids {
                let mut run_secrets = Cmd::new("bws");
                let run_secrets = add_password_maybe(
                    &mut run_secrets,
                    maybe_password.clone(),
                    "BWS_ACCESS_TOKEN",
                )
                .arg("secret")
                .arg("get")
                .arg(&id);
                let output = run_secrets.output().unwrap();

                if !output.status.success() {
                    if !output.stderr.is_empty() {
                        println!("{}", String::from_utf8(output.stderr).unwrap());
                    } else {
                        println!("Error loading secrets: {:?}!", output.status)
                    }
                    return;
                }

                let secrets_output = String::from_utf8(output.stdout).unwrap();

                let s_output: SecretOutput = serde_json::from_str(&secrets_output).unwrap();
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
                .unwrap();

            let entrypoint_stdout = entrypoint_output.stdout.take().unwrap();

            let reader = BufReader::new(entrypoint_stdout);

            reader
                .lines()
                .map_while(Result::ok)
                .for_each(|line| println!("{}", line));

            let output_stderr = entrypoint_output.wait_with_output().unwrap().stderr;
            if !output_stderr.is_empty() {
                println!("{}", String::from_utf8(output_stderr).unwrap());
            }
        }
    }
}
