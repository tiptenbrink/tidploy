use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64USNP, Engine};
use camino::Utf8Path;
use relative_path::RelativePath;
use spinoff::{spinners, Spinner};
use tracing::debug;

use crate::{
    filesystem::WrapToPath,
    next::errors::{ContextIOError, WrapStateErr},
};

use super::{
    errors::{GitError, GitProcessError, StateError},
    process::process_complete_output,
    state::{parse_url_repo_name, GitAddress, State},
};
use core::fmt::Debug;
use std::{
    ffi::OsStr,
    fs::{self, create_dir_all, remove_dir_all},
    io,
    path::Path,
};

fn run_git<S: AsRef<OsStr> + Debug>(
    working_dir: &Utf8Path,
    args: Vec<S>,
    op_name: &'static str,
) -> Result<String, GitError> {
    let git_out = process_complete_output(working_dir, "git", args);

    match git_out {
        Ok(out) => {
            if out.exit.success() {
                Ok(out.out)
            } else {
                Err(GitError::Failed(out.out))
            }
        }
        Err(err) => Err(GitProcessError {
            msg: format!("Git operation {} failed.", op_name),
            source: err,
        }
        .into()),
    }
}

pub(crate) fn git_root_dir(path: &Utf8Path) -> Result<String, GitError> {
    let args = vec!["rev-parse", "--show-toplevel"];

    run_git(path, args, "get git root dir")
}

pub(crate) fn repo_clone(
    current_dir: &Utf8Path,
    target_name: &str,
    repo_url: &str,
) -> Result<(), GitError> {
    debug!(
        "Cloning repository {} directory at target {}",
        repo_url, target_name
    );
    create_dir_all(current_dir).map_err(|e| {
        GitError::IO(ContextIOError {
            msg: format!(
                "Failed to create directory {} to Git clone to!",
                current_dir
            ),
            source: e,
        })
    })?;
    let mut sp = Spinner::new(spinners::Line, "Cloning repository...", None);

    let clone_args = vec![
        "clone",
        "--filter=tree:0",
        "--sparse",
        "--no-checkout",
        repo_url,
        target_name,
    ];
    run_git(current_dir, clone_args, "partial clone sparse")?;
    let target_dir = current_dir.join(target_name);
    let checkout_args = vec!["sparse-checkout", "init", "--cone"];
    run_git(&target_dir, checkout_args, "partial clone sparse")?;

    sp.success("Repository cloned!");

    Ok(())
}

pub(crate) fn git_fetch(repo_dir: &Utf8Path) -> Result<(), GitError> {
    let mut sp = Spinner::new(spinners::Line, "Running git fetch...", None);

    let clone_args = vec!["fetch"];
    run_git(repo_dir, clone_args, "fetch")?;

    sp.success("Fetched!");

    Ok(())
}

pub(crate) fn checkout(repo_dir: &Utf8Path, checkout_sha: &str) -> Result<(), GitError> {
    let mut sp = Spinner::new(spinners::Line, "Checking out...", None);

    let clone_args = vec!["checkout", checkout_sha];
    run_git(repo_dir, clone_args, "checkout")?;

    sp.success("Checked out!");

    Ok(())
}

pub(crate) fn sparse_checkout(repo_dir: &Utf8Path, mut paths: Vec<&str>) -> Result<(), GitError> {
    let mut sp = Spinner::new(spinners::Line, "Performing sparse checkout...", None);

    let mut args = vec!["sparse-checkout", "set"];
    args.append(&mut paths);
    run_git(repo_dir, args, "checkout")?;

    sp.success("Sparse checkout done!");

    Ok(())
}

#[derive(Debug)]
struct ShaRef {
    sha: String,
    tag: String,
}

pub(crate) fn ls_remote(repo_dir: &Utf8Path, pattern: &str) -> Result<String, GitError> {
    let mut sp = Spinner::new(spinners::Line, "Getting commit hash from remote...", None);

    let args = vec!["ls-remote", "origin", pattern];
    let out = run_git(repo_dir, args, "ls-remote origin")?;

    let split = out.trim().split('\n');
    let lines: Vec<&str> = split.take(3).collect();
    if lines.len() > 2 {
        return Err(GitError::Failed(format!(
            "Pattern is not specific enough, cannot determine commit for {}",
            pattern
        )));
    }
    let sha_refs = lines
        .into_iter()
        .take(2)
        .map(|s| {
            let spl: Vec<&str> = s.split_whitespace().collect();
            if spl.len() != 2 {
                return Err(GitError::Failed(format!(
                    "ls-remote returned invalid result: {}",
                    &out
                )));
            }

            let sha = spl[0].to_owned();
            let tag = spl[1].to_owned();

            Ok(ShaRef { sha, tag })
        })
        .collect::<Result<Vec<ShaRef>, GitError>>()?;

    let commit = if sha_refs.len() == 2 {
        // We want the one with ^{}
        if sha_refs[0].tag.ends_with("^{}") {
            &sha_refs[0].sha
        } else if sha_refs[1].tag.ends_with("^{}") {
            &sha_refs[1].sha
        } else {
            return Err(GitError::Failed(format!(
                "Could not choose tag from two options for ls-remote: {:?}",
                &sha_refs
            )));
        }
    } else if sha_refs.len() == 1 {
        &sha_refs[0].sha
    // Assume that the pattern given is a commit itself
    } else {
        pattern
    };
    //let rev_parse_arg = format!("{}^{{}}", rev_parse_arg);
    //println!("rev_parse_arg {}", &rev_parse_arg);
    //let args = vec!["rev-parse", &rev_parse_arg];
    //let commit = run_git(repo_dir, args, "rev-parse commit/tag")?;

    sp.success("Got commit hash from remote!");

    Ok(commit.to_owned())
}

/// https://stackoverflow.com/a/65192210
fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub(crate) fn get_dir_from_git(
    address: GitAddress,
    state_path: &RelativePath,
    state_root: &RelativePath,
    store_dir: &Utf8Path,
) -> Result<State, StateError> {
    let encoded_url = B64USNP.encode(&address.url);
    let name = parse_url_repo_name(&address.url)
        .to_state_err("Error passing Git url for determining name.".to_owned())?;
    let dir_name = format!("{}_{}", name, encoded_url);

    let target_dir = store_dir.join(&dir_name);
    if !target_dir.exists() {
        repo_clone(store_dir, &dir_name, &address.url)
            .to_state_err("Error cloning repository in address.".to_owned())?;
    }

    let commit = ls_remote(&target_dir, &address.git_ref)
        .to_state_err("Error getting provided tag.".to_owned())?;
    let commit_dir = store_dir.join("commits");
    let commit_path = commit_dir.join(&dir_name).join(&commit);

    let state_root_git = address.path.join(state_root);
    let state_path_git = state_root.join(state_path);

    if !commit_path.exists() {
        git_fetch(&target_dir)
            .to_state_err("Error updating repository to ensure commit exists.".to_owned())?;
        copy_dir_all(&target_dir, &commit_path)
            .to_state_err("Error copying main repository before checkout.".to_owned())?;
        checkout(&commit_path, &commit)
            .to_state_err("Error checking out new commit.".to_owned())?;
        let paths = vec![state_root_git.as_str(), state_path_git.as_str()];
        sparse_checkout(&commit_path, paths)
            .to_state_err("Error setting new paths for sparse checkout.".to_owned())?;
        remove_dir_all(commit_path.join(".git"))
            .to_state_err("Error removing .git directory.".to_owned())?;
    }

    Ok(State {
        resolve_root: address.path.to_utf8_path(&commit_path),
        state_root: state_root.to_owned(),
        state_path: state_path.to_owned(),
        address: None,
    })
}
