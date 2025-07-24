# Loot

This is a simple tool to help manage libraries of music files. It needs a working ffmpeg installation to do
most of its work.

## Subcommands

### norm

Normalize file and directory names for audio files in a directory tree.

**Usage:**

```
loot norm [--dry-run] <path>
```

- `--dry-run`, `-d`: Show what would be changed, but do not modify files.
- `<path>`: Root directory to process.

**Example:**

```
loot norm ~/Music/Library
```

---

### prune

Remove duplicate files from a destination directory if the original exists in one of the source directories (by matching relative path, but with `.flac` extension in the source).

**Usage:**

```
loot prune [--dry-run] <source1> <source2> ... <destination>
```

- `--dry-run`, `-d`: Show what would be deleted, but do not remove files.
- `<source1> <source2> ...`: One or more source directories.
- `<destination>`: Destination directory to prune.

**Example:**

```
loot prune ~/Music/Originals ~/Music/Other ~/Music/Compressed
```

---

### index

Create a JSON index of audio files with metadata in a directory.

**Usage:**

```
loot index [--dry-run] [--force] <path>
```

- `--dry-run`, `-d`: Show what would be indexed, but do not write the index file.
- `--force`, `-f`: Rebuild the index from scratch, ignoring any existing index.
- `<path>`: Directory to index.

**Example:**

```
loot index ~/Music/Library
```

---

### transcode-aac

Transcode audio files to AAC format (`.m4a`).

**Usage:**

```
loot transcode-aac [--dry-run] <source1> <source2> ... <destination>
```

- `--dry-run`, `-d`: Show what would be transcoded, but do not write files.
- `<source1> <source2> ...`: One or more source directories (must be at least one).
- `<destination>`: Destination directory for transcoded files.

**Example:**

```
loot transcode-aac ~/Music/Originals ~/Music/AAC
```

---

### transcode-mp3

Transcode audio files to MP3 format.

**Usage:**

```
loot transcode-mp3 [--dry-run] <source1> <source2> ... <destination>
```

- `--dry-run`, `-d`: Show what would be transcoded, but do not write files.
- `<source1> <source2> ...`: One or more source directories (must be at least one).
- `<destination>`: Destination directory for transcoded files.

**Example:**

```
loot transcode-mp3 ~/Music/Originals ~/Music/MP3
```

---

### transcode-opus

Transcode audio files to Opus format.

**Usage:**

```
loot transcode-opus [--dry-run] <source1> <source2> ... <destination>
```

- `--dry-run`, `-d`: Show what would be transcoded, but do not write files.
- `<source1> <source2> ...`: One or more source directories (must be at least one).
- `<destination>`: Destination directory for transcoded files.

**Example:**

```
loot transcode-opus ~/Music/Originals ~/Music/Opus
```
