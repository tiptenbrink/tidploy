
use crate::config::{load_dploy_config, ConfigError};
use crate::errors::{GitError, GitErrorKind, ProcessErrorKind};
use crate::secret_store::{get_password, set_password};
use crate::secrets::SecretOutput;
use base64::Engine;
use clap::{Parser, Subcommand, ValueEnum};
use keyring::Error as KeyringError;
use rpassword::prompt_password;
use spinoff::{spinners, Spinner};
use std::ffi::OsString;
use std::fs::{self};
use std::process::Output;
use std::string::FromUtf8Error;
use std::{
    io::Error as IOError,
    process::{Command as Cmd, Stdio},
};
use thiserror::Error as ThisError;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64USNP;

pub(crate) fn git_root_origin_url() -> Result<String, GitError> {
    let git_origin_output = Cmd::new("git")
        .arg("config")
        .arg("--get")
        .arg("remote.origin.url")
        .output()
        .map_err(|e| GitError::from_io(e, "IO failure for git config get remote.origin.url!"))?;

    if !git_origin_output.status.success() {
        return Err(GitError::from_f(git_origin_output.status, "Git get remote origin failed!"))
    }

    Ok(String::from_utf8(git_origin_output.stdout).map_err(|e| GitError::from_dec(e, "Failed to decode Git origin output!"))?
        .trim_end()
        .to_owned())
}

pub(crate) fn relative_to_git_root() -> Result<String, GitError> {
    
    let git_root_relative_output = Cmd::new("git")
        .arg("rev-parse")
        .arg("--show-prefix")
        .output()
        .map_err(|e| GitError::from_io(e, "IO failure for get relative to git root!"))?;

    if !git_root_relative_output.status.success() {
        return Err(GitError::from_f(git_root_relative_output.status, "Git get relative to root failed!"))
    }

    Ok(String::from_utf8(git_root_relative_output.stdout).map_err(|e| GitError::from_dec(e, "Failed to decode Git relative to root path!"))?
        .trim_end()
        .to_owned())
}

#[derive(Debug, ThisError)]
pub(crate) enum RepoParseError {
    #[error("Repo URL {0} doesn't end with /<name>.git and cannot be parsed!")]
    InvalidURL(String),
}

pub(crate) struct Repo {
    pub(crate) name: String,
    pub(crate) encoded_url: String,
    pub(crate) url: String
}

pub(crate) fn parse_repo_url(url: String) -> Result<Repo, RepoParseError> {
    let mut split_parts: Vec<&str> = url.split('/').collect();
    let last_part = *split_parts
        .last()
        .ok_or(RepoParseError::InvalidURL(url.clone()))?;
    let first_parts = split_parts.get(0..split_parts.len()-1).map(|a| a.to_vec().join("/"));
    let encoded_url = if let Some(pre_part) = first_parts {
        B64USNP.encode(pre_part)
    } else {
        return Err(RepoParseError::InvalidURL(url))
    };

    let split_parts_dot: Vec<&str> = last_part.split('.').collect();
    let name = (*split_parts_dot
        .first()
        .ok_or(RepoParseError::InvalidURL(url.clone()))?)
    .to_owned();

    Ok(Repo {
        name,
        encoded_url,
        url
    })
}