use filetime::FileTime;
use globwalk::DirEntry;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::fs_utils::{canonicalize_path, glob_pattern, modified_time};
use crate::tag;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;

pub(crate) const SOURCE_EXTENSIONS: &[&str] = &["flac", "opus", "m4a"];

/// Supported output formats for the transcode workflow.
#[derive(Copy, Clone)]
pub enum TranscodeFormat {
    Aac {
        mode: AacBitrateMode,
        /// Embed the source's cover art in each file, in addition to the folder `cover.jpg`.
        embed_cover: bool,
    },
    Opus {
        bitrate_kbps: Option<u32>,
    },
    Mp3,
    Flac,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AacBitrateMode {
    /// True VBR at an encoder quality level (0-127).
    Vbr {
        quality: u8,
    },
    Cbr {
        bitrate_kbps: u32,
    },
}

/// TVBR quality step that averages roughly 128 kbps for stereo music.
pub const DEFAULT_AAC_VBR_QUALITY: u8 = 63;

fn target_path(dest_path: &Path, relative: &Path, format: TranscodeFormat) -> PathBuf {
    match format {
        TranscodeFormat::Aac { .. } => dest_path.join(relative).with_extension("m4a"),
        TranscodeFormat::Opus { .. } => dest_path.join(relative).with_extension("opus"),
        TranscodeFormat::Mp3 => dest_path.join(relative).with_extension("mp3"),
        TranscodeFormat::Flac => dest_path.join(relative).with_extension("flac"),
    }
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

fn aac_encoder_args(mode: AacBitrateMode) -> Vec<String> {
    // afconvert ignores -b in true VBR mode, so VBR is driven by vbrq instead.
    let mut args = match mode {
        AacBitrateMode::Vbr { quality } => vec![
            "-s".to_owned(),
            "3".to_owned(),
            "-ue".to_owned(),
            "vbrq".to_owned(),
            quality.to_string(),
        ],
        AacBitrateMode::Cbr { bitrate_kbps } => vec![
            "-s".to_owned(),
            "0".to_owned(),
            "-b".to_owned(),
            (u64::from(bitrate_kbps) * 1000).to_string(),
        ],
    };
    // Highest encoder quality (slowest search).
    args.push("-q".to_owned());
    args.push("127".to_owned());
    args
}

fn opus_encoder_args(bitrate_kbps: Option<u32>) -> Vec<String> {
    match bitrate_kbps {
        Some(bitrate_kbps) => vec!["-b:a".to_owned(), format!("{}k", bitrate_kbps)],
        None => Vec::new(),
    }
}

fn transcode_file(source: &Path, dest: &Path, format: TranscodeFormat) -> std::io::Result<()> {
    fs::remove_file(dest).ok();
    let mut tmp = PathBuf::from(dest);
    tmp.set_extension("tmp");
    let source_meta = fs::metadata(source)?;

    let child = match format {
        TranscodeFormat::Opus { bitrate_kbps } => {
            let mut command = std::process::Command::new("ffmpeg");
            command
                .arg("-y")
                .arg("-loglevel")
                .arg("quiet")
                .arg("-i")
                .arg(source)
                .arg("-c:a")
                .arg("libopus")
                .arg("-map")
                .arg("a:0");
            for arg in opus_encoder_args(bitrate_kbps) {
                command.arg(arg);
            }
            command.arg("-f").arg("opus").arg(tmp.as_path()).spawn()?
        }
        TranscodeFormat::Aac { mode, .. } => {
            let mut command = std::process::Command::new("afconvert");
            command.arg("-d").arg("aac").arg("-f").arg("m4af");
            for arg in aac_encoder_args(mode) {
                command.arg(arg);
            }
            command.arg(source).arg(tmp.as_path()).spawn()?
        }
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
            .arg(tmp.as_path())
            .spawn()?,
        TranscodeFormat::Flac => std::process::Command::new("ffmpeg")
            .arg("-y")
            .arg("-loglevel")
            .arg("quiet")
            .arg("-i")
            .arg(source)
            .arg("-map_metadata")
            .arg("0")
            .arg("-map")
            .arg("0")
            .arg("-map")
            .arg("-0:1")
            .arg("-c:a")
            .arg("flac")
            .arg("-f")
            .arg("flac")
            .arg(tmp.as_path())
            .spawn()?,
    };
    let output = child.wait_with_output()?;
    if !output.status.success() {
        fs::remove_file(&tmp).ok();
        return Err(std::io::Error::other(format!(
            "encoder exited with {}",
            output.status
        )));
    }
    fs::create_dir_all(dest.parent().unwrap())?;
    fs::rename(tmp.as_path(), dest)?;
    match format {
        TranscodeFormat::Aac { embed_cover, .. } => {
            if let Err(e) = tag::copy(source, dest, embed_cover) {
                // Remove the untagged output so the next run retries it instead of
                // treating its fresh mtime as up to date.
                fs::remove_file(dest).ok();
                return Err(std::io::Error::other(format!("copying tags: {:?}", e)));
            }
        }
        _ => (),
    }

    let mtime = FileTime::from_last_modification_time(&source_meta);
    filetime::set_file_mtime(dest, mtime)?;
    touch_parents(dest)?;

    return Ok(());
}

fn extract_cover(source: &Path, dest: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dest.parent().unwrap())?;
    let child = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-loglevel")
        .arg("quiet")
        .arg("-i")
        .arg(source)
        .arg("-an")
        .arg(dest)
        .spawn()?;
    child.wait_with_output()?;
    return Ok(());
}

/// Transcodes supported source files into a destination tree while preserving metadata.
pub fn transcode(source_paths: &[String], dest_dir: &str, dry_run: bool, format: TranscodeFormat) {
    let canonicals = source_paths
        .iter()
        .map(canonicalize_path)
        .collect::<Vec<_>>();

    for canonical in &canonicals {
        let canonical_string = canonical.to_str().expect("Invalid path.");
        println!("processing {}", canonical_string);
    }

    let dest_path = Path::new(dest_dir);
    for canonical_path in canonicals {
        let pattern = glob_pattern(&canonical_path, SOURCE_EXTENSIONS);
        let mut matches = globwalk::glob(&pattern)
            .expect("glob error")
            .filter_map(Result::ok)
            .into_iter()
            .collect::<Vec<DirEntry>>();
        matches.sort_by(|a, b| a.path().cmp(b.path()));

        // Collect all files to process
        let files_to_process = matches;
        let total = files_to_process.len() as u64;
        let pb = Arc::new(ProgressBar::new(total));
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
                )
                .unwrap()
                .progress_chars("█▉▊▋▌▍▎▏  "),
        );
        pb.set_message("Transcoding...");

        let pb_clone = pb.clone();
        files_to_process.into_par_iter().for_each(|entry| {
            let source_meta = modified_time(entry.path());
            let relative = entry
                .path()
                .strip_prefix(&canonical_path)
                .expect("Not a prefix");
            let cover = dest_path
                .join(relative)
                .with_file_name("cover")
                .with_extension("jpg");
            let cover_meta = modified_time(&cover);
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
            let target = target_path(dest_path, relative, format);
            let target_meta = modified_time(&target);
            let file_display = relative.to_string_lossy();
            match (source_meta, target_meta) {
                (Some(source_time), Some(target_time))
                    if round_time(source_time) > round_time(target_time) =>
                {
                    if dry_run {
                        pb_clone.set_message(format!("{}", file_display));
                        pb_clone.inc(1);
                    } else {
                        pb_clone.set_message(format!("{}", file_display));
                        if let Err(e) = transcode_file(entry.path(), &target, format) {
                            pb_clone
                                .suspend(|| eprintln!("Error transcoding {}: {}", file_display, e));
                        }
                        pb_clone.inc(1);
                    }
                }
                (Some(_), None) => {
                    if dry_run {
                        pb_clone.set_message(format!("{}", file_display));
                        pb_clone.inc(1);
                    } else {
                        pb_clone.set_message(format!("{}", file_display));
                        if let Err(e) = transcode_file(entry.path(), &target, format) {
                            pb_clone
                                .suspend(|| eprintln!("Error transcoding {}: {}", file_display, e));
                        }
                        pb_clone.inc(1);
                    }
                }
                _ => {
                    // nothing
                }
            }
        });
        pb.finish_with_message("Done");
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::{Duration, SystemTime};

    use crate::fs_utils::glob_pattern;

    use super::SOURCE_EXTENSIONS;
    use super::{
        aac_encoder_args, opus_encoder_args, round_time, target_path, AacBitrateMode,
        TranscodeFormat,
    };

    #[test]
    fn target_path_uses_expected_extension_for_each_format() {
        let dest = Path::new("/tmp/output");
        let relative = Path::new("Artist/Album/track.flac");

        assert_eq!(
            target_path(
                dest,
                relative,
                TranscodeFormat::Aac {
                    mode: AacBitrateMode::Vbr { quality: 63 },
                    embed_cover: false,
                }
            ),
            dest.join("Artist/Album/track.m4a")
        );
        assert_eq!(
            target_path(dest, relative, TranscodeFormat::Opus { bitrate_kbps: None }),
            dest.join("Artist/Album/track.opus")
        );
        assert_eq!(
            target_path(dest, relative, TranscodeFormat::Mp3),
            dest.join("Artist/Album/track.mp3")
        );
        assert_eq!(
            target_path(dest, relative, TranscodeFormat::Flac),
            dest.join("Artist/Album/track.flac")
        );
    }

    #[test]
    fn source_extensions_include_apple_lossless_m4a() {
        assert_eq!(
            glob_pattern(Path::new("/tmp/music"), SOURCE_EXTENSIONS),
            "/tmp/music/**/*.{flac,opus,m4a}"
        );
    }

    #[test]
    fn round_time_returns_epoch_milliseconds() {
        let time = SystemTime::UNIX_EPOCH + Duration::from_millis(1234);
        assert_eq!(round_time(time), 1234);
    }

    #[test]
    fn aac_encoder_args_set_strategy_and_rate_control() {
        assert_eq!(
            aac_encoder_args(AacBitrateMode::Vbr { quality: 63 }),
            vec!["-s", "3", "-ue", "vbrq", "63", "-q", "127"]
        );
        assert_eq!(
            aac_encoder_args(AacBitrateMode::Cbr { bitrate_kbps: 256 }),
            vec!["-s", "0", "-b", "256000", "-q", "127"]
        );
    }

    #[test]
    fn opus_encoder_args_only_set_bitrate_when_requested() {
        assert_eq!(opus_encoder_args(None), Vec::<String>::new());
        assert_eq!(
            opus_encoder_args(Some(192)),
            vec!["-b:a".to_owned(), "192k".to_owned()]
        );
    }
}
