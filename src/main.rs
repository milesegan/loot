use clap::{App, Arg, SubCommand};
use regex::Regex;
use std::path::Path;

mod tag;

fn tidy_string(string: &str) -> String {
    let tidied = string.trim().to_lowercase();
    let no_diacritics = deunicode::deunicode_with_tofu(&tidied, "_");
    let remove_regex = Regex::new(r#"[":\?\*'!,&\-\(\)\.]"#).unwrap();
    let removed = remove_regex.replace_all(&no_diacritics, "").into_owned();
    let replace_regex = Regex::new(r#"[/><|\\ ]+"#).unwrap();
    return replace_regex.replace_all(&removed, "_").into_owned();
}

fn process_file(base: &str, path: &str, dry_run: bool) -> tag::Result<()> {
    let the_regex = Regex::new(r#"^the "#).unwrap();

    let tag = tag::Tag::read(path)?;

    let extension = Path::new(&path)
        .extension()
        .ok_or(tag::TagError::ReadError)?
        .to_str()
        .ok_or(tag::TagError::ReadError)?;

    let tidy_artist = tidy_string(&(tag.album_artist.unwrap_or(tag.artist.clone())));

    let nicedir = format!(
        "{}/{}/{}",
        base,
        the_regex.replace_all(&tidy_artist, "").into_owned(),
        tidy_string(&tag.album)
    );

    let disc_prefix = tag.disc.map(|t| format!("{}-", t)).unwrap_or("".to_owned());

    let nicepath = format!(
        "{}/{}{:0>2}_{}.{}",
        nicedir,
        disc_prefix,
        tag.number,
        tidy_string(&tag.track),
        extension
    );

    if path.to_owned() != nicepath {
        println!("{} -> {}", path, nicepath);
        if !dry_run {
            std::fs::create_dir_all(&nicedir)?;
            std::fs::rename(&path, &nicepath)?;
        }
    }

    return Ok(());
}

fn normalize(path: &std::ffi::OsString, dry_run: bool) {
    let canonical = Path::new(path).canonicalize().expect("Invalid path.");
    let canonical_string = canonical.to_str().expect("Invalid path.");
    println!("processing {}", canonical_string);

    let pattern = format!("{}/**/*.{{flac,opus}}", canonical_string);
    for entry in globwalk::glob(pattern)
        .expect("Glob error.")
        .filter_map(|e| e.ok())
    {
        if let Some(path) = entry.path().to_str() {
            match process_file(&canonical_string, &path, dry_run) {
                Ok(_) => (),
                Err(_) => eprintln!("Error reading tag: {}", path),
            }
        }
    }
}

fn main() {
    let matches = App::new("frust")
        .version("1.0")
        .subcommand(
            SubCommand::with_name("norm").args(&[
                Arg::with_name("dry-run")
                    .short("d")
                    .long("dry-run")
                    .help("show changes but don't rename"),
                Arg::with_name("path")
                    .help("the root path of files to normalize")
                    .index(1)
                    .required(true),
            ]),
        )
        .get_matches();

    if let Some(norm) = matches.subcommand_matches("norm") {
        let dry_run = norm.args.contains_key("dry-run");
        normalize(&norm.args["path"].vals[0], dry_run);
    }
}
