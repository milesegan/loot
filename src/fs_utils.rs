use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::Result;

/// Canonicalizes a user-provided path with the consistent CLI error message.
pub fn canonicalize_path(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref().canonicalize().expect("Invalid path.")
}

/// Builds a recursive `globwalk` pattern rooted at `base` for the given extensions.
pub fn glob_pattern(base: &Path, extensions: &[&str]) -> String {
    format!(
        "{}/**/*.{{{}}}",
        base.to_string_lossy(),
        extensions.join(",")
    )
}

/// Returns a displayable relative path string for `path` within `base`.
pub fn relative_path_string(base: &Path, path: &Path) -> Result<String> {
    Ok(path.strip_prefix(base)?.to_string_lossy().into_owned())
}

/// Returns the modified time for a file if its metadata can be read.
pub fn modified_time(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

/// Converts a system time into seconds since the Unix epoch.
pub fn unix_timestamp_secs(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::{glob_pattern, modified_time, relative_path_string, unix_timestamp_secs};

    #[test]
    fn creates_glob_patterns_for_multiple_extensions() {
        let pattern = glob_pattern(Path::new("/tmp/music"), &["flac", "mp3"]);
        assert_eq!(pattern, "/tmp/music/**/*.{flac,mp3}");
    }

    #[test]
    fn returns_relative_path_strings() {
        let base = Path::new("/tmp/library");
        let child = base.join("Artist/Album/track.flac");

        let relative = relative_path_string(base, &child).expect("expected relative path");

        assert_eq!(relative, "Artist/Album/track.flac");
    }

    #[test]
    fn reads_file_modified_times() {
        let dir = tempdir().expect("expected temp dir");
        let file = dir.path().join("track.flac");
        fs::write(&file, b"audio").expect("expected temp file write to succeed");

        let modified = modified_time(&file);

        assert!(modified.is_some());
        assert!(unix_timestamp_secs(modified.expect("mtime")) > 0);
    }
}
