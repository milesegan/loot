use regex::Regex;
use std::path::Path;

use crate::tag;

fn tidy_string(string: &str) -> String {
    let tidied = string.trim();
    let no_diacritics = deunicode::deunicode_with_tofu(&tidied, "_").to_lowercase();
    let remove_regex = Regex::new(r#"["+<>#:;?*'!,&\-().]"#).unwrap();
    let removed = remove_regex.replace_all(&no_diacritics, "").into_owned();
    let replace_regex = Regex::new(r#"[/><|\\ ]+"#).unwrap();
    return replace_regex.replace_all(&removed, "_").into_owned();
}

fn process_file(base: &str, path: &str, dry_run: bool) -> tag::Result<()> {
    let the_regex = Regex::new(r#"^the "#).unwrap();

    let tag = tag::Tag::read(path)?;

    let extension = Path::new(&path)
        .extension()
        .ok_or(tag::TagError::ReadError)?
        .to_str()
        .ok_or(tag::TagError::ReadError)?;

    let tidy_artist = tidy_string(&(tag.album_artist.unwrap_or(tag.artist.clone())));

    let nice_dir = format!(
        "{}/{}/{}",
        base,
        the_regex.replace_all(&tidy_artist, "").into_owned(),
        tidy_string(&tag.album)
    );

    let disc_prefix = tag.disc.map(|t| format!("{}-", t)).unwrap_or("".to_owned());

    let nice_path = format!(
        "{}/{}{:0>2}_{}.{}",
        nice_dir,
        disc_prefix,
        tag.number,
        tidy_string(&tag.track),
        extension
    );

    if path.to_owned() != nice_path {
        println!("{} -> {}", path, nice_path);
        if !dry_run {
            std::fs::create_dir_all(&nice_dir)?;
            std::fs::rename(&path, &nice_path)?;
        }
    }

    return Ok(());
}

pub fn normalize(path: &std::ffi::OsString, dry_run: bool) {
    let canonical = Path::new(path).canonicalize().expect("Invalid path.");
    let canonical_string = canonical.to_str().expect("Invalid path.");
    println!("processing {}", canonical_string);

    let pattern = format!("{}/**/*.{{flac,opus}}", canonical_string);
    for entry in globwalk::glob(pattern)
        .expect("Glob error.")
        .filter_map(|e| e.ok())
    {
        if let Some(path) = entry.path().to_str() {
            match process_file(&canonical_string, &path, dry_run) {
                Ok(_) => (),
                Err(_) => eprintln!("Error reading tag: {}", path),
            }
        }
    }
}
