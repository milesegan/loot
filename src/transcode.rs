use globwalk::DirEntry;
use rayon::prelude::*;
use std::path::Path;

#[derive(Copy, Clone)]
pub enum TranscodeFormat {
    Aac,
    Opus,
    Mp3,
}

fn transcode_file(source: &Path, dest: &Path, format: TranscodeFormat) -> std::io::Result<()> {
    std::fs::remove_file(dest).ok();
    let tmp = tempfile::NamedTempFile::new()?;

    let child = match format {
        TranscodeFormat::Opus => std::process::Command::new("ffmpeg")
            .arg("-y")
            .arg("-loglevel")
            .arg("quiet")
            .arg("-i")
            .arg(source)
            .arg("-c:a")
            .arg("libopus")
            .arg("-map")
            .arg("a:0")
            .arg("-b:a")
            .arg("128k")
            .arg("-f")
            .arg("opus")
            .arg(tmp.path())
            .spawn()
            .expect("failed to execute child"),
        TranscodeFormat::Aac => std::process::Command::new("ffmpeg")
            .arg("-y")
            .arg("-loglevel")
            .arg("quiet")
            .arg("-i")
            .arg(source)
            .arg("-c:a")
            .arg("aac_at")
            .arg("-ar")
            .arg("44100")
            .arg("-map")
            .arg("a:0")
            .arg("-q:a")
            .arg("9")
            .arg("-f")
            .arg("mp4")
            .arg(tmp.path())
            .spawn()
            .expect("failed to execute child"),
        TranscodeFormat::Mp3 => std::process::Command::new("ffmpeg")
            .arg("-y")
            .arg("-loglevel")
            .arg("quiet")
            .arg("-i")
            .arg(source)
            .arg("-map_metadata")
            .arg("0")
            .arg("-id3v2_version")
            .arg("3")
            .arg("-map")
            .arg("0")
            .arg("-map")
            .arg("-0:1")
            .arg("-q:a")
            .arg("5")
            .arg("-f")
            .arg("mp3")
            .arg(tmp.path())
            .spawn()
            .expect("failed to execute child"),
    };
    child.wait_with_output().expect("failed to wait on child");
    std::fs::create_dir_all(dest.parent().unwrap()).expect("Error making dest dir");
    std::fs::rename(tmp.path(), dest).expect("Error moving file");
    return Ok(());
}

fn extract_cover(source: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest.parent().unwrap()).expect("Error making dest dir");
    let child = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-loglevel")
        .arg("quiet")
        .arg("-i")
        .arg(source)
        .arg("-an")
        .arg(dest)
        .spawn()
        .expect("Failed to execute");
    child.wait_with_output().expect("Failed to wait");
    return Ok(());
}

pub fn transcode(source_path: &str, dest_dir: &str, dry_run: bool, format: TranscodeFormat) {
    let canonical = Path::new(source_path)
        .canonicalize()
        .expect("Invalid path.");
    let canonical_string = canonical.to_str().expect("Invalid path.");
    println!("processing {}", canonical_string);

    let dest_path = Path::new(dest_dir);
    let pattern = format!("{}/**/*.{{flac,opus}}", canonical_string);
    let mut matches = globwalk::glob(&pattern)
        .expect("glob error")
        .filter_map(Result::ok)
        .into_iter()
        .collect::<Vec<DirEntry>>();
    matches.sort_by(|a, b| a.path().cmp(b.path()));
    matches.into_iter().par_bridge().for_each(|entry| {
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
        let target = match format {
            TranscodeFormat::Aac => dest_path.join(relative).with_extension("m4a"),
            TranscodeFormat::Opus => dest_path.join(relative).with_extension("opus"),
            TranscodeFormat::Mp3 => dest_path.join(relative).with_extension("mp3"),
        };
        let source_meta = entry.metadata().ok().and_then(|m| m.modified().ok());
        let target_meta = target.metadata().and_then(|m| m.modified()).ok();
        match (source_meta, target_meta) {
            (Some(source_time), Some(target_time)) if source_time > target_time => {
                if dry_run {
                    println!("{}", target.to_string_lossy());
                } else {
                    println!("{}", relative.to_string_lossy());
                    transcode_file(entry.path(), &target, format).expect("Error transcoding");
                }
            }
            (Some(_), None) => {
                if dry_run {
                    println!("{}", target.to_string_lossy());
                } else {
                    println!("{}", relative.to_string_lossy());
                    transcode_file(entry.path(), &target, format).expect("Error transcoding");
                }
            }
            _ => {
                // nothing
            }
        }
    });
}
