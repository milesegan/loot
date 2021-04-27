use regex::Regex;
use std::env;
use std::path::Path;
use unicode_normalization::UnicodeNormalization;

mod tag;

fn tidy_string(string: &str) -> String {
    let tidied = diacritics::remove_diacritics(string)
        .nfc()
        .collect::<String>()
        .replace("/", "-")
        .trim()
        .to_lowercase();

    let remove_regex = Regex::new(r#"[":?']"#).unwrap();
    let removed = remove_regex.replace_all(&tidied, "").into_owned();

    let replace_regex = Regex::new(r#"[><|]"#).unwrap();
    return replace_regex.replace_all(&removed, "-").into_owned();
}

fn process_file(base: &str, path: &str) -> tag::Result<()> {
    let tag = tag::Tag::read(path)?;

    let extension = Path::new(&path)
        .extension()
        .ok_or(tag::TagError::ReadError)?
        .to_str()
        .ok_or(tag::TagError::ReadError)?;

    let nicedir = format!(
        "{}/{}/{}",
        base,
        tidy_string(&tag.artist),
        tidy_string(&tag.album)
    );

    let nicepath = format!(
        "{}/{:0>2} {}.{}",
        nicedir,
        tag.number,
        tidy_string(&tag.track),
        extension
    );

    if path.nfc().collect::<String>() != nicepath {
        println!("{} -> {}", path, nicepath);
        std::fs::create_dir_all(&nicedir)?;
        std::fs::rename(&path, &nicepath)?;
    }

    return Ok(());
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args.get(1).expect("No path specified.");

    let canonical = Path::new(path).canonicalize().expect("Invalid path.");
    let canonical_string = canonical.to_str().expect("Invalid path.");
    println!("processing {}", canonical_string);

    let pattern = format!("{}/**/*.{{flac,opus}}", canonical_string);
    for entry in globwalk::glob(pattern)
        .expect("Glob error.")
        .filter_map(|e| e.ok())
    {
        if let Some(path) = entry.path().to_str() {
            match process_file(&canonical_string, &path) {
                Ok(_) => (),
                Err(_) => eprintln!("Error reading tag: {}", path),
            }
        }
    }
}
