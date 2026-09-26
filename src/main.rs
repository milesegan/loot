use clap::{error::ErrorKind, Args, CommandFactory, Parser, Subcommand, ValueEnum};
use transcode::{AacBitrateMode, TranscodeFormat, DEFAULT_AAC_VBR_QUALITY};

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
    /// Transcode audio files to FLAC format
    TranscodeFlac(TranscodeArgs),
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

const DEFAULT_AAC_CBR_BITRATE_KBPS: u32 = 128;

#[derive(Args)]
struct TranscodeAacArgs {
    #[command(flatten)]
    shared: TranscodeArgs,
    /// CBR bitrate in kbps, 64-320 (cbr mode only)
    #[arg(short, long, value_name = "KBPS", value_parser = clap::value_parser!(u32).range(64..=320))]
    bitrate: Option<u32>,
    /// True VBR quality, 0-127 (vbr mode only)
    #[arg(short, long, value_name = "0-127", value_parser = clap::value_parser!(u8).range(0..=127))]
    quality: Option<u8>,
    #[arg(long, value_enum, default_value = "vbr")]
    mode: AacCliBitrateMode,
}

impl TranscodeAacArgs {
    fn bitrate_mode(&self) -> Result<AacBitrateMode, &'static str> {
        match self.mode {
            AacCliBitrateMode::Vbr if self.bitrate.is_some() => {
                Err("--bitrate only applies to --mode cbr; use --quality for vbr")
            }
            AacCliBitrateMode::Cbr if self.quality.is_some() => {
                Err("--quality only applies to --mode vbr")
            }
            AacCliBitrateMode::Vbr => Ok(AacBitrateMode::Vbr {
                quality: self.quality.unwrap_or(DEFAULT_AAC_VBR_QUALITY),
            }),
            AacCliBitrateMode::Cbr => Ok(AacBitrateMode::Cbr {
                bitrate_kbps: self.bitrate.unwrap_or(DEFAULT_AAC_CBR_BITRATE_KBPS),
            }),
        }
    }
}

#[derive(Args)]
struct TranscodeOpusArgs {
    #[command(flatten)]
    shared: TranscodeArgs,
    #[arg(short, long, value_name = "KBPS")]
    bitrate: Option<u32>,
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
        Commands::TranscodeAac(args) => match args.bitrate_mode() {
            Ok(mode) => transcode(
                &args.shared,
                TranscodeFormat::Aac {
                    mode,
                    embed_cover: false,
                },
            ),
            Err(message) => {
                let mut command = Cli::command();
                command.build();
                command
                    .find_subcommand_mut("transcode-aac")
                    .expect("transcode-aac subcommand")
                    .error(ErrorKind::ArgumentConflict, message)
                    .exit()
            }
        },
        Commands::TranscodeAacCBR(args) => {
            transcode(
                args,
                TranscodeFormat::Aac {
                    mode: AacBitrateMode::Cbr { bitrate_kbps: 256 },
                    embed_cover: true,
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
        Commands::TranscodeFlac(args) => {
            transcode(args, TranscodeFormat::Flac);
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
                assert_eq!(
                    args.bitrate_mode(),
                    Ok(AacBitrateMode::Cbr { bitrate_kbps: 192 })
                );
                assert_eq!(args.shared.paths, vec!["src".to_owned(), "dest".to_owned()]);
            }
            _ => panic!("expected transcode-aac command"),
        }
    }

    #[test]
    fn transcode_aac_defaults_to_vbr_quality_63() {
        let cli = Cli::try_parse_from(["loot", "transcode-aac", "src", "dest"])
            .expect("expected transcode-aac args to parse");

        match cli.command {
            Commands::TranscodeAac(args) => {
                assert_eq!(args.bitrate_mode(), Ok(AacBitrateMode::Vbr { quality: 63 }));
            }
            _ => panic!("expected transcode-aac command"),
        }
    }

    #[test]
    fn transcode_aac_cbr_defaults_to_128() {
        let cli = Cli::try_parse_from(["loot", "transcode-aac", "--mode", "cbr", "src", "dest"])
            .expect("expected transcode-aac args to parse");

        match cli.command {
            Commands::TranscodeAac(args) => {
                assert_eq!(
                    args.bitrate_mode(),
                    Ok(AacBitrateMode::Cbr { bitrate_kbps: 128 })
                );
            }
            _ => panic!("expected transcode-aac command"),
        }
    }

    #[test]
    fn transcode_aac_rejects_mismatched_rate_flags() {
        let vbr = Cli::try_parse_from(["loot", "transcode-aac", "-b", "192", "src", "dest"])
            .expect("expected transcode-aac args to parse");
        let cbr = Cli::try_parse_from([
            "loot",
            "transcode-aac",
            "--mode",
            "cbr",
            "-q",
            "91",
            "src",
            "dest",
        ])
        .expect("expected transcode-aac args to parse");

        for cli in [vbr, cbr] {
            match cli.command {
                Commands::TranscodeAac(args) => assert!(args.bitrate_mode().is_err()),
                _ => panic!("expected transcode-aac command"),
            }
        }
    }

    #[test]
    fn transcode_aac_rejects_bitrate_out_of_range() {
        for bitrate in ["32", "321"] {
            assert!(Cli::try_parse_from([
                "loot",
                "transcode-aac",
                "--mode",
                "cbr",
                "-b",
                bitrate,
                "src",
                "dest",
            ])
            .is_err());
        }
    }

    #[test]
    fn transcode_aac_rejects_quality_out_of_range() {
        assert!(
            Cli::try_parse_from(["loot", "transcode-aac", "-q", "128", "src", "dest"]).is_err()
        );
    }

    #[test]
    fn transcode_opus_defaults_to_encoder_bitrate() {
        let cli = Cli::try_parse_from(["loot", "transcode-opus", "src", "dest"])
            .expect("expected transcode-opus args to parse");

        match cli.command {
            Commands::TranscodeOpus(args) => {
                assert_eq!(args.bitrate, None);
                assert_eq!(args.shared.paths, vec!["src".to_owned(), "dest".to_owned()]);
            }
            _ => panic!("expected transcode-opus command"),
        }
    }

    #[test]
    fn transcode_opus_accepts_bitrate() {
        let cli =
            Cli::try_parse_from(["loot", "transcode-opus", "--bitrate", "192", "src", "dest"])
                .expect("expected transcode-opus args to parse");

        match cli.command {
            Commands::TranscodeOpus(args) => {
                assert_eq!(args.bitrate, Some(192));
                assert_eq!(args.shared.paths, vec!["src".to_owned(), "dest".to_owned()]);
            }
            _ => panic!("expected transcode-opus command"),
        }
    }

    #[test]
    fn transcode_flac_accepts_shared_args() {
        let cli = Cli::try_parse_from(["loot", "transcode-flac", "--dry-run", "src", "dest"])
            .expect("expected transcode-flac args to parse");

        match cli.command {
            Commands::TranscodeFlac(args) => {
                assert!(args.dry_run);
                assert_eq!(args.paths, vec!["src".to_owned(), "dest".to_owned()]);
            }
            _ => panic!("expected transcode-flac command"),
        }
    }
}
