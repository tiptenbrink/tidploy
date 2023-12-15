use crate::errors::GitError;
use crate::process::process_out;

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64USNP;
use base64::Engine;

use std::process::Command as Cmd;
use thiserror::Error as ThisError;

pub(crate) fn git_root_origin_url() -> Result<String, GitError> {
    let git_origin_output = Cmd::new("git")
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

    Ok(String::from_utf8(git_origin_output.stdout)
        .map_err(|e| GitError::from_dec(e, "Failed to decode Git origin output!".to_owned()))?
        .trim_end()
        .to_owned())
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

pub(crate) fn rev_parse_tag(tag: &str, use_origin: bool) -> Result<String, GitError> {
    let _prefixed_tag = if use_origin {
        if tag.starts_with("origin/") {
            tag.to_owned() // If it already contains origin/ we will just leave it as is
        } else {
            format!("origin/{}", tag)
        }
    } else {
        tag.to_owned()
    };

    let parsed_tag_output = Cmd::new("git")
        .arg("rev-parse")
        .arg(tag)
        .output()
        .map_err(|e| GitError::from_io(e, "IO failure for parsing Git tag!".to_owned()))?;

    if !parsed_tag_output.status.success() {
        let err_out = process_out(
            parsed_tag_output.stderr,
            "Git parse tag failed! Could not decode output!".to_owned(),
        )?;
        let _msg = format!("Git parse tag failed! err: {}", err_out);
        return Err(GitError::from_f(
            parsed_tag_output.status,
            "Git parse tag failed!".to_owned(),
        ));
    }

    Ok(String::from_utf8(parsed_tag_output.stdout)
        .map_err(|e| {
            GitError::from_dec(e, "Failed to decode Git relative to root path!".to_owned())
        })?
        .trim_end()
        .to_owned())
}
