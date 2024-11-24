use std::path::Path;

use lofty::{
    config::{ParseOptions, ParsingMode, WriteOptions},
    file::TaggedFileExt,
    probe::Probe,
    tag::{ItemKey, Tag, TagExt},
};

use crate::error::{AppError, Result};

pub fn read(path: &Path) -> Result<Tag> {
    let parsing_options = ParseOptions::new()
        .parsing_mode(ParsingMode::Relaxed)
        .read_cover_art(false);
    let tagged_file = Probe::open(path)?.options(parsing_options).read()?;
    let tag = tagged_file.primary_tag().ok_or(AppError::ReadTagError)?;
    return Ok(tag.to_owned());
}

pub fn copy(src: &Path, dest: &Path) -> Result<()> {
    let src_tag = read(src)?;

    let mut dest_file = Probe::open(&dest)?.read()?;
    let dest_tag = dest_file.primary_tag_mut().ok_or(AppError::WriteTagError)?;
    for item in src_tag.items() {
        dest_tag.push(item.clone());
    }
    if let Some(publisher) = src_tag.get_string(&ItemKey::Publisher) {
        dest_tag.insert_text(ItemKey::ContentGroup, publisher.to_string());
    }
    if let Some(compilation) = src_tag.get_string(&ItemKey::FlagCompilation) {
        dest_tag.insert_text(ItemKey::FlagCompilation, compilation.to_string());
    }
    if let Some(work) = src_tag.get_string(&ItemKey::Work) {
        dest_tag.insert_text(ItemKey::Work, work.to_string());
    }
    match dest_tag.save_to_path(dest, WriteOptions::new()) {
        Err(e) => {
            println!("{:?}", e);
            Err(AppError::WriteTagError)
        }
        Ok(o) => Ok(o),
    }
}
