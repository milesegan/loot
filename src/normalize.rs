use lofty::tag::{Accessor, ItemKey};
use rayon::prelude::*;
use std::path::Path;

use crate::error::{AppError, Result};
use crate::fs_utils::{canonicalize_path, glob_pattern, relative_path_string};
use crate::tag;
use crate::text::{strip_leading_the, tidy_string};

fn process_file(base: &Path, path: &Path, dry_run: bool) -> Result<()> {
    let tag = tag::read(path, false)?;

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or(AppError::PathError)?;

    let is_compilation = tag
        .get_string(ItemKey::FlagCompilation)
        .unwrap_or("")
        .eq("1");
    let album_artist = tag.get_string(ItemKey::AlbumArtist);
    let tidy_artist = if is_compilation {
        "various".to_string()
    } else {
        strip_leading_the(&tidy_string(
            album_artist.unwrap_or(tag.artist().as_deref().unwrap_or("")),
        ))
    };

    let nice_dir = base
        .join(&tidy_artist)
        .join(tidy_string(tag.album().as_deref().unwrap_or("")));

    let disc_prefix = tag
        .disk()
        .map(|t| format!("{}-", t))
        .unwrap_or("".to_owned());

    let nice_path = nice_dir.join(format!(
        "{}{:0>2}_{}.{}",
        disc_prefix,
        tag.track().unwrap_or(1),
        tidy_string(&tag.title().as_deref().unwrap_or("")),
        extension
    ));

    if path != nice_path {
        println!(
            "{} -> {}",
            relative_path_string(base, path)?,
            relative_path_string(base, &nice_path)?
        );
        if !dry_run {
            std::fs::create_dir_all(&nice_dir)?;
            std::fs::rename(&path, &nice_path)?;
        }
    }

    return Ok(());
}

/// Renames supported audio files into a normalized artist/album/track layout.
pub fn normalize(path: &str, dry_run: bool) {
    let canonical = canonicalize_path(path);
    println!("processing {}", canonical.to_string_lossy());

    let pattern = glob_pattern(&canonical, &["flac", "opus", "m4a", "mp3"]);
    globwalk::glob(pattern)
        .expect("Glob error.")
        .filter_map(|e| e.ok())
        .into_iter()
        .par_bridge()
        .for_each(
            |entry| match process_file(&canonical, entry.path(), dry_run) {
                Ok(_) => (),
                Err(_) => eprintln!("Error reading tag: {}", entry.path().to_string_lossy()),
            },
        )
}
