use std::path::Path;

use crate::fs_utils::{canonicalize_path, glob_pattern};

fn has_source_counterpart(source_dirs: &[String], relative: &Path) -> bool {
    source_dirs.iter().any(|source_dir| {
        Path::new(source_dir)
            .join(relative)
            .with_extension("flac")
            .exists()
    })
}

/// Removes transcoded files from the destination when the source FLAC no longer exists.
pub fn prune(source_dirs: &[String], dest_dir: &str, dry_run: bool) {
    let canonical = canonicalize_path(dest_dir);
    let canonical_string = canonical.to_str().expect("Invalid path.");
    println!("processing {}", canonical_string);

    let pattern = glob_pattern(&canonical, &["mp3", "opus", "m4a"]);
    globwalk::glob(&pattern)
        .expect("glob error")
        .filter_map(Result::ok)
        .into_iter()
        .for_each(|entry| {
            let relative = entry.path().strip_prefix(&canonical).expect("Not a prefix");
            if has_source_counterpart(source_dirs, relative) {
                return;
            }
            println!("{:?}", entry.path());
            if !dry_run {
                std::fs::remove_file(entry.path()).ok();
            }
        });
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::has_source_counterpart;

    #[test]
    fn detects_existing_flac_in_any_source_dir() {
        let source_a = tempdir().expect("tempdir");
        let source_b = tempdir().expect("tempdir");
        let relative = Path::new("Artist/Album/track.mp3");
        let flac_path = source_b.path().join(relative).with_extension("flac");
        fs::create_dir_all(flac_path.parent().expect("parent")).expect("mkdirs");
        fs::write(&flac_path, b"audio").expect("write");

        let sources = vec![
            source_a.path().to_string_lossy().into_owned(),
            source_b.path().to_string_lossy().into_owned(),
        ];

        assert!(has_source_counterpart(&sources, relative));
    }

    #[test]
    fn returns_false_when_no_source_flac_exists() {
        let source = tempdir().expect("tempdir");
        let sources = vec![source.path().to_string_lossy().into_owned()];

        assert!(!has_source_counterpart(
            &sources,
            Path::new("Artist/Album/missing.m4a")
        ));
    }
}
