use std::path::Path;

use lofty::{read_from_path, ItemKey, Probe, Tag, TagExt};

use crate::error::{AppError, Result};

pub fn read(path: &Path) -> Result<Tag> {
    let tagged_file = read_from_path(path, false)?;
    let tag = tagged_file.primary_tag().ok_or(AppError::ReadTagError)?;
    return Ok(tag.to_owned());
}

pub fn copy(src: &Path, dest: &Path) -> Result<()> {
    let tagged_file = read_from_path(src, false)?;
    let src_tag = tagged_file.primary_tag().ok_or(AppError::ReadTagError)?;

    let mut dest_file = Probe::open(&dest)?.read(false)?;
    let dest_tag = dest_file.primary_tag_mut().ok_or(AppError::WriteTagError)?;
    for item in src_tag.items() {
        dest_tag.push_item(item.clone());
    }
    if let Some(publisher) = src_tag.get_string(&ItemKey::Publisher) {
        dest_tag.insert_text(ItemKey::ContentGroup, publisher.to_string());
    }
    match dest_tag.save_to_path(dest) {
        Err(e) => {
            println!("{:?}", e);
            Err(AppError::WriteTagError)
        }
        Ok(o) => Ok(o),
    }
}
