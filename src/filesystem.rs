use camino::{Utf8Path, Utf8PathBuf};
use directories::ProjectDirs;
use relative_path::{RelativePath, RelativePathBuf};
use std::{env, io::Error as StdIOError};
use thiserror::Error as ThisError;
use once_cell::sync::OnceCell;

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

pub(crate) fn get_current_dir() -> Result<Utf8PathBuf, FileErrorKind> {
    let current_dir = env::current_dir().map_err(FileErrorKind::IO)?;
    Utf8PathBuf::from_path_buf(current_dir).map_err(|_e| FileErrorKind::InvalidPath)
}

pub(crate) struct Dirs {
    pub(crate) cache: Utf8PathBuf,
    pub(crate) tmp: Utf8PathBuf,
}

pub(crate) fn get_dirs() -> Result<&'static Dirs, FileErrorKind> {
    static DIRS: OnceCell<Dirs> = OnceCell::new();
    DIRS.get_or_try_init(|| {
        let project_dirs = ProjectDirs::from("", "", "tidploy").unwrap();

        let cache = project_dirs.cache_dir().to_owned();
        let tmp = env::temp_dir();
        let cache = Utf8PathBuf::from_path_buf(cache).map_err(|_e| FileErrorKind::InvalidPath)?;
        let tmp = Utf8PathBuf::from_path_buf(tmp).map_err(|_e| FileErrorKind::InvalidPath)?;

        Ok(Dirs { cache, tmp })
    })
}

pub trait WrapToPath {
    fn to_utf8_path<P: AsRef<Utf8Path>>(&self, path: P) -> Utf8PathBuf;
}

impl WrapToPath for RelativePath
{
    fn to_utf8_path<P: AsRef<Utf8Path>>(&self, path: P) -> Utf8PathBuf {
        let path = path.as_ref().as_std_path();
        let std_path = self.to_path(path);
        // Since we started with Utf8Path, we know this will work
        Utf8PathBuf::from_path_buf(std_path).unwrap()
    }
}

impl WrapToPath for RelativePathBuf
{
    fn to_utf8_path<P: AsRef<Utf8Path>>(&self, path: P) -> Utf8PathBuf {
        let path = path.as_ref().as_std_path();
        let std_path = self.to_path(path);
        // Since we started with Utf8Path, we know this will work
        Utf8PathBuf::from_path_buf(std_path).unwrap()
    }
}