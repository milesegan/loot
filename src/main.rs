use clap::{Args, Parser, Subcommand, ValueEnum};
use transcode::{AacBitrateMode, TranscodeFormat};

mod cli;
mod error;
mod fs_utils;
mod index;
mod normalize;
mod prune;
mod tag;
mod text;
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
    /// Normalize file and directory names
    Norm(NormArgs),
    /// Remove duplicate files from destination directory
    Prune(PruneArgs),
    /// Create a JSON index of audio files with metadata
    Index(IndexArgs),
    /// Transcode audio files to AAC format
    TranscodeAac(TranscodeAacArgs),
    /// Transcode audio files to AAC format at 256kbps
    TranscodeAacCBR(TranscodeArgs),
    /// Transcode audio files to MP3 format
    TranscodeMp3(TranscodeArgs),
    /// Transcode audio files to Opus format
    TranscodeOpus(TranscodeOpusArgs),
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum AacCliBitrateMode {
    Vbr,
    Cbr,
}

impl From<AacCliBitrateMode> for AacBitrateMode {
    fn from(mode: AacCliBitrateMode) -> Self {
        match mode {
            AacCliBitrateMode::Vbr => AacBitrateMode::Vbr,
            AacCliBitrateMode::Cbr => AacBitrateMode::Cbr,
        }
    }
}

#[derive(Args)]
struct TranscodeAacArgs {
    #[command(flatten)]
    shared: TranscodeArgs,
    #[arg(short, long, default_value_t = 128, value_name = "KBPS")]
    bitrate: u32,
    #[arg(long, value_enum, default_value = "vbr")]
    mode: AacCliBitrateMode,
}

#[derive(Args)]
struct TranscodeOpusArgs {
    #[command(flatten)]
    shared: TranscodeArgs,
    #[arg(short, long, default_value_t = 128, value_name = "KBPS")]
    bitrate: u32,
}

#[derive(Args)]
struct IndexArgs {
    #[arg(short, long)]
    dry_run: bool,
    #[arg(short, long)]
    force: bool,
    path: String,
}

fn run_prune(paths: &[String], dry_run: bool) {
    if let Some((sources, dest)) = cli::split_sources_and_dest(paths) {
        prune::prune(sources, dest, dry_run);
    } else {
        eprintln!("At least two paths required.");
    }
}

fn transcode(args: &TranscodeArgs, format: TranscodeFormat) {
    if let Some((sources, dest)) = cli::split_sources_and_dest(&args.paths) {
        prune::prune(sources, dest, args.dry_run);
        transcode::transcode(sources, dest, args.dry_run, format)
    } else {
        eprintln!("At least two paths required.");
    }
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Norm(args) => {
            normalize::normalize(&args.path, args.dry_run);
        }
        Commands::Prune(args) => {
            run_prune(&args.paths, args.dry_run);
        }
        Commands::Index(args) => {
            index::index_directory(&args.path, args.dry_run, args.force);
        }
        Commands::TranscodeAac(args) => {
            transcode(
                &args.shared,
                TranscodeFormat::Aac {
                    mode: args.mode.into(),
                    bitrate_kbps: args.bitrate,
                },
            );
        }
        Commands::TranscodeAacCBR(args) => {
            transcode(
                args,
                TranscodeFormat::Aac {
                    mode: AacBitrateMode::Cbr,
                    bitrate_kbps: 256,
                },
            );
        }
        Commands::TranscodeMp3(args) => {
            transcode(args, TranscodeFormat::Mp3);
        }
        Commands::TranscodeOpus(args) => {
            transcode(
                &args.shared,
                TranscodeFormat::Opus {
                    bitrate_kbps: args.bitrate,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcode_aac_accepts_bitrate_and_mode() {
        let cli = Cli::try_parse_from([
            "loot",
            "transcode-aac",
            "--bitrate",
            "192",
            "--mode",
            "cbr",
            "src",
            "dest",
        ])
        .expect("expected transcode-aac args to parse");

        match cli.command {
            Commands::TranscodeAac(args) => {
                assert_eq!(args.bitrate, 192);
                assert_eq!(args.mode, AacCliBitrateMode::Cbr);
                assert_eq!(args.shared.paths, vec!["src".to_owned(), "dest".to_owned()]);
            }
            _ => panic!("expected transcode-aac command"),
        }
    }

    #[test]
    fn transcode_aac_defaults_to_vbr_128() {
        let cli = Cli::try_parse_from(["loot", "transcode-aac", "src", "dest"])
            .expect("expected transcode-aac args to parse");

        match cli.command {
            Commands::TranscodeAac(args) => {
                assert_eq!(args.bitrate, 128);
                assert_eq!(args.mode, AacCliBitrateMode::Vbr);
            }
            _ => panic!("expected transcode-aac command"),
        }
    }
}
