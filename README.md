# Sonica 🎵

Sonica is an open-source, high-performance music recognition system inspired by Shazam. It identifies songs in real-time by analyzing audio fingerprints using a robust backend written in Rust and a modern, responsive frontend built with React.

## 🚀 Live Demo

-   **Frontend:** [https://sonicamusic.netlify.app](https://sonicamusic.netlify.app)
-   **Backend API:** [https://huggingface.co/spaces/vishal4743/sonica-backend](https://huggingface.co/spaces/vishal4743/sonica-backend)

## ✨ Features

-   **Real-time Recognition**: Identifies songs from microphone input or system audio in seconds with continuous streaming support.
-   **Advanced Fingerprinting**: Uses STFT (Short-Time Fourier Transform) with peak detection and combinatorial hashing to create unique audio signatures.
-   **Robust Matching**: Implements the "Histogram of Offsets" algorithm for high-accuracy matching, resilient to noise and timing variations.
-   **Automatic File Watching**: Detects and processes new songs automatically when added to the `songs/` directory.
-   **Metadata Extraction**: Automatically extracts Title and Artist information from audio files using ID3 tags (via `lofty` crate).
-   **WebSocket Streaming**: Low-latency real-time audio streaming from client to server for continuous recognition.
-   **Modern UI**: Beautiful, dark-themed interface with audio source toggle (microphone/system audio) and real-time animations using Framer Motion and TailwindCSS.
-   **DoS Protection**: Built-in concurrency limiting (50 concurrent requests) and file size validation.

## 🛠️ Tech Stack

### Backend (Rust)
-   **Framework**: Axum 0.7 (High-performance web framework with WebSocket support)
-   **Audio Processing**: 
    -   Symphonia 0.5 (Audio decoding for multiple formats)
    -   RustFFT 6.2 (Fast Fourier Transform for frequency analysis)
    -   FFmpeg (Audio preprocessing: resampling to 16kHz mono)
-   **Database**: SQLite (via Rusqlite 0.31) with indexed hash lookups
-   **Metadata Extraction**: Lofty 0.22.4 (ID3 tag parsing)
-   **File Watching**: Notify 6.1 (Automatic detection of new songs)
-   **Concurrency**: 
    -   Tokio 1.x (Async runtime)
    -   Rayon 1.8 (Parallel song processing)
-   **Middleware**: Tower-HTTP (CORS, Tracing, Concurrency limiting)
-   **Deployment**: Docker on Hugging Face Spaces

### Frontend (React + TypeScript)
-   **Build Tool**: Vite 5.4
-   **UI Framework**: React 18.3
-   **Styling**: TailwindCSS 3.4
-   **State Management**: React Hooks (Custom hooks for audio recording and WebSocket)
-   **Animations**: Framer Motion 11.11
-   **Icons**: Lucide React 0.454
-   **Audio Capture**: MediaRecorder API (supports microphone and system audio)
-   **Real-time Communication**: WebSocket API
-   **Deployment**: Netlify

### Algorithm Details
-   **Window Size**: 4096 samples (better frequency resolution)
-   **Hop Size**: 2048 samples (50% overlap)
-   **Sample Rate**: 16kHz mono
-   **Fingerprint Format**: 32-bit hash combining frequency pairs and time delta
-   **Hash Structure**: `[freq1: 9 bits][freq2: 9 bits][time_delta: 14 bits]`
-   **Matching**: Histogram of time offsets with threshold-based alignment
-   **Match Threshold**: 8 aligned fingerprints minimum
-   **Supported Formats**: MP3, WAV, FLAC, M4A, AAC, OGG, OPUS, WMA

## 📦 Installation & Setup

### Prerequisites
-   [Rust & Cargo](https://www.rust-lang.org/tools/install) (latest stable)
-   [Node.js & npm](https://nodejs.org/) (v18 or higher recommended)
-   [FFmpeg](https://ffmpeg.org/) (Required for audio preprocessing)
-   [Git](https://git-scm.com/) (for cloning the repository)

#### Installing FFmpeg

**Windows:**
```bash
choco install ffmpeg
# or download from https://ffmpeg.org/download.html
```

**macOS:**
```bash
brew install ffmpeg
```

**Linux (Ubuntu/Debian):**
```bash
sudo apt-get update
sudo apt-get install ffmpeg
```

### 1. Clone the Repository
```bash
git clone https://github.com/Vishal4742/Sonica_1.git
cd Sonica_1
```

### 2. Backend Setup
```bash
cd backend

# Create directories for songs and temporary files
mkdir -p songs temp

# Build and run the server
cargo run --release
```

The server will:
- Start on `http://0.0.0.0:8000` (or port specified by `PORT` environment variable)
- Create a `songs.db` SQLite database
- Automatically scan and index any songs in the `songs/` directory
- Watch for new files added to `songs/` and process them automatically

**Adding Songs:**
- Place audio files (MP3, WAV, FLAC, M4A, AAC, OGG, OPUS, WMA) in the `backend/songs/` directory
- The file watcher will automatically detect and process new files
- Metadata is extracted from ID3 tags if available, otherwise filename is used
- Each song is preprocessed to 16kHz mono WAV and fingerprinted
- Fingerprints are stored in the SQLite database for fast matching

### 3. Frontend Setup
```bash
cd ../frontend

# Install dependencies
npm install

# Create .env file with backend URL
echo "VITE_API_URL=http://localhost:8000" > .env

# Run development server
npm run dev
```

The app will run at `http://localhost:5173`.

**For Production:**
```bash
npm run build
# Output will be in frontend/dist/
```

## 🚀 Deployment

### Backend (Hugging Face Spaces)

The backend is deployed using Docker on Hugging Face Spaces. The deployment configuration is in `backend/Dockerfile`.

**To deploy:**
1.  Create a new Space on Hugging Face with Docker SDK
2.  Push your code to the Space repository:
    ```bash
    git remote add space https://huggingface.co/spaces/your-username/your-space
    git push space main
    ```
3.  The Space will automatically build and deploy using Docker
4.  The `songs.db` is created on startup and persists if using persistent storage
5.  Add reference songs by uploading them via the API or including them in the repository

**Note:** The `songs/` directory can be included in the repository or songs can be uploaded via the `/upload` endpoint after deployment.

### Frontend (Netlify/Vercel)

**Netlify:**
1.  Connect your GitHub repository to Netlify
2.  Configure build settings:
    -   **Base directory**: `frontend`
    -   **Build command**: `npm run build`
    -   **Publish directory**: `frontend/dist`
3.  Add environment variables:
    -   `VITE_API_URL`: Your backend URL (e.g., `https://your-space.hf.space`)
4.  Deploy!

**Vercel:**
1.  Install Vercel CLI: `npm i -g vercel`
2.  Navigate to frontend directory: `cd frontend`
3.  Configure environment:
    ```bash
    echo "VITE_API_URL=https://your-backend-url.hf.space" > .env.production
    ```
4.  Deploy: `vercel --prod`

## 🔌 API Reference

### `GET /health`
Health check endpoint.

**Response:**
```json
{"status": "ok"}
```

### `GET /songs`
Get list of all indexed songs.

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
Recognize an audio clip using multipart form upload.

**Request:**
- Content-Type: `multipart/form-data`
- Field: `audio` or `file` (audio file, max 10MB)

**Response:**
```json
{
  "match": {
    "title": "Song Title",
    "artist": "Artist Name",
    "score": 0.87
  }
}
```
Or `{"match": null}` if no match found.

**Score:** Confidence value between 0 and 1 (multiply by 100 for percentage).

### `POST /upload`
Upload a new song to the database.

**Request:**
- Content-Type: `multipart/form-data`
- Field: `audio` or `file` (audio file, max 10MB)
- Optional: `title` (text field)
- Optional: `artist` (text field)

**Response:**
```json
{
  "message": "Song uploaded and processing started",
  "path": "songs/filename.mp3",
  "title": "Song Title",
  "artist": "Artist Name"
}
```

### `GET /ws` (WebSocket)
Real-time audio recognition via WebSocket.

**Protocol:**
1. Client connects to `ws://localhost:8000/ws`
2. Client sends binary audio chunks (WAV format, max 1MB per message)
3. Server processes each chunk and responds when a match is found:
```json
{
  "type": "match",
  "match": {
    "title": "Song Title",
    "artist": "Artist Name",
    "score": 0.87
  }
}
```
4. Connection remains open for continuous recognition
5. Client closes connection when done

## 🔍 How It Works

### Fingerprinting Algorithm

1. **Preprocessing**: Audio is converted to 16kHz mono WAV using FFmpeg
2. **Decoding**: Audio samples are decoded using Symphonia
3. **STFT**: Short-Time Fourier Transform with 4096-sample windows and 50% overlap
4. **Peak Detection**: Identify spectral peaks in frequency bands (Low, Mid, High)
5. **Combinatorial Hashing**: Generate hashes from peak pairs:
   - Hash format: `[freq1: 9 bits][freq2: 9 bits][time_delta: 14 bits]`
   - Each anchor peak is paired with peaks 5-50 frames ahead
   - Only frequencies below 2kHz are used (512 bins)

### Matching Algorithm

1. **Query Fingerprinting**: Extract fingerprints from audio clip
2. **Database Lookup**: Find all matching hashes in SQLite database
3. **Histogram of Offsets**: For each candidate song:
   - Calculate relative time offset: `db_offset - query_offset`
   - Build histogram of offsets
   - Peak in histogram indicates time-aligned match
4. **Scoring**: Count of aligned fingerprints (peak height)
5. **Normalization**: Convert raw score to confidence: `1 - exp(-score/15)`
6. **Threshold**: Require minimum 8 aligned fingerprints for valid match

### Why This Works

- **Time Invariant**: Histogram approach handles different starting points
- **Noise Resistant**: Multiple aligned fingerprints required, not just one match
- **Fast**: SQLite indexed hash lookups + in-memory histogram
- **Accurate**: Combinatorial hashing creates unique signatures even from short clips

## 🔒 Security

-   **CORS**: Backend restricts origins to production frontend (`sonicamusic.netlify.app`) and localhost
-   **Rate Limiting**: Maximum 50 concurrent requests to prevent DoS attacks
-   **File Size Limits**: 
    -   HTTP uploads: 10MB maximum
    -   WebSocket messages: 1MB maximum per chunk
-   **Path Sanitization**: File uploads use sanitized filenames to prevent path traversal
-   **Input Validation**: Audio files validated for minimum size and format
-   **Temporary Files**: Cleaned up immediately after processing to prevent disk filling

## ⚡ Performance

-   **Recognition Speed**: < 0.5 seconds for most queries (depends on database size)
-   **Fingerprinting**: Parallel processing using Rayon for batch song indexing
-   **Database**: Indexed hash lookups for fast matching
-   **Memory Efficient**: Streaming audio processing, temporary files cleaned immediately
-   **Scalability**: Async Tokio runtime handles multiple concurrent requests
-   **WebSocket**: Real-time streaming with minimal latency for continuous recognition

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**Ways to Contribute:**
-   Report bugs or request features via [GitHub Issues](https://github.com/Vishal4742/Sonica_1/issues)
-   Submit pull requests for bug fixes or new features
-   Improve documentation
-   Add support for more audio formats
-   Optimize the fingerprinting algorithm

**Before submitting:**
-   Run `cargo fmt` for Rust code
-   Run `npm run lint` for frontend code
-   Test your changes locally
-   Write clear commit messages

## 🐛 Known Issues & Limitations

-   **Short Clips**: Recognition accuracy decreases for clips < 3 seconds
-   **Background Noise**: Very noisy environments may reduce matching accuracy
-   **Database Size**: Large databases (>10,000 songs) may impact query speed
-   **Format Support**: Requires FFmpeg for preprocessing non-standard formats
-   **System Audio Capture**: Browser support varies by platform (works best on Chrome/Edge)

## 🛣️ Roadmap

- [ ] Implement song deduplication detection
- [ ] Add support for live streaming recognition (radio, etc.)
- [ ] Optimize fingerprinting for very short clips (1-2 seconds)
- [ ] Add batch upload endpoint for multiple songs
- [ ] Implement song removal/update functionality
- [ ] Add visualization of matched fingerprints
- [ ] Support for more audio formats without FFmpeg dependency
- [ ] Mobile app (React Native or Flutter)
- [ ] Offline mode with local database

## 📚 Learn More

-   [Shazam Paper](https://www.ee.columbia.edu/~dpwe/papers/Wang03-shazam.pdf) - Original algorithm inspiration
-   [Rust Audio Processing](https://github.com/RustAudio) - Community resources
-   [Symphonia Documentation](https://docs.rs/symphonia/) - Audio decoding library
-   [Axum Web Framework](https://docs.rs/axum/) - Rust web framework

## 🙏 Acknowledgments

- Inspired by [Shazam](https://www.shazam.com/) and the original paper by Avery Li-Chun Wang
- Built as a Minor Project at Oriental College of Technology, Bhopal
- Team: Vishal Kumar, Navin Kumar, Khushi Kumari, Prabhat Raushan
- Guided by: Ms. Neelima Pandey

## 📄 License

MIT License - feel free to use and modify!

---

<div align="center">

**Made with ❤️ by the Sonica Team**

[Report Bug](https://github.com/Vishal4742/Sonica_1/issues) · [Request Feature](https://github.com/Vishal4742/Sonica_1/issues) · [Live Demo](https://sonicamusic.netlify.app)

</div>
