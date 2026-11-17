---
title: Sonica Backend
emoji: 🎵
colorFrom: blue
colorTo: purple
sdk: docker
pinned: false
---

# Sonica Backend


Rust backend for local music recognition. It preprocesses audio with FFmpeg, derives STFT peak-pair fingerprints, and matches them against an indexed SQLite database.

## Features

- **Peak-Pair Fingerprinting**: Uses an STFT spectrogram, adaptive per-band peaks, and 32-bit peak-pair hashes
- **Indexed Matching**: Looks up hashes in SQLite and scores candidates with a histogram of time offsets
- **Auto-Detection**: Automatically indexes new songs added to `songs/` directory
- **SQLite Storage**: Stores song metadata and fingerprints locally; WAL mode supports batch insertion
- **Multiple Formats**: Supports MP3, WAV, FLAC, M4A, AAC, OGG, OPUS, WMA

## Prerequisites

- Rust (stable)
- FFmpeg installed and in PATH

### Installing FFmpeg

**Windows:**
```bash
choco install ffmpeg
# or download from https://ffmpeg.org/download.html
```

**macOS:**
```bash
brew install ffmpeg
```

**Linux:**
```bash
sudo apt-get install ffmpeg
```

## Setup

1. Clone the repository
2. Install dependencies:
```bash
cd backend
cargo build --release
```

3. Create `songs/` directory (or it will be created automatically):
```bash
mkdir songs
```

4. Place audio files in `songs/` directory

## Running

```bash
cargo run --release
```

Server will start on `http://0.0.0.0:8000`

## API Endpoints

### `GET /health`
Health check endpoint.

**Response:**
```json
{"status": "ok"}
```

### `GET /songs`
Get list of all processed songs.

**Response:**
```json
[
  {
    "id": 1,
    "title": "Song Title",
    "artist": "Artist Name",
    "path": "songs/song.mp3",
    "created_at": "2025-01-01 12:00:00"
  }
]
```

### `POST /recognize`
Recognize an audio clip.

**Request:** Multipart form with field `audio` or `file` containing audio file

**Response:**
```json
{
  "match": {
    "title": "Blinding Lights",
    "artist": "The Weeknd",
    "score": 0.94
  }
}
```

If no match found:
```json
{
  "match": null
}
```

### `POST /upload`
Upload a new song file.

**Request:** Multipart form with:
- `audio` or `file`: Audio file
- `title` (optional): Song title
- `artist` (optional): Artist name

**Response:**
```json
{
  "message": "Song uploaded and processing started",
  "path": "songs/song.mp3",
  "title": "Song Title",
  "artist": "Artist Name"
}
```

## How It Works

1. **Startup**: Server opens the local SQLite database and watches the `songs/` directory
2. **Preprocessing**: New songs are converted to mono 16kHz WAV using FFmpeg
3. **Fingerprinting**: An STFT produces a spectrogram, from which the backend derives peak-pair hashes
4. **Storage**: Each hash and its time offset are stored in SQLite with an index on the hash value
5. **Recognition**: Query hashes retrieve candidates from SQLite; a histogram of relative offsets selects the best match
6. **Auto-Detection**: File watcher automatically processes new files added to `songs/` directory

## Measured Configuration

The accompanying paper reports measurements for a 50-song local catalog. Runtime depends on the catalog, audio content, hardware, and query length; this repository does not define fixed performance guarantees.

## Deployment

For Render deployment:

1. Build command: `cargo build --release`
2. Start command: `./target/release/sonica-backend`
3. Install FFmpeg: `apt-get update && apt-get install -y ffmpeg`
4. Expose port: 8000

## Notes

- Songs are stored locally in `songs/` directory
- Database (`songs.db`) stores metadata and fingerprints
- Temporary processing files are created in `temp/` and cleaned up automatically
- Google Drive integration planned for Phase 2
