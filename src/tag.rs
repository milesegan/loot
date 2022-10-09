use std::path::Path;

use lofty::{read_from_path, Tag};

use crate::error::{AppError, Result};

pub fn read(path: &Path) -> Result<Tag> {
    let tagged_file = read_from_path(path, false)?;
    let tag = tagged_file.primary_tag().ok_or(AppError::ReadTagError)?;
    return Ok(tag.to_owned());
}
