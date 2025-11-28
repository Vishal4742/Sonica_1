# Sonica 🎵

Sonica is an open-source, high-performance music recognition system inspired by Shazam. It identifies songs in real-time by analyzing audio fingerprints using a robust backend written in Rust and a modern, responsive frontend built with React.

## 🚀 Live Demo

-   **Frontend:** [https://sonicamusic.netlify.app](https://sonicamusic.netlify.app)
-   **Backend API:** [https://huggingface.co/spaces/vishal4743/sonica-backend](https://huggingface.co/spaces/vishal4743/sonica-backend)

## ✨ Features

-   **Real-time Recognition**: Identifies songs from microphone input or system audio in seconds.
-   **Advanced Fingerprinting**: Uses STFT (Short-Time Fourier Transform) and peak detection to create unique audio signatures.
-   **Robust Matching**: Implements a "Histogram of Offsets" algorithm for high-accuracy matching, even with background noise.
-   **Metadata Extraction**: Automatically extracts Title and Artist information from audio files using ID3 tags (via `lofty`).
-   **WebSocket Streaming**: Low-latency audio streaming from client to server.
-   **Modern UI**: Beautiful, dark-themed interface with real-time visualizations using Framer Motion and TailwindCSS.

## 🛠️ Tech Stack

### Backend (Rust)
-   **Framework**: Axum (High-performance web framework)
-   **Audio Processing**: Symphonia (Decoding), RustFFT (Frequency analysis)
-   **Database**: SQLite (via Rusqlite) for storing fingerprints
-   **Concurrency**: Tokio (Async runtime) & Rayon (Parallel processing)
-   **Deployment**: Docker on Hugging Face Spaces

### Frontend (React)
-   **Build Tool**: Vite
-   **Styling**: TailwindCSS
-   **State Management**: React Hooks
-   **Audio Capture**: MediaRecorder API
-   **Deployment**: Netlify

## 📦 Installation & Setup

### Prerequisites
-   [Rust & Cargo](https://www.rust-lang.org/tools/install)
-   [Node.js & npm](https://nodejs.org/)
-   [Git LFS](https://git-lfs.github.com/) (Required for large audio files)

### 1. Clone the Repository
```bash
git clone https://github.com/Vishal4742/Sonica.git
cd Sonica
git lfs install
git lfs pull
```

### 2. Backend Setup
```bash
cd backend
# Place your reference audio files (.mp3, .wav) in the 'songs/' directory
mkdir -p songs
# Run the server (it will automatically scan and index songs)
cargo run --release
```
The server will start on `http://localhost:8000`.

### 3. Frontend Setup
```bash
cd frontend
npm install
npm run dev
```
The app will run at `http://localhost:5173`.

## 🚀 Deployment

### Backend (Hugging Face Spaces)
The backend is deployed using Docker. To update:
1.  Ensure `songs.db` is **not** pushed (it is rebuilt on the server).
2.  Use Git LFS for any audio files >10MB in `songs/`.
3.  Push the `backend` folder to your Space:
    ```bash
    git subtree push --prefix backend space main
    ```

### Frontend (Netlify/Vercel)
1.  Connect your repository.
2.  Set the Build Command: `npm run build`.
3.  Set the Publish Directory: `dist`.
4.  **Important:** Add an Environment Variable:
    -   `VITE_API_URL`: `https://your-backend-url.hf.space`

## 🔒 Security
-   **CORS**: The backend is configured to only accept requests from the production frontend and localhost.
-   **Data**: Audio is processed in memory and temporary chunks are deleted immediately after analysis.

## 📄 License
MIT License - feel free to use and modify!
