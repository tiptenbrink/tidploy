use directories::ProjectDirs;
use std::{env, io::Error as StdIOError, path::PathBuf, sync::OnceLock};
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

pub(crate) struct Dirs {
    pub(crate) cache: PathBuf,
    pub(crate) tmp: PathBuf,
}

pub(crate) fn get_dirs() -> &'static Dirs {
    static DIRS: OnceLock<Dirs> = OnceLock::new();
    DIRS.get_or_init(|| {
        let project_dirs = ProjectDirs::from("", "", "tidploy").unwrap();

        let cache = project_dirs.cache_dir().to_owned();
        let tmp = env::temp_dir();

        Dirs { cache, tmp }
    })
}
