use std::path::Path;

fn transcode_file(source: &Path, dest: &Path) -> std::io::Result<()> {
    println!("{:?} => {:?}", source, dest);
    std::fs::remove_file(dest).ok();
    let tmp = tempfile::NamedTempFile::new()?;

    let child = std::process::Command::new("opusenc")
        .arg("--quiet")
        .arg("--bitrate")
        .arg("128")
        .arg("--discard-pictures")
        .arg(source)
        .arg(tmp.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to execute child");
    child.wait_with_output().expect("failed to wait on child");
    std::fs::create_dir_all(dest.parent().unwrap()).expect("Error making dest dir");
    std::fs::rename(tmp.path(), dest).expect("Error moving file");
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
    for entry in globwalk::glob(pattern)
        .expect("Glob error.")
        .filter_map(|e| e.ok())
        .take(20)
    {
        let relative = entry
            .path()
            .strip_prefix(source_path)
            .expect("Not a prefix");
        let target = dest_path.join(relative).with_extension("opus");
        if !target.exists() {
            if dry_run {
                println!("{}", target.to_string_lossy());
            } else {
                let handle = std::thread::spawn(move || {
                    transcode_file(entry.path(), &target).expect("Error transcoding");
                });
                handle.join().ok();
            }
        }
    }
}
