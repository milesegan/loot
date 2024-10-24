use clap::{Args, Parser, Subcommand};
use transcode::TranscodeFormat;

mod error;
mod normalize;
mod prune;
mod tag;
mod transcode;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Adds files to myapp
    Norm(NormArgs),
    TranscodeAac(TranscodeArgs),
    TranscodeMp3(TranscodeArgs),
    TranscodeOpus(TranscodeArgs),
}

#[derive(Args)]
struct NormArgs {
    #[arg(short, long)]
    dry_run: bool,
    path: String,
}

#[derive(Args)]
struct TranscodeArgs {
    #[arg(short, long)]
    dry_run: bool,
    paths: Vec<String>,
}

fn transcode(args: &TranscodeArgs, format: TranscodeFormat) {
    if args.paths.len() < 2 {
        println!("At least two paths required.")
    } else {
        if let Some((dest, sources)) = args.paths.split_last() {
            prune::prune(sources, dest, args.dry_run);
            transcode::transcode(sources, dest, args.dry_run, format)
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Norm(args) => {
            normalize::normalize(&args.path, args.dry_run);
        }
        Commands::TranscodeAac(args) => {
            transcode(args, TranscodeFormat::Aac);
        }
        Commands::TranscodeMp3(args) => {
            transcode(args, TranscodeFormat::Mp3);
        }
        Commands::TranscodeOpus(args) => {
            transcode(args, TranscodeFormat::Opus);
        }
    }

    /*
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
    */
}
