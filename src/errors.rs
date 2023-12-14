use std::io::Error as IOError;
use std::string::FromUtf8Error;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub(crate) enum ProcessError {
    #[error("IO failure for external process! {0}")]
    IO(#[from] IOError),
    #[error("Failure decoding process output! {0}")]
    Decode(#[from] FromUtf8Error),
    #[error("Process had no output!")]
    NoOutput,
    #[error("Process failed! {0}")]
    Failed(std::process::ExitStatus),
}

#[derive(Debug, ThisError)]
pub(crate) enum FileError {
    #[error("IO failure for file! {0}")]
    IO(#[from] IOError),
    #[error("Failed working with file in external process! {0}")]
    Process(#[from] ProcessError),
}

#[derive(Debug, ThisError)]
pub(crate) enum GitError {
    #[error("IO failure for external process! {0}")]
    IO(#[from] IOError),
    #[error("Failure decoding Git output! {0}")]
    Decode(#[from] FromUtf8Error),
}
