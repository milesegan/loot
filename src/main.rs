use clap::{App, Arg, SubCommand};

mod error;
mod normalize;
mod prune;
mod tag;
mod transcode;

fn main() {
    let matches = App::new("loot")
        .version("0.8.0")
        .subcommand(
            SubCommand::with_name("norm")
                .about("Normalize file paths.")
                .args(&[
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
        .subcommand(
            SubCommand::with_name("transcode-aac")
                .about("Transcode flac files to aac.")
                .args(&[
                    Arg::with_name("dry-run")
                        .short("d")
                        .long("dry-run")
                        .help("show changes but don't transcode"),
                    Arg::with_name("source_path")
                        .help("The path of files to transcode.")
                        .required(true),
                    Arg::with_name("dest_path")
                        .help("The destination path to write transcode files.")
                        .required(true),
                ]),
        )
        .subcommand(
            SubCommand::with_name("transcode-opus")
                .about("Transcode flac files to opus.")
                .args(&[
                    Arg::with_name("dry-run")
                        .short("d")
                        .long("dry-run")
                        .help("show changes but don't transcode"),
                    Arg::with_name("source_path")
                        .help("The path of files to transcode.")
                        .required(true),
                    Arg::with_name("dest_path")
                        .help("The destination path to write transcode files.")
                        .required(true),
                ]),
        )
        .subcommand(
            SubCommand::with_name("transcode-mp3")
                .about("Transcode flac files to mp3.")
                .args(&[
                    Arg::with_name("dry-run")
                        .short("d")
                        .long("dry-run")
                        .help("show changes but don't transcode"),
                    Arg::with_name("source_path")
                        .help("The path of files to transcode.")
                        .required(true),
                    Arg::with_name("dest_path")
                        .help("The destination path to write transcode files.")
                        .required(true),
                ]),
        )
        .subcommand(
            SubCommand::with_name("prune")
                .about("Prune transcoded files.")
                .args(&[
                    Arg::with_name("dry-run")
                        .short("d")
                        .long("dry-run")
                        .help("show changes but don't prune"),
                    Arg::with_name("source_path")
                        .help("The path of files to transcode.")
                        .required(true),
                    Arg::with_name("dest_path")
                        .help("The destination path to write transcode files.")
                        .required(true),
                ]),
        )
        .get_matches();

    if let Some(norm) = matches.subcommand_matches("norm") {
        let dry_run = norm.args.contains_key("dry-run");
        normalize::normalize(&norm.args["path"].vals[0], dry_run);
    }

    if let Some(transcode) = matches.subcommand_matches("transcode-aac") {
        let dry_run = transcode.args.contains_key("dry-run");
        prune::prune(
            &transcode.args["source_path"].vals[0].to_str().unwrap(),
            &transcode.args["dest_path"].vals[0].to_str().unwrap(),
            dry_run,
        );
        transcode::transcode(
            &transcode.args["source_path"].vals[0].to_str().unwrap(),
            &transcode.args["dest_path"].vals[0].to_str().unwrap(),
            dry_run,
            transcode::TranscodeFormat::Aac,
        )
    }

    if let Some(transcode) = matches.subcommand_matches("transcode-opus") {
        let dry_run = transcode.args.contains_key("dry-run");
        prune::prune(
            &transcode.args["source_path"].vals[0].to_str().unwrap(),
            &transcode.args["dest_path"].vals[0].to_str().unwrap(),
            dry_run,
        );
        transcode::transcode(
            &transcode.args["source_path"].vals[0].to_str().unwrap(),
            &transcode.args["dest_path"].vals[0].to_str().unwrap(),
            dry_run,
            transcode::TranscodeFormat::Opus,
        )
    }

    if let Some(transcode) = matches.subcommand_matches("transcode-mp3") {
        let dry_run = transcode.args.contains_key("dry-run");
        prune::prune(
            &transcode.args["source_path"].vals[0].to_str().unwrap(),
            &transcode.args["dest_path"].vals[0].to_str().unwrap(),
            dry_run,
        );
        transcode::transcode(
            &transcode.args["source_path"].vals[0].to_str().unwrap(),
            &transcode.args["dest_path"].vals[0].to_str().unwrap(),
            dry_run,
            transcode::TranscodeFormat::Mp3,
        )
    }

    if let Some(transcode) = matches.subcommand_matches("prune") {
        let dry_run = transcode.args.contains_key("dry-run");
        prune::prune(
            &transcode.args["source_path"].vals[0].to_str().unwrap(),
            &transcode.args["dest_path"].vals[0].to_str().unwrap(),
            dry_run,
        )
    }
}
