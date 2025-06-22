use colored::*;
use globwalk::GlobWalkerBuilder;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use lofty::prelude::*;
use lofty::read_from_path;
use lofty::tag::{Accessor, ItemKey};
use rayon::prelude::*;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn configure_thread_pool() {
    // Configure rayon to use more threads for better parallelism
    // This is especially helpful for I/O bound operations
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .max(4); // Ensure minimum of 4 threads

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap_or_else(|_| {
            // ThreadPool already initialized, that's fine
        });
}

pub fn index_directory(directory: &str, dry_run: bool) {
    // Configure thread pool for optimal performance
    configure_thread_pool();
    let dir_path = Path::new(directory);

    if !dir_path.exists() {
        eprintln!(
            "{} Directory does not exist: {}",
            "✗".red().bold(),
            directory
        );
        return;
    }

    if !dir_path.is_dir() {
        eprintln!(
            "{} Path is not a directory: {}",
            "✗".red().bold(),
            directory
        );
        return;
    }

    println!(
        "{} {}",
        "🎵".bright_blue(),
        format!("Scanning directory: {}", directory)
            .bright_white()
            .bold()
    );

    let index_path = dir_path.join("index.json");

    // Load existing index if it exists
    let mut existing_tracks = load_existing_index(&index_path);
    println!(
        "{} Loaded existing index with {} tracks",
        "📚".bright_green(),
        existing_tracks.len().to_string().bright_yellow().bold()
    );

    // Find all current audio files in parallel
    let audio_extensions = [
        "*.mp3", "*.flac", "*.wav", "*.m4a", "*.aac", "*.ogg", "*.opus", "*.wv", "*.ape",
    ];

    // Create a progress bar for file discovery
    let discovery_pb = ProgressBar::new_spinner();
    discovery_pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    discovery_pb.set_message("🔍 Discovering audio files...");
    discovery_pb.enable_steady_tick(std::time::Duration::from_millis(80));

    // Parallelize file discovery across extensions
    let current_files: HashMap<String, (std::path::PathBuf, SystemTime)> = audio_extensions
        .par_iter()
        .map(|ext| {
            let walker = GlobWalkerBuilder::new(directory, ext)
                .follow_links(false)
                .build()
                .unwrap();

            let mut files = Vec::new();
            for entry in walker.flatten() {
                let file_path = entry.path();
                let relative_path = file_path
                    .strip_prefix(dir_path)
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                files.push((relative_path, file_path.to_path_buf()));
            }
            files
        })
        .flatten()
        .collect::<Vec<_>>()
        .par_iter()
        .filter_map(|(relative_path, file_path)| {
            // Parallelize metadata collection
            if let Ok(metadata) = fs::metadata(file_path) {
                if let Ok(modified) = metadata.modified() {
                    return Some((relative_path.clone(), (file_path.clone(), modified)));
                }
            }
            None
        })
        .collect();

    discovery_pb.finish_with_message(format!(
        "{} Found {} audio files",
        "✓".bright_green().bold(),
        current_files.len().to_string().bright_yellow().bold()
    ));

    if current_files.is_empty() {
        println!(
            "{} No audio files found in directory tree",
            "⚠️".bright_yellow()
        );
        if !existing_tracks.is_empty() {
            println!(
                "{} Clearing existing index since no audio files found",
                "🧹".bright_blue()
            );
            existing_tracks.clear();
        }
    } else {
        // Identify files to process
        let (files_to_process, files_to_remove) =
            identify_changes(&existing_tracks, &current_files);

        println!(
            "{} Files to process: {}",
            "⚡".bright_blue(),
            files_to_process.len().to_string().bright_yellow().bold()
        );
        println!(
            "{} Files to remove: {}",
            "🗑️".bright_red(),
            files_to_remove.len().to_string().bright_yellow().bold()
        );

        // Remove files that no longer exist
        for file_to_remove in &files_to_remove {
            existing_tracks.remove(file_to_remove);
            println!("{} Removed: {}", "🗑️".red(), file_to_remove.dimmed());
        }

        // Process only changed/new files
        if !files_to_process.is_empty() {
            // Create multi-progress for parallel processing
            let multi_progress = Arc::new(MultiProgress::new());
            let main_pb = multi_progress.add(ProgressBar::new(files_to_process.len() as u64));
            main_pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                    .unwrap()
                    .progress_chars("█▉▊▋▌▍▎▏  ")
            );
            main_pb.set_message("🎵 Processing audio files...");

            // Use Arc<Mutex<>> to reduce progress bar contention
            let pb_clone = Arc::new(Mutex::new(main_pb.clone()));
            let processed_count = Arc::new(Mutex::new(0usize));

            let new_tracks: HashMap<String, Value> = files_to_process
                .par_iter()
                .filter_map(|(relative_path, (file_path, _))| {
                    let result = match extract_metadata(file_path) {
                        Ok(metadata) => {
                            // Update progress less frequently to reduce contention
                            let mut count = processed_count.lock().unwrap();
                            *count += 1;
                            if *count % 10 == 0 || *count == files_to_process.len() {
                                if let Ok(pb) = pb_clone.lock() {
                                    pb.set_message(format!(
                                        "✓ {}",
                                        relative_path
                                            .split('/')
                                            .last()
                                            .unwrap_or(relative_path)
                                            .bright_green()
                                    ));
                                    pb.set_position(*count as u64);
                                }
                            }
                            Some((relative_path.clone(), metadata))
                        }
                        Err(e) => {
                            // Update progress for errors
                            let mut count = processed_count.lock().unwrap();
                            *count += 1;
                            if let Ok(pb) = pb_clone.lock() {
                                pb.set_message(format!(
                                    "✗ {} - {}",
                                    relative_path
                                        .split('/')
                                        .last()
                                        .unwrap_or(relative_path)
                                        .bright_red(),
                                    e.to_string().red()
                                ));
                                pb.set_position(*count as u64);
                            }
                            None
                        }
                    };
                    // Remove artificial delay that was limiting throughput
                    result
                })
                .collect();

            main_pb.finish_with_message(format!(
                "{} Processed {} files successfully",
                "🎉".bright_green(),
                new_tracks.len().to_string().bright_yellow().bold()
            ));

            // Update the existing tracks with new data
            for (path, metadata) in new_tracks {
                existing_tracks.insert(path, metadata);
            }
        }
    }

    // Create the index structure
    let index = json!({
        "version": 1,
        "tracks": existing_tracks
    });

    if dry_run {
        println!(
            "{} {} Would write index to: {}",
            "🔍".bright_blue(),
            "DRY RUN:".bright_yellow().bold(),
            index_path.display().to_string().bright_white()
        );
        println!(
            "{} Index would contain {} tracks",
            "📊".bright_blue(),
            existing_tracks.len().to_string().bright_yellow().bold()
        );
    } else {
        match serde_json::to_string_pretty(&index) {
            Ok(json_string) => match fs::write(&index_path, json_string) {
                Ok(_) => println!(
                    "{} Index written to: {} ({} tracks)",
                    "💾".bright_green(),
                    index_path.display().to_string().bright_white(),
                    existing_tracks.len().to_string().bright_yellow().bold()
                ),
                Err(e) => eprintln!("{} Error writing index file: {}", "✗".red().bold(), e),
            },
            Err(e) => eprintln!(
                "{} Error serializing index to JSON: {}",
                "✗".red().bold(),
                e
            ),
        }
    }
}

fn load_existing_index(index_path: &Path) -> HashMap<String, Value> {
    if !index_path.exists() {
        return HashMap::new();
    }

    match fs::read_to_string(index_path) {
        Ok(content) => match serde_json::from_str::<Value>(&content) {
            Ok(index) => {
                if let Some(tracks) = index.get("tracks") {
                    if let Some(tracks_obj) = tracks.as_object() {
                        return tracks_obj
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                    }
                }
            }
            Err(e) => {
                eprintln!("{} Error parsing existing index: {}", "✗".red().bold(), e);
            }
        },
        Err(e) => {
            eprintln!("{} Error reading existing index: {}", "✗".red().bold(), e);
        }
    }

    HashMap::new()
}

fn identify_changes(
    existing_tracks: &HashMap<String, Value>,
    current_files: &HashMap<String, (std::path::PathBuf, SystemTime)>,
) -> (
    HashMap<String, (std::path::PathBuf, SystemTime)>,
    HashSet<String>,
) {
    let mut files_to_process = HashMap::new();
    let mut files_to_remove = HashSet::new();

    // Find files that need to be removed (exist in index but not on disk)
    for existing_path in existing_tracks.keys() {
        if !current_files.contains_key(existing_path) {
            files_to_remove.insert(existing_path.clone());
        }
    }

    // Find files that need to be processed (new files or modified files)
    for (path, (file_path, current_mtime)) in current_files {
        let needs_processing = if let Some(existing_metadata) = existing_tracks.get(path) {
            // File exists in index, check if it's been modified
            if let Some(stored_mtime) = existing_metadata.get("mtime") {
                if let Some(stored_timestamp) = stored_mtime.as_u64() {
                    let current_timestamp = current_mtime
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    current_timestamp != stored_timestamp
                } else {
                    true // Invalid stored timestamp, reprocess
                }
            } else {
                true // No stored timestamp, reprocess
            }
        } else {
            true // New file
        };

        if needs_processing {
            files_to_process.insert(path.clone(), (file_path.clone(), *current_mtime));
        }
    }

    (files_to_process, files_to_remove)
}

fn extract_metadata(file_path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let tagged_file = read_from_path(file_path)?;
    let tag = tagged_file.primary_tag();
    let properties = tagged_file.properties();

    let mut metadata = Map::new();

    // Add required fields that match Track type

    // Get file metadata for mtime and size
    if let Ok(file_metadata) = fs::metadata(file_path) {
        if let Ok(modified) = file_metadata.modified() {
            let timestamp = modified
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            metadata.insert("mtime".to_string(), json!(timestamp));
        }
        metadata.insert("size".to_string(), json!(file_metadata.len()));
    }

    // Duration in seconds (matching Track.duration field)
    if properties.duration().as_secs() > 0 {
        metadata.insert(
            "duration".to_string(),
            json!(properties.duration().as_secs()),
        );
    }

    // Bitrate
    if let Some(bitrate) = properties.overall_bitrate() {
        metadata.insert("bitrate".to_string(), json!(bitrate));
    }

    // Tag metadata - only add fields that have values
    if let Some(tag) = tag {
        // Required fields (album, artist) - add empty strings if missing
        let album = tag.album().map(|s| s.to_string()).unwrap_or_default();
        let artist = tag.artist().map(|s| s.to_string()).unwrap_or_default();

        metadata.insert("album".to_string(), json!(album));
        metadata.insert("artist".to_string(), json!(artist));

        // Optional fields - only add if present
        if let Some(title) = tag.title() {
            metadata.insert("title".to_string(), json!(title.to_string()));
        }

        if let Some(album_artist) = tag.get_string(&ItemKey::AlbumArtist) {
            metadata.insert("albumArtist".to_string(), json!(album_artist));
        }

        if let Some(composer) = tag.get_string(&ItemKey::Composer) {
            metadata.insert("composer".to_string(), json!(composer));
        }

        if let Some(genre) = tag.genre() {
            metadata.insert("genre".to_string(), json!(genre.to_string()));
        }

        if let Some(year) = tag.year() {
            metadata.insert("year".to_string(), json!(year));
        }

        if let Some(track) = tag.track() {
            metadata.insert("trackNumber".to_string(), json!(track));
        }

        if let Some(track_total) = tag.track_total() {
            metadata.insert("trackNumberTotal".to_string(), json!(track_total));
        }

        if let Some(disc) = tag.disk() {
            metadata.insert("diskNumber".to_string(), json!(disc));
        }

        if let Some(disc_total) = tag.disk_total() {
            metadata.insert("diskNumberTotal".to_string(), json!(disc_total));
        }

        // Additional fields that might be available - using ItemKey for fields not in standard API
        if let Some(performer) = tag.get_string(&ItemKey::Performer) {
            metadata.insert("performer".to_string(), json!(performer));
        }

        if let Some(work) = tag.get_string(&ItemKey::Work) {
            metadata.insert("work".to_string(), json!(work));
        }

        if let Some(grouping) = tag.get_string(&ItemKey::ContentGroup) {
            metadata.insert("grouping".to_string(), json!(grouping));
        }

        if let Some(label) = tag.get_string(&ItemKey::Publisher) {
            metadata.insert("label".to_string(), json!(label));
        }

        // Rating would typically need to be stored/managed by the application
        // as it's not commonly stored in audio file tags
    }

    Ok(Value::Object(metadata))
}
