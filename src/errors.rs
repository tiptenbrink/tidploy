use thiserror::Error as ThisError;
use std::io::Error as IOError;
use std::string::FromUtf8Error;



#[derive(Debug, ThisError)]
pub(crate) enum ProcessError {
    #[error("IO failure for external process!")]
    IO(#[from] IOError),
    #[error("Failure decoding process output!")]
    Decode(#[from] FromUtf8Error),
    #[error("Process had no output!")]
    NoOutput,
    #[error("Process failed!")]
    Failed(std::process::ExitStatus)
}

#[derive(Debug, ThisError)]
pub(crate) enum FileError {
    #[error("IO failure for file!")]
    IO(#[from] IOError),
    #[error("Failed working with file in external process!")]
    Process(#[from] ProcessError)
}

#[derive(Debug, ThisError)]
pub(crate) enum GitError {
    #[error("IO failure for external process!")]
    IO(#[from] IOError),
    #[error("Failure decoding Git output!")]
    Decode(#[from] FromUtf8Error),
}