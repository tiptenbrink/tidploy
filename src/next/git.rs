use camino::Utf8Path;
use spinoff::{spinners, Spinner};
use tracing::debug;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64USNP, Engine};

use super::{
    errors::{GitError, GitProcessError}, fs::{get_dirs, Dirs}, process::process_complete_output, state::parse_url_repo_name
};
use core::fmt::Debug;
use std::ffi::OsStr;

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

    let clone_args = vec!["clone", "--filter=tree:0", "--sparse", repo_url, target_name];
    run_git(current_dir, clone_args, "partial clone sparse")?;
    let target_dir = current_dir.join(target_name);
    let checkout_args = vec!["sparse-checkout", "init", "--cone"];
    run_git(&target_dir, checkout_args, "partial clone sparse")?;

    sp.success("Repository cloned!");

    Ok(())
}

fn do_clone() {
    let dirs = get_dirs();
    let a = dirs.cache.as_path();
    let b = dirs.tmp.as_path();

    let url = "https://github.com/tiptenbrink/tidploy.git";
    let encoded_url = B64USNP.encode(url);
    let name = parse_url_repo_name(&url).unwrap();
    let dir_name = format!("{}_{}", name, encoded_url);

    let t = b;
    repo_clone(t, &dir_name, url).unwrap();
}


// mod tests {

//     use super::do_clone;

//     #[test]
//     fn test_do_clone() {
//         do_clone();
//     }
// }