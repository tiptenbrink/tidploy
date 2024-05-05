use camino::{Utf8Path, Utf8PathBuf};
use directories::ProjectDirs;
use std::{env, fs::create_dir, sync::OnceLock};


pub(crate) struct Dirs {
    pub(crate) cache: Utf8PathBuf,
    pub(crate) tmp: Utf8PathBuf,
}

/// You cannot assume these directories actually exist.
pub(crate) fn get_dirs() -> &'static Dirs {
    static DIRS: OnceLock<Dirs> = OnceLock::new();
    DIRS.get_or_init(|| {
        let project_dirs = ProjectDirs::from("", "", "tidploy").unwrap();

        let cache = project_dirs.cache_dir().to_owned();
        let tmp = env::temp_dir();
        let cache = Utf8PathBuf::from_path_buf(cache).unwrap();
        let tmp = Utf8PathBuf::from_path_buf(tmp).unwrap().join("tidploy");

        Dirs { cache, tmp }
    })
}