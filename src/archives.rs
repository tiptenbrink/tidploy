use crate::errors::{RepoError, TarError};
use crate::filesystem::{FileError, FileErrorKind};
use crate::process::process_out;
use std::fs;

use std::path::{Path, PathBuf};
use std::process::Command as Cmd;

pub(crate) fn make_archive(
    archives_path: &Path,
    current_dir: &Path,
    source_name: &str,
    target_name: &str,
) -> Result<PathBuf, RepoError> {
    if !archives_path.exists() {
        fs::create_dir_all(archives_path).map_err(|e| {
            RepoError::from_io(
                e,
                format!("Couldn't create archives directory {:?}!", archives_path),
            )
        })?;
    }

    let archive_name = format!("{}.tar.gz", target_name);

    let archive_path = archives_path.join(archive_name);
    let archive_path_name = archive_path.to_str().ok_or(FileError {
        msg: format!("Cannot represent path {:?} as string!", archive_path),
        source: FileErrorKind::InvalidPath,
    })?;

    if archive_path.exists() {
        return Ok(archive_path);
    }

    let mut output_archive_prog = Cmd::new("tar");
    let output_archive = output_archive_prog
        .current_dir(current_dir)
        .arg("-czf")
        .arg(archive_path_name)
        .arg(source_name);

    let archive_output = output_archive
        .output()
        .map_err(|e| TarError::from_io(e, "IO failure for tar archive!".to_owned()))?;

    if !archive_output.status.success() {
        return Err(
            TarError::from_f(archive_output.status, "Tar archive failed!".to_owned()).into(),
        );
    }

    println!("Saved deploy archive in tmp.");

    Ok(archive_path)
}

pub(crate) fn extract_archive(
    archive_path: &Path,
    current_dir: &Path, // TMP DIR
    target_name: &str,
) -> Result<(), RepoError> {
    let archive_path_name = archive_path.to_str().ok_or(FileError {
        msg: format!("Cannot represent path {:?} as string!", archive_path),
        source: FileErrorKind::InvalidPath,
    })?;

    let target_path = current_dir.join(target_name);
    if target_path.exists() {
        fs::remove_dir_all(&target_path).map_err(|e| {
            RepoError::from_io(
                e,
                format!(
                    "Couldn't remove target directory before recreation {:?}!",
                    target_name
                ),
            )
        })?;
    }
    fs::create_dir_all(&target_path).map_err(|e| {
        RepoError::from_io(
            e,
            format!("Couldn't create target directory {:?}!", target_name),
        )
    })?;

    let mut output_archive_prog = Cmd::new("tar");
    let output_archive = output_archive_prog
        .current_dir(current_dir)
        .arg("-xzf")
        .arg(archive_path_name)
        .arg("-C")
        .arg(target_name)
        .arg("--strip-components")
        .arg("1");

    let archive_output = output_archive
        .output()
        .map_err(|e| TarError::from_io(e, "IO failure for extract archive!".to_owned()))?;

    if !archive_output.status.success() {
        let err_out = process_out(
            archive_output.stderr,
            "Tar extract failed! Could not decode output!".to_owned(),
        )
        .map_err(TarError::Process)?;
        let msg = format!("Tar exctract failed! err: {}", err_out);
        return Err(TarError::from_f(archive_output.status, msg).into());
    }

    println!("Extracted archive.");

    Ok(())
}
