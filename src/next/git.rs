use camino::Utf8Path;
use spinoff::{spinners, Spinner};
use tracing::debug;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64USNP, Engine};

use super::{
    errors::{GitError, GitProcessError}, fs::{get_dirs, Dirs}, process::process_complete_output, state::parse_url_repo_name
};
use core::fmt::Debug;
use std::{ffi::OsStr, fs, io, path::{Path, PathBuf}};

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
    let mut sp = Spinner::new(spinners::Line, "Cloning repository...", None);

    let clone_args = vec!["clone", "--filter=tree:0", "--sparse", "--no-checkout", repo_url, target_name];
    run_git(current_dir, clone_args, "partial clone sparse")?;
    // let target_dir = current_dir.join(target_name);
    // let checkout_args = vec!["sparse-checkout", "init", "--cone"];
    // run_git(&target_dir, checkout_args, "partial clone sparse")?;

    sp.success("Repository cloned!");

    Ok(())
}

pub(crate) fn git_fetch(
    repo_dir: &Utf8Path,
) -> Result<(), GitError> {
    let mut sp = Spinner::new(spinners::Line, "Running git fetch...", None);

    let clone_args = vec!["fetch"];
    run_git(repo_dir, clone_args, "fetch")?;

    sp.success("Fetched!");

    Ok(())
}

pub(crate) fn checkout(
    repo_dir: &Utf8Path,
    checkout_sha: &str
) -> Result<(), GitError> {
    let mut sp = Spinner::new(spinners::Line, "Checking out...", None);

    let clone_args = vec!["checkout", checkout_sha];
    run_git(repo_dir, clone_args, "checkout")?;

    sp.success("Checked out!");

    Ok(())
}

pub(crate) fn sparse_checkout(
    repo_dir: &Utf8Path,
    mut paths: Vec<&str>
) -> Result<(), GitError> {
    let mut sp = Spinner::new(spinners::Line, "Performing sparse checkout...", None);

    let mut args = vec!["sparse-checkout", "set"];
    args.append(&mut paths);
    run_git(repo_dir, args, "checkout")?;

    sp.success("Sparse checkout done!");

    Ok(())
}

struct ShaRef {
    sha: String,
    tag: String
}

pub(crate) fn ls_remote(
    repo_dir: &Utf8Path,
    pattern: &str,
) -> Result<String, GitError> {
    let mut sp = Spinner::new(spinners::Line, "Getting commit hash from remote...", None);

    let args = vec!["ls-remote", "origin", pattern];
    let out = run_git(repo_dir, args, "partial clone sparse")?;

    let split = out.trim().split("\n");
    let lines: Vec<&str> = split.take(3).collect();
    if lines.len() > 2 {
        return Err(GitError::Failed(format!("Pattern is not specific enough, cannot determine commit for {}", pattern)))
    }
    let sha_refs = lines.into_iter().take(2).map(|s| {
        let spl: Vec<&str> = s.split_whitespace().collect();
        if spl.len() != 2 {
            return Err(GitError::Failed(format!("ls-remote returned invalid result: {}", &out)))
        }
        
        let sha = spl[0].to_owned();
        let tag = spl[1].to_owned();
        
        Ok(ShaRef {
            sha,
            tag
        })
    }).collect::<Result<Vec<ShaRef>,GitError>>()?;

    let rev_parse_arg = if sha_refs.len() == 2 {
        // We want the one without ^{} so we can add it ourselves
        if sha_refs[0].tag.ends_with("^{}") {
            &sha_refs[1].tag
        } else {
            &sha_refs[0].tag
        }
    } else if sha_refs.len() == 1 {
        &sha_refs[0].tag
    } else {
        pattern
    };
    let rev_parse_arg = format!("{}^{{}}", rev_parse_arg);
    let args = vec!["rev-parse", &rev_parse_arg];
    let commit = run_git(repo_dir, args, "rev-parse commit/tag")?;

    sp.success("Got commit hash from remote!");

    Ok(commit)
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

fn do_clone() {
    // Have a single sparse, no-checkout repo for each URL
    // do ls-remote to find the tag
    // Check if you already have that tag in the store
    // If not, do git fetch and copy the no-checkout sparse repo
    // In the new repo
    // git checkout <commit>
    // Do the sparse-checkout

    let dirs = get_dirs();
    let cache = dirs.cache.as_path();
    let tmp = dirs.tmp.as_path();

    let url = "https://github.com/tiptenbrink/tidploy.git";
    let encoded_url = B64USNP.encode(url);
    let name = parse_url_repo_name(&url).unwrap();
    let dir_name = format!("{}_{}", name, encoded_url);
    
    let target_dir = tmp.join(&dir_name);
    repo_clone(tmp, &dir_name, url).unwrap();
    let commit = ls_remote(&target_dir, "HEAD").unwrap();
    let tmp_commits = tmp.join("commits");
    println!("{}", commit);
    let commit_path_name = format!("{}_{}", dir_name, commit);
    let commit_path = tmp_commits.join(commit_path_name);

    if !commit_path.exists() {
        git_fetch(&target_dir).unwrap();
        copy_dir_all(&target_dir, &commit_path).unwrap();
        checkout(&commit_path, &commit).unwrap();
        let paths = vec!["examples", "src"];
        sparse_checkout(&commit_path, paths).unwrap();
    }
    
}


// mod tests {

//     use super::do_clone;

//     #[test]
//     fn test_do_clone() {
//         do_clone();
//     }
// }