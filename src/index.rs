use globwalk::GlobWalkerBuilder;
use lofty::prelude::*;
use lofty::read_from_path;
use rayon::prelude::*;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn index_directory(directory: &str, dry_run: bool) {
    let dir_path = Path::new(directory);

    if !dir_path.exists() {
        eprintln!("Directory does not exist: {}", directory);
        return;
    }

    if !dir_path.is_dir() {
        eprintln!("Path is not a directory: {}", directory);
        return;
    }

    println!("Scanning directory: {}", directory);

    let index_path = dir_path.join("index.json");

    // Load existing index if it exists
    let mut existing_tracks = load_existing_index(&index_path);
    println!(
        "Loaded existing index with {} tracks",
        existing_tracks.len()
    );

    // Find all current audio files
    let audio_extensions = [
        "*.mp3", "*.flac", "*.wav", "*.m4a", "*.aac", "*.ogg", "*.opus", "*.wv", "*.ape",
    ];
    let mut current_files = HashMap::new();

    for ext in &audio_extensions {
        let walker = GlobWalkerBuilder::new(directory, ext)
            .follow_links(false)
            .build()
            .unwrap();

        for entry in walker.flatten() {
            let file_path = entry.path();
            let relative_path = file_path
                .strip_prefix(dir_path)
                .unwrap()
                .to_string_lossy()
                .to_string();

            if let Ok(metadata) = fs::metadata(file_path) {
                if let Ok(modified) = metadata.modified() {
                    current_files.insert(relative_path, (file_path.to_path_buf(), modified));
                }
            }
        }
    }

    println!("Found {} audio files", current_files.len());

    if current_files.is_empty() {
        println!("No audio files found in directory tree");
        if !existing_tracks.is_empty() {
            println!("Clearing existing index since no audio files found");
            existing_tracks.clear();
        }
    } else {
        // Identify files to process
        let (files_to_process, files_to_remove) =
            identify_changes(&existing_tracks, &current_files);

        println!("Files to process: {}", files_to_process.len());
        println!("Files to remove: {}", files_to_remove.len());

        // Remove files that no longer exist
        for file_to_remove in &files_to_remove {
            existing_tracks.remove(file_to_remove);
            println!("Removed: {}", file_to_remove);
        }

        // Process only changed/new files
        if !files_to_process.is_empty() {
            let new_tracks: HashMap<String, Value> = files_to_process
                .par_iter()
                .filter_map(
                    |(relative_path, (file_path, _))| match extract_metadata(file_path) {
                        Ok(metadata) => {
                            println!("Processed: {}", relative_path);
                            Some((relative_path.clone(), metadata))
                        }
                        Err(e) => {
                            eprintln!("Error processing {}: {}", relative_path, e);
                            None
                        }
                    },
                )
                .collect();

            // Update the existing tracks with new data
            for (path, metadata) in new_tracks {
                existing_tracks.insert(path, metadata);
            }
        }
    }

    // Create the index structure
    let index = json!({
        "tracks": existing_tracks
    });

    if dry_run {
        println!("Dry run: Would write index to: {}", index_path.display());
        println!("Index would contain {} tracks", existing_tracks.len());
    } else {
        match serde_json::to_string_pretty(&index) {
            Ok(json_string) => match fs::write(&index_path, json_string) {
                Ok(_) => println!("Index written to: {}", index_path.display()),
                Err(e) => eprintln!("Error writing index file: {}", e),
            },
            Err(e) => eprintln!("Error serializing index to JSON: {}", e),
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
                eprintln!("Error parsing existing index: {}", e);
            }
        },
        Err(e) => {
            eprintln!("Error reading existing index: {}", e);
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
            if let Some(stored_mtime) = existing_metadata.get("modified_time") {
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

    // Add modification time
    if let Ok(file_metadata) = fs::metadata(file_path) {
        if let Ok(modified) = file_metadata.modified() {
            let timestamp = modified
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            metadata.insert("modified_time".to_string(), json!(timestamp));
        }
    }

    // Basic file info
    metadata.insert(
        "file_type".to_string(),
        json!(format!("{:?}", tagged_file.file_type())),
    );

    // Duration and bitrate
    metadata.insert(
        "duration_seconds".to_string(),
        json!(properties.duration().as_secs()),
    );
    metadata.insert(
        "duration_ms".to_string(),
        json!(properties.duration().as_millis()),
    );

    if let Some(bitrate) = properties.overall_bitrate() {
        metadata.insert("bitrate".to_string(), json!(bitrate));
    }

    if let Some(sample_rate) = properties.sample_rate() {
        metadata.insert("sample_rate".to_string(), json!(sample_rate));
    }

    if let Some(bit_depth) = properties.bit_depth() {
        metadata.insert("bit_depth".to_string(), json!(bit_depth));
    }

    if let Some(channels) = properties.channels() {
        metadata.insert("channels".to_string(), json!(channels));
    }

    // Tag metadata
    if let Some(tag) = tag {
        if let Some(title) = tag.title() {
            metadata.insert("title".to_string(), json!(title.to_string()));
        }

        if let Some(artist) = tag.artist() {
            metadata.insert("artist".to_string(), json!(artist.to_string()));
        }

        if let Some(album) = tag.album() {
            metadata.insert("album".to_string(), json!(album.to_string()));
        }

        if let Some(date) = tag.year() {
            metadata.insert("year".to_string(), json!(date));
        }

        if let Some(track) = tag.track() {
            metadata.insert("track_number".to_string(), json!(track));
        }

        if let Some(track_total) = tag.track_total() {
            metadata.insert("track_total".to_string(), json!(track_total));
        }

        if let Some(disc) = tag.disk() {
            metadata.insert("disc_number".to_string(), json!(disc));
        }

        if let Some(disc_total) = tag.disk_total() {
            metadata.insert("disc_total".to_string(), json!(disc_total));
        }

        if let Some(genre) = tag.genre() {
            metadata.insert("genre".to_string(), json!(genre.to_string()));
        }

        if let Some(comment) = tag.comment() {
            metadata.insert("comment".to_string(), json!(comment.to_string()));
        }
    }

    Ok(Value::Object(metadata))
}
