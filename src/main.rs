#![allow(dead_code, unused_variables)]

use clap::{Parser, ValueEnum, Subcommand};
use rpassword::prompt_password;
use keyring::{Entry, Result as KeyringResult, Error as KeyringError, Error::NoEntry};
use std::process::{Command as Cmd, Output};

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
    fn to_string(self: Self) -> &'static str {
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
    fn to_string(self: Self) -> &'static str {
        match self {
            Self::Download => "download",
            Self::Deploy => "deploy"
        }
    }
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, args_conflicts_with_subcommands(true))]
struct Args {
    /// Environment
    #[arg(value_enum, required = true)]
    env: Option<Environment>,

    /// Version or tag to deploy
    #[arg(id = "version_tag", default_value = "latest")]
    tag: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
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

    /// Download tag or version with specific env
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

fn get_password() -> KeyringResult<Option<String>> {
    let entry = Entry::new("ti_dploy", "default")?;
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

fn download_release(repo: &str, env: &str, tag: &str) -> () {
    let output = Cmd::new("gh")
        .arg("release")
        .arg("download")
        .arg(tag)
        .arg("-R")
        .arg(repo)
        .arg("-p")
        .arg(env)
        .output().unwrap();
}

fn make_archive(source_dir_parent: &str, source_dir: &str, env: &str, tag: &str) -> () {
    mk_tmp_dir();

    let archives_dir = format!("{}/archives", TMP_DIR);
    let mk_tmp_dir = Cmd::new("mkdir")
        .arg("-p")
        .arg(&archives_dir)
        .output().unwrap();
    let archive_name = format!("{}.tar.gz", env_tag_name(env, tag));

    let archive_loc = format!("{}/{}", &archives_dir, &archive_name);

    let remove_existing = Cmd::new("rm")
        .arg(&archive_loc)
        .output().unwrap();

    let mut output_archive_prog = Cmd::new("tar");
    let output_archive = output_archive_prog
        .current_dir(source_dir_parent)
        .arg("-czf")
        .arg(archive_loc)
        .arg(source_dir);
    println!("{:?}", output_archive);
    output_archive.output().unwrap();
}

fn download_tag(repo_url: &str, env: &str, tag: &str) -> () {
    let loc_str = location(env, tag);
    let repo_loc = format!("{}_repo", loc_str);
    
    let mk_tmp_dir = Cmd::new("mkdir")
        .arg("-p")
        .arg(TMP_DIR)
        .output().unwrap();

    let remove_existing = Cmd::new("rm")
        .arg("-rf")
        .arg(&repo_loc)
        .output().unwrap();
    
    let repo_clone = Cmd::new("git")
        .arg("clone")
        .arg("--filter=tree:0")
        .arg(repo_url)
        .arg(&repo_loc)
        .output().unwrap();

    let checkout = Cmd::new("git")
        .current_dir(&repo_loc)
        .arg("checkout")
        .arg(tag)
        .output().unwrap();

    let use_dir = format!("{}/use", repo_loc);

    make_archive(&use_dir, env, env, tag);
}


fn move_to_archives(env: &str, tag: &str) -> () {
    mk_tmp_dir();

    let archives_dir = format!("{}/archives", TMP_DIR);
    let mk_tmp_dir = Cmd::new("mkdir")
        .arg("-p")
        .arg(&archives_dir)
        .output().unwrap();

    let env_tag = env_tag_name(env, tag);
    let archive_name = format!("{}.tar.gz", &env_tag);

    let archive_loc = format!("{}/{}", &archives_dir, &archive_name);

    let remove_existing = Cmd::new("rm")
        .arg(&archive_loc)
        .output().unwrap();
    

    let move_archive = Cmd::new("mv")
        .arg(format!("./{}", &archive_name))
        .arg(&archive_loc)
        .output().unwrap();
}

fn mk_tmp_dir() -> () {
    let mk_tmp_dir = Cmd::new("mkdir")
        .arg("-p")
        .arg(TMP_DIR)
        .output().unwrap();
}

fn extract(env: &str, tag: &str) -> () {
    let archives_dir = format!("{}/archives", TMP_DIR);
    let env_tag = env_tag_name(env, tag);
    let archive_name = format!("{}.tar.gz", &env_tag);

    let archive_loc = format!("{}/{}", &archives_dir, &archive_name);
    let target_dir = format!("{}/{}", TMP_DIR, env_tag);

    let remove_existing = Cmd::new("rm")
        .arg("-rf")
        .arg(&target_dir)
        .output().unwrap();

    let mk_tmp_dir = Cmd::new("mkdir")
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
    
    println!("{:?}", tar_prog);

    tar_prog.output().unwrap();
    ()
}

#[derive(Debug)]
enum Error {
    NoPassword,
    KeyringError(KeyringError)
}

fn add_password_maybe(env: Environment, cmd: &mut Cmd) -> Result<&mut Cmd, Error> {
    match env {
        Environment::Localdev => Ok(cmd),
        Environment::Staging | Environment::Production => {
            match get_password() {
                Ok(Some(password)) => Ok(cmd.env("TI_DPLOY_SECRET", password)),
                Ok(None) => Err(Error::NoPassword),
                Err(e) => Err(Error::KeyringError(e))
            }
        }
    }
}

fn show_cmd_result(output: &Output) {
    println!("{}", output.status);
    println!("{}", String::from_utf8(output.stderr.clone()).unwrap());
    println!("{}", String::from_utf8(output.stdout.clone()).unwrap());
}

fn main() {
    let args = Args::parse();

    let repo = "DSAV-Dodeka/dodeka";
    let repo_url = "https://github.com/DSAV-Dodeka/dodeka.git";

    println!("{:?}", args);

    match args.command {
        Some(Commands::Auth { stage }) => {
            let password = prompt_password("Enter password:\n").unwrap();
            set_password(&password, stage.to_string()).unwrap();
            println!("Set password!");
            return ()
        },
        Some(Commands::Download { env, tag }) => {
            let env_str = env.to_string();
            let tag = "b648930";
            download_tag(repo_url, env_str, tag);
            //TODO download step
            extract(env_str, &tag);

            return ()
        },
        Some(Commands::Deploy { env, tag }) => {

            let env_str = env.to_string();

            extract(env_str, &tag);

            let loc_str = location(env_str, &tag);

            let mut run_secrets = Cmd::new(format!("{}/{}", loc_str, "secrets.sh"));
            let pw_cmd = add_password_maybe(env, &mut run_secrets);
            if pw_cmd.as_ref().is_err_and(|e| matches!(e, Error::NoPassword)) {
                println!("Set password using `dploy auth`!");
                return ()
            }
            let run_secrets = pw_cmd.unwrap();

            let secrets_output = run_secrets.output().unwrap();
            show_cmd_result(&secrets_output);

            // let entrypoint_output = Cmd::new(format!("{}/{}", location, "entrypoint.sh"))
            // .output().unwrap();

            // show_cmd_result(&entrypoint_output);
        }
        None => {}
    }

    
}
