use crate::errors::{GitError, RepoError, RepoParseError};

use crate::process::process_out;

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64USNP;
use base64::Engine;
use relative_path::RelativePath;
use spinoff::{spinners, Spinner};

use std::fs;
use std::path::Path;
use std::process::{Command as Cmd, Stdio};
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

#[derive(Debug, PartialEq)]
pub(crate) struct Repo {
    pub(crate) name: String,
    pub(crate) encoded_url: String,
    pub(crate) url: String,
}

impl Repo {
    pub(crate) fn dir_name(&self) -> String {
        return format!("{}_{}", self.name, self.encoded_url)
    }
}

/// Parse a repo URL to extract a "name" from it, as well as encode the part before the name to still uniquely 
/// identify it. Only supports forward slashes as path seperator.
pub(crate) fn parse_repo_url(url: String) -> Result<Repo, RepoParseError> {
    let url = url.strip_suffix("/").unwrap_or(&url).to_owned();
    // We want the final part, after the slash, as the "file name"
    let split_parts: Vec<&str> = url.split('/').collect();
    
    // If last does not exist then the string is empty so invalid
    let last_part = *split_parts
        .last()
        .ok_or(RepoParseError::InvalidURL(url.to_owned()))?;

    // The first part will contain slashes and potentially other characters we don't want in a file name, so we
    // encode it
    let encoded_url = if split_parts.len() <= 1 {
        // In this case the part before the slash is empty so no encoding necessary
        "".to_owned()
    } else {
        // We get everything except the last part and then rejoin them using the slash we originally split them with
        let pre_part = split_parts
        .get(0..split_parts.len() - 1).unwrap()
        .join("/");
        debug!("Encoding parsed url pre_part: {}", pre_part);
        // base64urlsafe-encode
        B64USNP.encode(pre_part)
    };

    // In case there is a file extension (such as `.git`), we don't want that part of the name
    let split_parts_dot: Vec<&str> = last_part.split('.').collect();
    let name = if split_parts_dot.len() <= 1 {
        // In this case no "." exists and we return just the entire "file name"
        last_part.to_owned()
    } else {
        // We get only the part that comes before the first .
        (*split_parts_dot
            .first()
            .ok_or(RepoParseError::InvalidURL(url.clone()))?)
        .to_owned()
    };

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

    debug!(
        "Cloning repository {} directory at {:?}",
        repo_url, repo_path
    );
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
    
    let success_msg = format!("Checked out {}!", commit_sha);
    sp.success(&success_msg);

    Ok(())
}

pub(crate) fn checkout_path(repo_path: &Path, deploy_path: &RelativePath) -> Result<(), RepoError> {
    if !repo_path.exists() {
        return Err(RepoError::NotCreated);
    }

    let mut sp = Spinner::new(
        spinners::Line,
        format!(
            "Sparse-checkout repository to deploy path {}...",
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

    let success_msg = format!("Sparse checked out repository to deploy path {}!", deploy_path);
    sp.success(&success_msg);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_repo_url;

    #[test]
    fn parse_test_git() {
        let git_url = "https://github.com/tiptenbrink/tidploy.git".to_owned();
        let encoded_url = "aHR0cHM6Ly9naXRodWIuY29tL3RpcHRlbmJyaW5r".to_owned();
        let name = "tidploy".to_owned();
        assert_eq!(parse_repo_url(git_url.clone()).unwrap().encoded_url, encoded_url);
        assert_eq!(parse_repo_url(git_url.clone()).unwrap().name, name);
        assert_eq!(parse_repo_url(git_url.clone()).unwrap().url, git_url);
    }

    #[test]
    fn parse_test_local() {
        let path = "/home/tiptenbrink/tidploy/".to_owned();
        let path_no_slash = "/home/tiptenbrink/tidploy".to_owned();
        let encoded_url = "L2hvbWUvdGlwdGVuYnJpbms".to_owned();
        let name = "tidploy".to_owned();
        assert_eq!(parse_repo_url(path.clone()).unwrap().encoded_url, encoded_url);
        assert_eq!(parse_repo_url(path.clone()).unwrap().name, name);
        assert_eq!(parse_repo_url(path).unwrap().url, path_no_slash);
        assert_eq!(parse_repo_url(path_no_slash.clone()).unwrap().url, path_no_slash);
    }
}