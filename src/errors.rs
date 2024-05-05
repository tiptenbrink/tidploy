use relative_path::FromPathError;
use std::io::Error as StdIOError;
use std::process::ExitStatus;
use std::string::FromUtf8Error;
use thiserror::Error as ThisError;

use crate::filesystem::FileError;

#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
pub(crate) struct ProcessError {
    pub(crate) msg: String,
    pub(crate) source: ProcessErrorKind,
}

#[derive(Debug, ThisError)]
pub(crate) enum ProcessErrorKind {
    #[error("IO failure for external process! {0}")]
    IO(#[from] StdIOError),
    #[error("Failure decoding process output! {0}")]
    Decode(#[from] FromUtf8Error),
    #[error("Process had no output!")]
    NoOutput,
    #[error("Process failed! {0}")]
    Failed(std::process::ExitStatus),
}

#[derive(Debug, ThisError)]
pub(crate) enum GitError {
    #[error("External Git process failed: {0}")]
    Process(#[from] ProcessError),
}

#[derive(Debug, ThisError)]
pub(crate) enum TarError {
    #[error("External tar process failed: {0}")]
    Process(#[from] ProcessError),
}

impl TarError {
    pub(crate) fn from_io(e: StdIOError, msg: String) -> TarError {
        ProcessError {
            msg,
            source: ProcessErrorKind::IO(e),
        }
        .into()
    }

    pub(crate) fn from_f(f: ExitStatus, msg: String) -> TarError {
        ProcessError {
            msg,
            source: ProcessErrorKind::Failed(f),
        }
        .into()
    }
}

impl GitError {
    pub(crate) fn from_io(e: StdIOError, msg: String) -> GitError {
        ProcessError {
            msg,
            source: ProcessErrorKind::IO(e),
        }
        .into()
    }

    pub(crate) fn from_f(f: ExitStatus, msg: String) -> GitError {
        ProcessError {
            msg,
            source: ProcessErrorKind::Failed(f),
        }
        .into()
    }

    pub(crate) fn from_dec(e: FromUtf8Error, msg: String) -> GitError {
        ProcessError {
            msg,
            source: ProcessErrorKind::Decode(e),
        }
        .into()
    }
}

#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
pub(crate) struct RelPathError {
    pub(crate) msg: String,
    pub(crate) source: RelPathErrorKind,
}

impl RelPathError {
    pub(crate) fn from_knd(e: impl Into<RelPathErrorKind>, msg: String) -> RelPathError {
        RelPathError {
            msg,
            source: e.into(),
        }
    }
}

#[derive(Debug, ThisError)]
pub(crate) enum RelPathErrorKind {
    #[error(transparent)]
    FromPath(#[from] FromPathError),
}

#[derive(Debug, ThisError)]
pub(crate) enum RepoParseError {
    #[error("Repo URL '{0}' doesn't end with /<name>.git and cannot be parsed!")]
    InvalidURL(String),
}

#[derive(Debug, ThisError)]
pub(crate) enum RepoError {
    #[error("Failure during repo process dealing with files! {0}")]
    File(#[from] FileError),
    #[error("Failure during repo process dealing with tar! {0}")]
    Tar(#[from] TarError),
    #[error("Failure during repo process dealing with Git! {0}")]
    Git(#[from] GitError),
    #[error("Failure parsing repo/repo url! {0}")]
    RepoParse(#[from] RepoParseError),
    #[error("Cannot checkout if repository has not been created!")]
    NotCreated,
}

impl RepoError {
    pub(crate) fn from_io(e: std::io::Error, msg: String) -> RepoError {
        FileError {
            source: e.into(),
            msg,
        }
        .into()
    }
}
