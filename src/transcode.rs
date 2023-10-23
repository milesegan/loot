use rayon::prelude::*;
use std::path::Path;

#[derive(Copy, Clone)]
pub enum TranscodeFormat {
    Aac,
    Ogg,
    Opus,
    Mp3,
}

fn transcode_file(source: &Path, dest: &Path, format: TranscodeFormat) -> std::io::Result<()> {
    std::fs::remove_file(dest).ok();
    let tmp = tempfile::NamedTempFile::new()?;

    let child = match format {
        TranscodeFormat::Opus => std::process::Command::new("opusenc")
            .arg("--quiet")
            .arg("--bitrate")
            .arg("96")
            .arg("--discard-pictures")
            .arg(source)
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
        TranscodeFormat::Ogg => std::process::Command::new("oggenc")
            .arg("--quality")
            .arg("6")
            .arg("--quiet")
            .arg(source)
            .arg("--output")
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
    let child = std::process::Command::new("metaflac")
        .arg("--export-picture-to")
        .arg(dest)
        .arg(source)
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
            let target = match format {
                TranscodeFormat::Aac => dest_path.join(relative).with_extension("m4a"),
                TranscodeFormat::Ogg => dest_path.join(relative).with_extension("ogg"),
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
