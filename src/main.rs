extern crate diacritics;
extern crate globwalk;
extern crate metaflac;
extern crate opus_headers;
extern crate unicode_normalization;

use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::io;
use std::num;
use std::path::Path;
use unicode_normalization::UnicodeNormalization;

enum TagError {
    ReadError,
}

impl From<num::ParseIntError> for TagError {
    fn from(_err: num::ParseIntError) -> TagError {
        TagError::ReadError
    }
}

impl From<io::Error> for TagError {
    fn from(_err: io::Error) -> TagError {
        TagError::ReadError
    }
}

impl From<opus_headers::ParseError> for TagError {
    fn from(_err: opus_headers::ParseError) -> TagError {
        TagError::ReadError
    }
}

impl From<metaflac::Error> for TagError {
    fn from(_err: metaflac::Error) -> TagError {
        TagError::ReadError
    }
}

type Result<T> = std::result::Result<T, TagError>;

enum FileType {
    Opus,
    Flac,
}

struct Tag {
    album: String,
    artist: String,
    // disc: Option<i8>,
    number: i32,
    track: String,
}

fn parse_number(tag: &str) -> Result<i32> {
    let number_regex = Regex::new(r"[^0-9].*").unwrap();
    match number_regex.replace_all(tag, "").parse::<i32>() {
        Ok(number) => Ok(number),
        Err(_) => Err(TagError::ReadError),
    }
}

impl Tag {
    fn read(path: &str) -> Result<Tag> {
        match file_type(path) {
            Some(FileType::Flac) => Tag::read_flac(path),
            Some(FileType::Opus) => Tag::read_opus(path),
            _ => Err(TagError::ReadError),
        }
    }

    fn read_flac(path: &str) -> Result<Tag> {
        fn extract_tag<'a>(tag: &'a HashMap<String, Vec<String>>, key: &str) -> Result<&'a String> {
            return tag
                .get(key)
                .ok_or(TagError::ReadError)?
                .get(0)
                .ok_or(TagError::ReadError);
        }
        let mut tag = metaflac::Tag::read_from_path(&path)?;
        let comments = &tag.vorbis_comments_mut().comments;
        let artist = extract_tag(&comments, "ARTIST")?;
        let album = extract_tag(&comments, "album")?;
        let track = extract_tag(&comments, "TITLE")?;
        let number = extract_tag(&comments, "TRACKNUMBER").and_then(|t| parse_number(t))?;

        return Ok(Tag {
            artist: artist.to_owned(),
            album: album.to_owned(),
            // disc: None,
            number: number.to_owned(),
            track: track.to_owned(),
        });
    }

    fn read_opus(path: &str) -> Result<Tag> {
        let headers = opus_headers::parse_from_path(path)?;
        let comments = headers.comments.user_comments;
        let artist = comments.get("ARTIST").ok_or(TagError::ReadError)?;
        let album = comments.get("ALBUM").ok_or(TagError::ReadError)?;
        let track = comments.get("TITLE").ok_or(TagError::ReadError)?;
        let number = comments
            .get("TRACKNUMBER")
            .ok_or(TagError::ReadError)
            .and_then(|t| parse_number(t))?;

        return Ok(Tag {
            artist: artist.to_owned(),
            album: album.to_owned(),
            // disc: None,
            number: number.to_owned(),
            track: track.to_owned(),
        });
    }
}

fn file_type(path: &str) -> Option<FileType> {
    let ext = Path::new(&path).extension()?.to_str()?;

    return match ext {
        "flac" => Some(FileType::Flac),
        "opus" => Some(FileType::Opus),
        _ => None,
    };
}

fn tidy_string(string: &str) -> String {
    return diacritics::remove_diacritics(string)
        .nfc()
        .to_string()
        .replace(r#"""#, "")
        .replace(":", "")
        .replace("?", "")
        .replace("'", "");
}

fn process_file(base: &str, path: &str) -> Result<()> {
    let tag = match Tag::read(path) {
        Ok(tag) => tag,
        Err(_) => {
            eprintln!("Error reading tag: {}", path);
            return Err(TagError::ReadError);
        }
    };

    let extension = Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown");

    let nicedir = format!(
        "{}/{}/{}",
        tidy_string(base),
        tidy_string(&tag.artist).replace("/", "-").trim(),
        tidy_string(&tag.album).replace("/", "-").trim()
    );

    let nicepath = format!(
        "{}/{:0>2} {}.{}",
        nicedir,
        tag.number,
        tidy_string(&tag.track).replace("/", "-").trim(),
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
            process_file(&canonical_string, &path).ok();
        }
    }
}
