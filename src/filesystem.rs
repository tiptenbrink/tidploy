use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use directories::ProjectDirs;
use once_cell::sync::OnceCell;
use relative_path::{RelativePath, RelativePathBuf};
use std::{env, io::Error as StdIOError};
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

// #[derive(Debug, ThisError)]
// pub(crate) enum RelativePathError {
//     #[error("The full path {0} is not a child of the root (did you use too many ..?")]
//     Child(String),
//     #[error(
//         "An error occurred when canonicalizing the full path. Does it exist and is it UTF-8? {0}"
//     )]
//     Canonicalize(#[from] std::io::Error),
// }

pub trait WrapToPath {
    fn to_utf8_path<P: AsRef<Utf8Path>>(&self, path: P) -> Utf8PathBuf;

    // fn to_path_canon_checked(&self, root: &Utf8Path) -> Result<Utf8PathBuf, RelativePathError>;
}

impl WrapToPath for RelativePath {
    fn to_utf8_path<P: AsRef<Utf8Path>>(&self, path: P) -> Utf8PathBuf {
        let path = path.as_ref().as_std_path();
        let std_path = self.to_path(path);
        // Since we started with Utf8Path, we know this will work
        Utf8PathBuf::from_path_buf(std_path).unwrap()
    }

    // fn to_path_canon_checked(&self, root: &Utf8Path) -> Result<Utf8PathBuf, RelativePathError> {
    //     let full = self.to_utf8_path(root);

    //     if !full_canon.starts_with(root) {
    //         Err(RelativePathError::Child(full_canon.to_string()))
    //     } else {
    //         Ok(full_canon)
    //     }
    // }
}

impl WrapToPath for RelativePathBuf {
    fn to_utf8_path<P: AsRef<Utf8Path>>(&self, path: P) -> Utf8PathBuf {
        let path = path.as_ref().as_std_path();
        let std_path = self.to_path(path);
        // Since we started with Utf8Path, we know this will work
        Utf8PathBuf::from_path_buf(std_path).unwrap()
    }

    // fn to_path_canon_checked(&self, root: &Utf8Path) -> Result<Utf8PathBuf, RelativePathError> {
    //     let full = self.to_utf8_path(root);
    //     let full_canon = full.canonicalize_utf8()?;

    //     if !full_canon.starts_with(root) {
    //         Err(RelativePathError::Child(full_canon.to_string()))
    //     } else {
    //         Ok(full_canon)
    //     }
    // }
}

pub trait PathClean {
    fn clean(&self) -> Utf8PathBuf;
}

impl PathClean for Utf8Path {
    fn clean(&self) -> Utf8PathBuf {
        clean(self)
    }
}

impl PathClean for Utf8PathBuf {
    fn clean(&self) -> Utf8PathBuf {
        clean(self)
    }
}

/// From https://github.com/danreeves/path-clean/blob/3876d7cb5367997bcda17ce165bf69c4f434cb93/src/lib.rs
/// By Dan Reeves, used under the Apache License 2.0
/// Changed to work for Utf8Path
pub fn clean<P>(path: P) -> Utf8PathBuf
where
    P: AsRef<Utf8Path>,
{
    let mut out = Vec::new();

    for comp in path.as_ref().components() {
        match comp {
            Utf8Component::CurDir => (),
            Utf8Component::ParentDir => match out.last() {
                Some(Utf8Component::RootDir) => (),
                Some(Utf8Component::Normal(_)) => {
                    out.pop();
                }
                None
                | Some(Utf8Component::CurDir)
                | Some(Utf8Component::ParentDir)
                | Some(Utf8Component::Prefix(_)) => out.push(comp),
            },
            comp => out.push(comp),
        }
    }

    if !out.is_empty() {
        out.iter().collect()
    } else {
        Utf8PathBuf::from(".")
    }
}
