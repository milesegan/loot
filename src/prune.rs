use std::path::Path;

pub fn prune(source_dirs: &[String], dest_dir: &str, dry_run: bool) {
    let canonical = Path::new(dest_dir).canonicalize().expect("Invalid path.");
    let canonical_string = canonical.to_str().expect("Invalid path.");
    println!("processing {}", canonical_string);

    let pattern = format!("{}/**/*.{{mp3,opus,m4a}}", canonical_string);
    globwalk::glob(&pattern)
        .expect("glob error")
        .filter_map(Result::ok)
        .into_iter()
        .for_each(|entry| {
            for source_dir in source_dirs {
                let source_path = Path::new(source_dir);
                let relative = entry.path().strip_prefix(dest_dir).expect("Not a prefix");
                let source = source_path.join(relative).with_extension("flac");
                if !source.exists() {
                    println!("{:?}", entry.path());
                    if !dry_run {
                        std::fs::remove_file(entry.path()).ok();
                    }
                }
            }
        });
}
