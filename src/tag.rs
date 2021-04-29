use regex::Regex;
use std::collections::HashMap;
use std::io;
use std::num;
use std::path::Path;

pub enum TagError {
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

pub type Result<T> = std::result::Result<T, TagError>;

enum FileType {
    Opus,
    Flac,
}

pub struct Tag {
    pub album: String,
    pub artist: String,
    // disc: Option<i8>,
    pub number: i32,
    pub track: String,
    pub year: i32,
}

fn extract_tag<'a>(tag: &'a HashMap<String, Vec<String>>, key: &str) -> Result<&'a String> {
    return tag
        .get(key)
        .ok_or(TagError::ReadError)?
        .get(0)
        .ok_or(TagError::ReadError);
}

fn parse_number(tag: &str) -> Result<i32> {
    let number_regex = Regex::new(r"[^0-9].*").unwrap();
    return number_regex
        .replace_all(tag, "")
        .parse::<i32>()
        .map_err(|_| TagError::ReadError);
}

fn file_type(path: &str) -> Option<FileType> {
    let ext = Path::new(&path).extension()?.to_str()?;

    return match ext {
        "flac" => Some(FileType::Flac),
        "opus" => Some(FileType::Opus),
        _ => None,
    };
}

impl Tag {
    pub fn read(path: &str) -> Result<Tag> {
        match file_type(path) {
            Some(FileType::Flac) => Tag::read_flac(path),
            Some(FileType::Opus) => Tag::read_opus(path),
            _ => Err(TagError::ReadError),
        }
    }

    fn read_flac(path: &str) -> Result<Tag> {
        let mut tag = metaflac::Tag::read_from_path(&path)?;
        let comments = &tag.vorbis_comments_mut().comments;
        let artist = extract_tag(&comments, "ARTIST")?;
        let album = extract_tag(&comments, "ALBUM")?;
        let track = extract_tag(&comments, "TITLE")?;
        let number = extract_tag(&comments, "TRACKNUMBER").and_then(|t| parse_number(t))?;
        let date = extract_tag(&comments, "DATE").and_then(|t| parse_number(t));
        let year = extract_tag(&comments, "YEAR").and_then(|t| parse_number(t));
        let tag_year = date.or(year)?;

        return Ok(Tag {
            artist: artist.to_owned(),
            album: album.to_owned(),
            // disc: None,
            number,
            track: track.to_owned(),
            year: tag_year,
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
        let date = comments
            .get("DATE")
            .ok_or(TagError::ReadError)
            .and_then(|t| parse_number(t));
        let year = comments
            .get("YEAR")
            .ok_or(TagError::ReadError)
            .and_then(|t| parse_number(t));
        let tag_year = date.or(year)?;

        return Ok(Tag {
            artist: artist.to_owned(),
            album: album.to_owned(),
            // disc: None,
            number,
            track: track.to_owned(),
            year: tag_year,
        });
    }
}
