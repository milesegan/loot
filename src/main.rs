extern crate diacritics;
extern crate globwalk;
extern crate metaflac;
extern crate opus_headers;
extern crate unicode_normalization;

// use metaflac::Tag;
// use opus_headers;
use std::env;
use std::path::Path; // or parse_from_read or parse_from_file
use unicode_normalization::UnicodeNormalization;

enum FileType {
    Opus,
    Flac,
}

struct Tag {
    artist: String,
    album: String,
    track: String,
    number: i8,
}

impl Tag {
    fn read(path: &str) -> Option<Tag> {
        match file_type(path)? {
            FileType::Flac => Tag::read_flac(path),
            FileType::Opus => Tag::read_opus(path),
        }
    }

    fn read_flac(path: &str) -> Option<Tag> {
        let mut tag = metaflac::Tag::read_from_path(&path).ok()?;
        let comments = &tag.vorbis_comments_mut().comments;
        let artist = comments.get("ALBUMARTIST")?.get(0)?;
        let album = comments.get("ALBUM")?.get(0)?;
        let track = comments.get("TITLE")?.get(0)?;
        let number = comments.get("TRACKNUMBER")?.get(0)?.parse::<i8>().ok()?;

        return Some(Tag {
            artist: artist.to_owned(),
            album: album.to_owned(),
            track: track.to_owned(),
            number: number.to_owned(),
        });
    }

    fn read_opus(path: &str) -> Option<Tag> {
        let headers = opus_headers::parse_from_path(path).unwrap();
        let comments = headers.comments.user_comments;
        let artist = comments.get("ALBUMARTIST")?;
        let album = comments.get("ALBUM")?;
        let track = comments.get("TITLE")?;
        let number = comments.get("TRACKNUMBER")?.parse::<i8>().ok()?;

        return Some(Tag {
            artist: artist.to_owned(),
            album: album.to_owned(),
            track: track.to_owned(),
            number: number.to_owned(),
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
    return diacritics::remove_diacritics(string).nfc().collect();
}

fn process_file(base: &str, path: &str) -> Result<(), std::io::Error> {
    let tag = match Tag::read(path) {
        Some(tag) => tag,
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Coudn't read tags.",
            ))
        }
    };

    let nicedir = format!(
        "{}/{}/{}",
        tidy_string(base),
        tidy_string(&tag.artist),
        tidy_string(&tag.album)
    );

    let nicepath = format!(
        "{}/{:0>2} {}.flac",
        nicedir,
        tag.number,
        tidy_string(&tag.track).replace("/", "-")
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
