use clap::{Parser, ValueEnum, Subcommand};
use rpassword::prompt_password;
use keyring::{Entry, Result as KeyringResult, Error as KeyringError, Error::NoEntry};
use serde::Deserialize;
use std::{process::{Command as Cmd, Stdio}, io::BufReader, io::BufRead, fs, collections::HashMap, path::Path};
use spinoff::{Spinner, spinners};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Environment {
    /// Local development environment
    Localdev,
    /// Staging environment
    Staging,
    /// Production environment
    Production
}

impl Environment {
    fn to_string(self) -> &'static str {
        match self {
            Self::Localdev => "localdev",
            Self::Staging => "staging",
            Self::Production => "production"
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
            Self::Deploy => "deploy"
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
        #[arg(id = "version_tag", default_value = "latest")]
        tag: String,
    },

    /// Deploy tag or version with specific env
    Deploy {
        /// Environment
        #[arg(value_enum)]
        env: Environment,

        /// Version or tag to deploy
        #[arg(id = "version_tag", default_value = "latest")]
        tag: String,
    },
    
    /// Save authentication details for specific stage until reboot
    Auth {
        #[arg(value_enum)]
        stage: Stage
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
        .output().unwrap();
    let archive_name = format!("{}.tar.gz", env_tag_name(env, tag));

    let archive_loc = format!("{}/{}", &archives_dir, &archive_name);

    let _remove_existing = Cmd::new("rm")
        .arg(&archive_loc)
        .output().unwrap();

    let mut output_archive_prog = Cmd::new("tar");
    let output_archive = output_archive_prog
        .current_dir(source_dir_parent)
        .arg("-czf")
        .arg(archive_loc)
        .arg(source_dir);

    output_archive.output().unwrap();

    println!("Saved deploy archive in tmp.");
}

fn download_tag(repo_url: &str, env: &str, tag: &str) {
    let loc_str = location(env, tag);
    let repo_loc = format!("{}_repo", loc_str);
    
    let _mk_tmp_dir = Cmd::new("mkdir")
        .arg("-p")
        .arg(TMP_DIR)
        .output().unwrap();

    let _remove_existing = Cmd::new("rm")
        .arg("-rf")
        .arg(&repo_loc)
        .output().unwrap();

    let mut sp = Spinner::new(spinners::Line, "Cloning repository...", None);
    
    let _repo_clone_stdout = Cmd::new("git")
        .arg("clone")
        .arg("--filter=tree:0")
        .arg(repo_url)
        .arg(&repo_loc)
        .stdout(Stdio::piped())
        .output().unwrap();

    sp.success("Repository cloned!");

    let _checkout = Cmd::new("git")
        .current_dir(&repo_loc)
        .arg("checkout")
        .arg(tag)
        .output().unwrap();

    println!("Checked out ref {}.", tag);

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
    let _mk_tmp_dir = Cmd::new("mkdir")
        .arg("-p")
        .arg(TMP_DIR)
        .output().unwrap();
}


fn extract(env: &str, tag: &str) {
    let archives_dir = format!("{}/archives", TMP_DIR);
    let env_tag = env_tag_name(env, tag);
    let archive_name = format!("{}.tar.gz", &env_tag);

    let archive_loc = format!("{}/{}", &archives_dir, &archive_name);
    let target_dir = format!("{}/{}", TMP_DIR, env_tag);

    let _remove_existing = Cmd::new("rm")
        .arg("-rf")
        .arg(&target_dir)
        .output().unwrap();

    let _mk_tmp_dir = Cmd::new("mkdir")
        .arg("-p")
        .arg(&target_dir)
        .output().unwrap();

    let mut tar_prog = Cmd::new("tar");
    //tar -xzf /tmp/ti_dploy/archives/staging_b648930.tar.gz -C ./staging_b648930 --strip-components 1
    // strip components might not work on every platform
    let tar_prog = tar_prog.arg("-xzf")
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
    KeyringError(KeyringError)
}

fn get_password_env(env: Environment, stage: Stage) -> Result<Option<String>, Error> {
    match env {
        Environment::Localdev => Ok(None),
        Environment::Staging | Environment::Production => match get_password(stage.to_string()) {
            Ok(None) => Err(Error::NoPassword),
            Ok(pw_some) => Ok(pw_some),
            Err(e) => Err(Error::KeyringError(e))
        }
    }
}

fn add_password_maybe<'a>(cmd: &'a mut Cmd, password_option: Option<String>, env_key: &str) -> &'a mut Cmd {
    match password_option {
        None => cmd,
        Some(password) => cmd.env(env_key, password)
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
}

#[derive(Deserialize)]
struct DploySecrets {
    ids: Vec<String>
}

fn load_dploy_config(file_path: &str) -> DployConfig {
    let toml_file = fs::read_to_string(file_path).unwrap();

    let dploy_config: DployConfig = toml::from_str(&toml_file).unwrap();

    dploy_config
}

fn main() {
    let args = Args::parse();

    let repo_url = "https://github.com/DSAV-Dodeka/dodeka.git";

    println!("{:?}", args);

    match args.command {
        Commands::Auth { stage } => {
            let password = prompt_password("Enter password:\n").unwrap();
            set_password(&password, stage.to_string()).unwrap();
            println!("Set password!");
        },
        Commands::Download { env, tag } => {
            let env_str = env.to_string();
            download_tag(repo_url, env_str, &tag);
            extract(env_str, &tag);
        },
        Commands::Deploy { env, tag } => {

            let env_str = env.to_string();

            let archives_dir = format!("{}/archives", TMP_DIR);
            let env_tag = env_tag_name(env_str, &tag);
            let archive_name = format!("{}.tar.gz", &env_tag);

            let archive_loc = format!("{}/{}", &archives_dir, &archive_name);
            
            let archive_path = Path::new(&archive_loc);

            if !archive_path.exists() {
                println!("Archive doesn't exist, downloading...");
                download_tag(repo_url, env_str, &tag);
            }

            //copy_to_archives(env_str, &tag);
            extract(env_str, &tag);

            println!("Running deploy.");

            let loc_str = location(env_str, &tag);

            let config_path = format!("{}/{}", &loc_str, "tidploy.toml");
            let dploy_config = load_dploy_config(&config_path);

            let maybe_password = match get_password_env(env, Stage::Deploy) {
                Err(Error::NoPassword) => {
                    println!("Set password using `tidploy auth`!");
                    return
                },
                other => other
            }.unwrap();

            let mut sp = Spinner::new(spinners::Line, "Loading secrets...", None);
            let mut secrets = HashMap::<String, String>::new();
            for id in dploy_config.secrets.ids {
                let mut run_secrets = Cmd::new("bws");
                let run_secrets = add_password_maybe(&mut run_secrets, maybe_password.clone(), "BWS_ACCESS_TOKEN")
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
                    return
                }

                let secrets_output = String::from_utf8(output.stdout).unwrap();


                let s_output: SecretOutput = serde_json::from_str(&secrets_output).unwrap();
                secrets.insert(s_output.key, s_output.value);
            }
            sp.success("Secrets loaded into environment!");

            let deploy_name = format!("{}-{}", env_str, &tag).replace('.', "dot");

            let mut entrypoint_output = Cmd::new(format!("{}/{}", &loc_str, "entrypoint.sh"))
                .current_dir(&loc_str)
                .envs(&secrets)
                .env("DEPLOY_NAME", deploy_name)
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
