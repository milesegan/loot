use filetime::FileTime;
use globwalk::DirEntry;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::tag;

#[derive(Copy, Clone)]
pub enum TranscodeFormat {
    Aac,
    Opus,
    Mp3,
}

fn touch_parents(path: &Path) -> Result<(), std::io::Error> {
    let mut current_path = PathBuf::new();
    let now = FileTime::now();
    for component in path.components() {
        current_path.push(component);
        if let Some(parent) = current_path.parent() {
            let _ = filetime::set_file_mtime(parent, now);
        }
    }
    Ok(())
}

fn round_time(time: SystemTime) -> u128 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis()
}

fn transcode_file(source: &Path, dest: &Path, format: TranscodeFormat) -> std::io::Result<()> {
    fs::remove_file(dest).ok();
    let mut tmp = PathBuf::from(dest);
    tmp.set_extension("tmp");
    let source_meta = fs::metadata(source)?;

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
            .arg("192k")
            .arg("-f")
            .arg("opus")
            .arg(tmp.as_path())
            .spawn()
            .expect("failed to execute child"),
        TranscodeFormat::Aac => std::process::Command::new("afconvert")
            //        afconvert -d aac -f m4af -s 3 -b 192000 test.flac test.m4a
            .arg("-d")
            .arg("aac")
            .arg("-f")
            .arg("m4af")
            .arg("-s")
            .arg("3")
            .arg("-ue")
            .arg("vbrq")
            // .arg("45") ~ 96
            .arg("64") // ~ 128
            .arg(source)
            .arg(tmp.as_path())
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
            .arg("3")
            .arg("-f")
            .arg("mp3")
            .arg(tmp.as_path())
            .spawn()
            .expect("failed to execute child"),
    };
    child.wait_with_output().expect("failed to wait on child");
    fs::create_dir_all(dest.parent().unwrap()).expect("Error making dest dir");
    fs::rename(tmp.as_path(), dest).expect("Error moving file");
    match format {
        TranscodeFormat::Aac => tag::copy(source, dest).expect("Error copying tag"),
        _ => (),
    }

    let mtime = FileTime::from_last_modification_time(&source_meta);
    filetime::set_file_mtime(dest, mtime)?;
    touch_parents(dest)?;

    return Ok(());
}

fn extract_cover(source: &Path, dest: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dest.parent().unwrap()).expect("Error making dest dir");
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

pub fn transcode(source_paths: &[String], dest_dir: &str, dry_run: bool, format: TranscodeFormat) {
    let canonicals = source_paths
        .into_iter()
        .map(|path| {
            Path::new(path)
                .canonicalize()
                .expect("Invalid path.")
                .as_path()
                .to_owned()
        })
        .collect::<Vec<_>>();

    for canonical in &canonicals {
        let canonical_string = canonical.to_str().expect("Invalid path.");
        println!("processing {}", canonical_string);
    }

    let dest_path = Path::new(dest_dir);
    for canonical_string in canonicals {
        let pattern = format!(
            "{}/**/*.{{flac,opus}}",
            canonical_string.to_string_lossy().to_owned()
        );
        let mut matches = globwalk::glob(&pattern)
            .expect("glob error")
            .filter_map(Result::ok)
            .into_iter()
            .collect::<Vec<DirEntry>>();
        matches.sort_by(|a, b| a.path().cmp(b.path()));
        matches.into_iter().par_bridge().for_each(|entry| {
            let source_meta = entry.metadata().ok().and_then(|m| m.modified().ok());
            let relative = entry
                .path()
                .strip_prefix(&canonical_string)
                .expect("Not a prefix");
            let cover = dest_path
                .join(relative)
                .with_file_name("cover")
                .with_extension("jpg");
            let cover_meta = cover.metadata().and_then(|m| m.modified()).ok();
            match (source_meta, cover_meta) {
                (Some(source_time), Some(target_time)) if source_time > target_time => {
                    extract_cover(entry.path(), &cover).ok();
                }
                (Some(_), None) => {
                    extract_cover(entry.path(), &cover).ok();
                }
                _ => {
                    // nothing
                }
            }
            let target = match format {
                TranscodeFormat::Aac => dest_path.join(relative).with_extension("m4a"),
                TranscodeFormat::Opus => dest_path.join(relative).with_extension("opus"),
                TranscodeFormat::Mp3 => dest_path.join(relative).with_extension("mp3"),
            };
            let target_meta = target.metadata().and_then(|m| m.modified()).ok();
            match (source_meta, target_meta) {
                (Some(source_time), Some(target_time))
                    if round_time(source_time) > round_time(target_time) =>
                {
                    if dry_run {
                        println!("{}", target.to_string_lossy(),);
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
}
