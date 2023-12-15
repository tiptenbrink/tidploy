use std::{env, io::Error as StdIOError, path::PathBuf};
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
#[error("{msg} {source}")]
pub(crate) struct FileError {
    pub(crate) msg: String,
    pub(crate) source: FileErrorKind,
}

#[derive(Debug, ThisError)]
pub(crate) enum FileErrorKind {
    #[error("IO error dealing with filesystem! {0}")]
    IO(#[from] StdIOError),
    #[error("Path cannot be converted to a string!")]
    InvalidPath,
}

pub(crate) fn get_current_dir() -> Result<PathBuf, FileErrorKind> {
    env::current_dir().map_err(FileErrorKind::IO)
}
