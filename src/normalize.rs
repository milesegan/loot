use rayon::prelude::*;
use regex::Regex;
use std::path::Path;

use crate::tag;

fn tidy_string(string: &str) -> String {
    let tidied = string.trim();
    let no_diacritics = deunicode::deunicode_with_tofu(&tidied, "_").to_lowercase();
    let remove_regex = Regex::new(r#"["&,;.'(){}|:*!"-/?#>]"#).unwrap();
    let removed = remove_regex.replace_all(&no_diacritics, "").into_owned();
    let replace_regex = Regex::new(r#"[ ~]+"#).unwrap();
    return replace_regex.replace_all(&removed, "_").into_owned();
}

fn process_file(base: &Path, path: &Path, dry_run: bool) -> tag::Result<()> {
    let the_regex = Regex::new(r#"^the "#).unwrap();

    let tag = tag::Tag::read(path)?;

    let extension = Path::new(&path)
        .extension()
        .ok_or(tag::TagError::ReadError)?
        .to_str()
        .ok_or(tag::TagError::ReadError)?;

    let tidy_artist = tidy_string(&(tag.album_artist.unwrap_or(tag.artist.clone())));

    let nice_dir = base
        .join(the_regex.replace_all(&tidy_artist, "").into_owned())
        .join(tidy_string(&tag.album));

    let disc_prefix = tag.disc.map(|t| format!("{}-", t)).unwrap_or("".to_owned());

    let nice_path = nice_dir.join(format!(
        "{}{:0>2}_{}.{}",
        disc_prefix,
        tag.number,
        tidy_string(&tag.track),
        extension
    ));

    if path != nice_path {
        println!(
            "{} -> {}",
            path.strip_prefix(base)?.to_string_lossy(),
            nice_path.strip_prefix(base)?.to_string_lossy()
        );
        if !dry_run {
            std::fs::create_dir_all(&nice_dir)?;
            std::fs::rename(&path, &nice_path)?;
        }
    }

    return Ok(());
}

pub fn normalize(path: &std::ffi::OsString, dry_run: bool) {
    let canonical = Path::new(path).canonicalize().expect("Invalid path.");
    println!("processing {}", canonical.to_string_lossy());

    let pattern = format!("{}/**/*.{{flac,opus}}", canonical.to_string_lossy());
    globwalk::glob(pattern)
        .expect("Glob error.")
        .filter_map(|e| e.ok())
        .into_iter()
        .par_bridge()
        .for_each(
            |entry| match process_file(&canonical, entry.path(), dry_run) {
                Ok(_) => (),
                Err(_) => eprintln!("Error reading tag: {}", path.to_string_lossy()),
            },
        )
}
