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
    Prune(PruneArgs),
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
struct PruneArgs {
    #[arg(short, long)]
    dry_run: bool,
    paths: Vec<String>,
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
        Commands::Prune(args) => {
            if args.paths.len() < 2 {
                println!("At least two paths required.")
            } else {
                if let Some((dest, sources)) = args.paths.split_last() {
                    prune::prune(sources, dest, args.dry_run);
                }
            }
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
}
