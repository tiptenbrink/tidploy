use crate::errors::{GitError, RepoError};

use crate::process::process_out;

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64USNP;
use base64::Engine;
use relative_path::RelativePath;
use spinoff::{spinners, Spinner};

use std::fs;
use std::path::Path;
use std::process::{Command as Cmd, Stdio};
use thiserror::Error as ThisError;
use tracing::debug;

pub(crate) fn git_root_origin_url(path: &Path) -> Result<String, GitError> {
    let git_origin_output = Cmd::new("git")
        .current_dir(path)
        .arg("config")
        .arg("--get")
        .arg("remote.origin.url")
        .output()
        .map_err(|e| {
            GitError::from_io(
                e,
                "IO failure for git config get remote.origin.url!".to_owned(),
            )
        })?;

    if !git_origin_output.status.success() {
        return Err(GitError::from_f(
            git_origin_output.status,
            "Git get remote origin failed!".to_owned(),
        ));
    }

    let url = String::from_utf8(git_origin_output.stdout)
        .map_err(|e| GitError::from_dec(e, "Failed to decode Git origin output!".to_owned()))?
        .trim_end()
        .to_owned();

    debug!("Read remote url from git root origin: {}", url);

    Ok(url)
}

pub(crate) fn relative_to_git_root() -> Result<String, GitError> {
    let git_root_relative_output = Cmd::new("git")
        .arg("rev-parse")
        .arg("--show-prefix")
        .output()
        .map_err(|e| GitError::from_io(e, "IO failure for get relative to git root!".to_owned()))?;

    if !git_root_relative_output.status.success() {
        return Err(GitError::from_f(
            git_root_relative_output.status,
            "Git get relative to root failed!".to_owned(),
        ));
    }

    Ok(String::from_utf8(git_root_relative_output.stdout)
        .map_err(|e| {
            GitError::from_dec(e, "Failed to decode Git relative to root path!".to_owned())
        })?
        .trim_end()
        .to_owned())
}

#[derive(Debug, ThisError)]
pub(crate) enum RepoParseError {
    #[error("Repo URL '{0}' doesn't end with /<name>.git and cannot be parsed!")]
    InvalidURL(String),
}

#[derive(Debug)]
pub(crate) struct Repo {
    pub(crate) name: String,
    pub(crate) encoded_url: String,
    pub(crate) url: String,
}

pub(crate) fn parse_repo_url(url: String) -> Result<Repo, RepoParseError> {
    let split_parts: Vec<&str> = url.split('/').collect();

    if split_parts.len() <= 1 {
        return Err(RepoParseError::InvalidURL(url));
    }
    let last_part = *split_parts
        .last()
        .ok_or(RepoParseError::InvalidURL(url.clone()))?;

    let first_parts = split_parts
        .get(0..split_parts.len() - 1)
        .map(|a| a.to_vec().join("/"));

    let encoded_url = if let Some(pre_part) = first_parts {
        B64USNP.encode(pre_part)
    } else {
        return Err(RepoParseError::InvalidURL(url));
    };

    let split_parts_dot: Vec<&str> = last_part.split('.').collect();
    if split_parts_dot.len() <= 1 {
        return Err(RepoParseError::InvalidURL(url));
    }

    let name = (*split_parts_dot
        .first()
        .ok_or(RepoParseError::InvalidURL(url.clone()))?)
    .to_owned();

    Ok(Repo {
        name,
        encoded_url,
        url,
    })
}

pub(crate) fn rev_parse_tag(tag: &str, path: &Path) -> Result<String, GitError> {
    let parsed_tag_output = Cmd::new("git")
        .current_dir(path)
        .arg("rev-parse")
        .arg(tag)
        .output()
        .map_err(|e| GitError::from_io(e, "IO failure for parsing Git tag!".to_owned()))?;

    if !parsed_tag_output.status.success() {
        let err_out = process_out(
            parsed_tag_output.stderr,
            "Git parse tag failed! Could not decode output!".to_owned(),
        )?;
        let msg = format!("Git parse tag failed! err: {}", err_out);
        return Err(GitError::from_f(parsed_tag_output.status, msg));
    }

    Ok(String::from_utf8(parsed_tag_output.stdout)
        .map_err(|e| {
            GitError::from_dec(e, "Failed to decode Git relative to root path!".to_owned())
        })?
        .trim_end()
        .to_owned())
}

pub(crate) fn repo_clone(
    current_dir: &Path,
    target_name: &str,
    repo_url: &str,
) -> Result<(), RepoError> {
    let repo_path = current_dir.join(target_name);
    let exists = repo_path.exists();
    if !current_dir.exists() {
        fs::create_dir_all(current_dir).map_err(|e| {
            RepoError::from_io(
                e,
                format!("Couldn't create directory {:?} before clone", current_dir),
            )
        })?;
    }

    if exists {
        fs::remove_dir_all(&repo_path).map_err(|e| {
            RepoError::from_io(
                e,
                format!("Couldn't remove directory {:?} before clone", repo_path),
            )
        })?;
    }

    let mut sp = Spinner::new(spinners::Line, "Cloning repository...", None);

    let _repo_clone_stdout = Cmd::new("git")
        .current_dir(current_dir)
        .arg("clone")
        .arg("--filter=tree:0")
        .arg("--sparse")
        .arg(repo_url)
        .arg(target_name)
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| {
            GitError::from_io(
                e,
                format!("IO failure for clone Git repository {}!", target_name),
            )
        })?;

    let _init_sparse = Cmd::new("git")
        .current_dir(&repo_path)
        .arg("sparse-checkout")
        .arg("init")
        .arg("--cone")
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| {
            GitError::from_io(
                e,
                format!("IO failure for sparse-checkout init {:?}!", repo_path),
            )
        })?;

    sp.success("Repository cloned!");

    Ok(())
}

pub(crate) fn checkout(repo_path: &Path, commit_sha: &str) -> Result<(), RepoError> {
    if !repo_path.exists() {
        return Err(RepoError::NotCreated);
    }

    let mut sp = Spinner::new(
        spinners::Line,
        format!("Checking out commit {}...", commit_sha),
        None,
    );

    let _repo_clone_stdout = Cmd::new("git")
        .current_dir(repo_path)
        .arg("reset")
        .arg("--hard")
        .arg(commit_sha)
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| {
            GitError::from_io(
                e,
                format!("IO failure for reset hard Git repository {:?}!", repo_path),
            )
        })?;

    let _init_sparse = Cmd::new("git")
        .current_dir(repo_path)
        .arg("clean")
        .arg("-fxd")
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| GitError::from_io(e, format!("IO failure for git clean {:?}!", repo_path)))?;

    sp.success("Commit checked out!");

    Ok(())
}

pub(crate) fn checkout_path(repo_path: &Path, deploy_path: &RelativePath) -> Result<(), RepoError> {
    if !repo_path.exists() {
        return Err(RepoError::NotCreated);
    }

    let mut sp = Spinner::new(
        spinners::Line,
        format!(
            "Sparse-checkout repository to deploy path {:?}...",
            deploy_path
        ),
        None,
    );

    let _repo_clone_stdout = Cmd::new("git")
        .current_dir(repo_path)
        .arg("sparse-checkout")
        .arg("set")
        .arg(deploy_path.as_str())
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| {
            GitError::from_io(
                e,
                format!("IO failure for sparse-checkout repository {:?}!", repo_path),
            )
        })?;

    sp.success("Sparse checked out repository to deploy path!");

    Ok(())
}
