use camino::Utf8Path;
use color_eyre::eyre::Report;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io;
use tar::Archive;

fn archive(target_path: &Utf8Path, source_path: &Utf8Path) -> Result<(), io::Error> {
    let tar_gz = File::create(target_path)?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.append_dir_all("", source_path)?;
    tar.into_inner()?;
    Ok(())
}

fn unarchive(target_path: &Utf8Path, archive_path: &Utf8Path) -> Result<(), std::io::Error> {
    let tar_gz = File::open(archive_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(target_path)?;

    Ok(())
}

pub(crate) fn archive_command() -> Result<(), Report> {
    let source = Utf8Path::new("/tmp/tidploy/a.tar.gz");
    let target = Utf8Path::new("/tmp/tidploy/something2");

    unarchive(target, source)?;

    Ok(())
}
