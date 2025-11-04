# Sonica Backend - Music Recognition API

A fast and lightweight music recognition backend built in Rust with MFCC fingerprinting.

## Features

- ✅ Fast music recognition using MFCC fingerprinting
- ✅ Automatic audio preprocessing (FFmpeg)
- ✅ In-memory fingerprint caching for instant recognition
- ✅ Auto-detection of new songs via file watcher
- ✅ REST API with Axum
- ✅ SQLite database for persistence

## Prerequisites

- Rust (stable)
- FFmpeg installed and available in PATH

### Installing FFmpeg

**Windows:**
```powershell
choco install ffmpeg
# or download from https://ffmpeg.org/download.html
```

**macOS:**
```bash
brew install ffmpeg
```

**Linux:**
```bash
sudo apt-get update && sudo apt-get install ffmpeg
```

## Setup

1. Clone the repository and navigate to the backend:
```bash
cd backend
```

2. Build the project:
```bash
cargo build --release
```

3. Create the songs directory (if it doesn't exist):
```bash
mkdir songs
```

4. Place audio files (MP3, WAV, FLAC, etc.) in the `songs/` directory

5. Run the server:
```bash
cargo run --release
```

The server will start on `http://localhost:8000`

## API Endpoints

### Health Check
```bash
GET /health
```

### List All Songs
```bash
GET /songs
```

### Recognize Audio Clip
```bash
POST /recognize
Content-Type: multipart/form-data

# Form field: "audio" (audio file)
```

**Response:**
```json
{
  "match": {
    "title": "Song Title",
    "artist": "Artist Name",
    "score": 0.94
  }
}
```

### Upload Song
```bash
POST /upload
Content-Type: multipart/form-data

# Form fields:
# - "file" (audio file)
# - "title" (optional)
# - "artist" (optional)
```

## How It Works

1. **Startup**: Server loads all existing songs from the database and preprocesses any new files in the `songs/` directory
2. **Fingerprinting**: Each song is processed to extract MFCC (Mel-frequency Cepstral Coefficients) features
3. **Caching**: All fingerprints are loaded into memory for fast recognition
4. **File Watcher**: Automatically detects new files in `songs/` and processes them
5. **Recognition**: Audio clips are fingerprinted and matched against the cache using cosine similarity

## Performance

- Recognition: < 0.5 seconds
- New song preprocessing: 1-2 seconds
- Memory usage: < 50 MB for 1000 songs

## Supported Audio Formats

- MP3
- WAV
- FLAC
- M4A
- OGG
- AAC
- Opus

## Deployment

For Render.com deployment:

1. Set build command:
```
cargo build --release
```

2. Set start command:
```
./target/release/sonica-backend
```

3. Install FFmpeg in build:
```
apt-get update && apt-get install -y ffmpeg
```

4. Expose port 8000

## License

MIT

