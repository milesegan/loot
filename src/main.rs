extern crate diacritics;
extern crate metaflac;
extern crate opus_headers;

use glob::glob;
use metaflac::Tag;
// use opus_headers;
use std::env;
use std::path::Path; // or parse_from_read or parse_from_file

fn tidy_string(string: &str) -> String {
    return diacritics::remove_diacritics(string);
}

fn process_file_flac(base: &str, path: &str) -> Option<i32> {
    let mut tag = Tag::read_from_path(&path).ok()?;
    let comments = &tag.vorbis_comments_mut().comments;
    let artist = &comments.get("ALBUMARTIST")?.get(0)?; //["ALBUMARTIST"];
    let album = &comments.get("ALBUM")?.get(0)?;
    let track = &comments.get("TITLE")?.get(0)?;
    let number = &comments.get("TRACKNUMBER")?.get(0)?;

    let nicedir = format!(
        "{}/{}/{}",
        tidy_string(base),
        tidy_string(artist),
        tidy_string(album)
    );

    let nicepath = format!(
        "{}/{:0>2} {}.flac",
        nicedir,
        number,
        tidy_string(track).replace("/", "-")
    );

    println!("{}", path);
    if path != nicepath {
        println!("{} -> {}", path, nicepath);
        // std::fs::create_dir_all(&nicedir).expect("can't make directories");
        // std::fs::rename(&path, &nicepath).expect("rename failed");
    }

    return Some(0);
}

fn process_file_opus(base: &str, path: &str) -> Option<i32> {
    let headers = opus_headers::parse_from_path(path).unwrap();
    let comments = headers.comments.user_comments;

    let artist = &comments.get("ALBUMARTIST")?;
    let album = &comments.get("ALBUM")?;
    let track = &comments.get("TITLE")?;
    let number = &comments.get("TRACKNUMBER")?;

    let nicedir = format!(
        "{}/{}/{}",
        tidy_string(base),
        tidy_string(artist),
        tidy_string(album)
    );

    let nicepath = format!(
        "{}/{:0>2} {}.opus",
        nicedir,
        number,
        tidy_string(track).replace("/", "-")
    );

    if path != nicepath {
        println!("{} -> {}", path, nicepath);
        std::fs::create_dir_all(&nicedir).expect("can't make directories");
        std::fs::rename(&path, &nicepath).expect("rename failed");
    }

    return Some(0);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args.get(1).expect("No path specified.");

    let canonical = Path::new(path).canonicalize().expect("Invalid path.");
    let canonical_string = canonical.to_str().expect("Invalid path.");
    println!("{}", canonical_string);

    let pattern = format!("{}/**/*.opus", canonical_string);
    let entries = glob(&pattern).expect("Illegal glob pattern.");
    for entry in entries {
        process_file_opus(&canonical_string, &entry.unwrap().to_str().unwrap());
    }
}
