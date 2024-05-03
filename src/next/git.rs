use super::{
    errors::{GitError, GitProcessError},
    process::process_complete_output,
};
use core::fmt::Debug;
use std::{ffi::OsStr, path::Path};

fn run_git<S: AsRef<OsStr> + Debug>(
    working_dir: &Path,
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

pub(crate) fn git_root_dir(path: &Path) -> Result<String, GitError> {
    let args = vec!["rev-parse", "--show-toplevel"];

    run_git(path, args, "get git root dir")
}

#[derive(Debug, PartialEq)]
pub(crate) struct Repo {
    pub(crate) name: String,
    pub(crate) encoded_url: String,
    pub(crate) url: String,
}

impl Repo {
    pub(crate) fn dir_name(&self) -> String {
        format!("{}_{}", self.name, self.encoded_url)
    }
}

