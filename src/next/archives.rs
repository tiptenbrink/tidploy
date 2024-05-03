use std::fs::File;
use flate2::Compression;
use flate2::write::GzEncoder;

fn archive() -> Result<(), ArchiveError> {
    let tar_gz = File::create("archive.tar.gz")?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.append_dir_all("backup/logs", "/var/log")?;
    let a = tar.into_inner();
    Ok(())
}