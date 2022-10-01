use clap::{App, Arg, SubCommand};

mod normalize;
mod tag;
mod transcode;

fn main() {
    let matches = App::new("frust")
        .version("1.0")
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
            SubCommand::with_name("transcode")
                .about("Transcode flac files.")
                .args(&[
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

    if let Some(transcode) = matches.subcommand_matches("transcode") {
        transcode::transcode(
            &transcode.args["source_path"].vals[0].to_str().unwrap(),
            &transcode.args["dest_path"].vals[0].to_str().unwrap(),
        )
    }
}
