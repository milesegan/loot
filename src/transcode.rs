use rayon::prelude::*;
use std::path::Path;

fn transcode_file(source: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::remove_file(dest).ok();
    let tmp = tempfile::NamedTempFile::new()?;

    let child = std::process::Command::new("opusenc")
        .arg("--quiet")
        .arg("--bitrate")
        .arg("128")
        .arg("--discard-pictures")
        .arg(source)
        .arg(tmp.path())
        .spawn()
        .expect("failed to execute child");
    child.wait_with_output().expect("failed to wait on child");
    std::fs::create_dir_all(dest.parent().unwrap()).expect("Error making dest dir");
    std::fs::rename(tmp.path(), dest).expect("Error moving file");
    return Ok(());
}

fn extract_cover(source: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest.parent().unwrap()).expect("Error making dest dir");
    let child = std::process::Command::new("metaflac")
        .arg("--export-picture-to")
        .arg(dest)
        .arg(source)
        .spawn()
        .expect("Failed to execute");
    child.wait_with_output().expect("Failed to wait");
    return Ok(());
}

pub fn transcode(source_path: &str, dest_dir: &str, dry_run: bool) {
    let canonical = Path::new(source_path)
        .canonicalize()
        .expect("Invalid path.");
    let canonical_string = canonical.to_str().expect("Invalid path.");
    println!("processing {}", canonical_string);

    let dest_path = Path::new(dest_dir);
    let pattern = format!("{}/**/*.{{flac,opus}}", canonical_string);
    globwalk::glob(&pattern)
        .expect("glob error")
        .filter_map(Result::ok)
        .into_iter()
        .par_bridge()
        .for_each(|entry| {
            let relative = entry
                .path()
                .strip_prefix(source_path)
                .expect("Not a prefix");
            let cover = dest_path
                .join(relative)
                .with_file_name("cover")
                .with_extension("jpg");
            if !cover.exists() {
                extract_cover(entry.path(), &cover).ok();
            }
            let target = dest_path.join(relative).with_extension("opus");
            if !target.exists() {
                if dry_run {
                    println!("{}", target.to_string_lossy());
                } else {
                    println!("{}", relative.to_string_lossy());
                    transcode_file(entry.path(), &target).expect("Error transcoding");
                }
            }
        });
}