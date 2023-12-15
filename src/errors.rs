use std::io::Error as StdIOError;
use std::process::ExitStatus;
use std::string::FromUtf8Error;
use relative_path::FromPathError;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
struct IOError {
    msg: String,
    source: StdIOError
}

#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
pub(crate) struct ProcessError {
    msg: String,
    source: ProcessErrorKind
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

#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
pub(crate) struct GitError {
    pub(crate) msg: &'static str,
    pub(crate) source: GitErrorKind
}

impl GitError {
    pub(crate) fn from_io(e: StdIOError, msg: &'static str) -> GitError {
        return GitError { msg, source: ProcessErrorKind::IO(e).into()}
    }

    pub(crate) fn from_f(f: ExitStatus, msg: &'static str) -> GitError {
        return GitError { msg, source: ProcessErrorKind::Failed(f).into()}
    }

    pub(crate) fn from_dec(e: FromUtf8Error, msg: &'static str) -> GitError {
        return GitError { msg, source: ProcessErrorKind::Decode(e).into()}
    }
}   


#[derive(Debug, ThisError)]
pub(crate) enum GitErrorKind {
    #[error("Failure for external Git process! {0}")]
    Process(#[from] ProcessErrorKind),
    #[error("Failure decoding Git output! {0}")]
    Decode(#[from] FromUtf8Error),
}

#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
pub(crate) struct RelPathError {
    pub(crate) msg: String,
    pub(crate) source: RelPathErrorKind
}

impl RelPathError {
    pub(crate) fn from_knd(e: impl Into<RelPathErrorKind>, msg: String) -> RelPathError {
        return RelPathError { msg, source: e.into() }
    }
}   

#[derive(Debug, ThisError)]
enum RelPathErrorKind {
    #[error(transparent)]
    FromPath(#[from] FromPathError)
}