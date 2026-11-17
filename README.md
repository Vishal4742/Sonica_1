# Sonica 🎵

Sonica is an open-source music recognizer that works on your own music collection. Services like Shazam can only name a song that is already in their catalog. Sonica indexes the files you give it, so it can recognize tracks those services never indexed. We built it around a local collection of Bollywood and Bhojpuri songs.

A Rust service turns audio into a spectrogram, picks out spectral peaks (a constellation map), packs pairs of peaks into 32-bit hashes, and stores them in SQLite. A React client records audio in the browser and sends it to the service for recognition.

## 📄 Paper

> V. Kumar, S. Kumar, U. Patel, P. Kumar, and N. Kumar, "Sonica: Open-Source Audio Fingerprinting for Real-Time Music Recognition," accepted at the 2026 IEEE International Conference on Artificial Intelligence, Machine Learning & Cybernetics (IC-AIMLC), Oriental College of Technology, Bhopal, India, Nov. 2026. Paper ID 1219.

All numbers below come from the paper.

## 🚀 Live Demo

-   **Frontend:** [https://sonicamusic.netlify.app](https://sonicamusic.netlify.app)
-   **Backend API:** [https://huggingface.co/spaces/vishal4743/sonica-backend](https://huggingface.co/spaces/vishal4743/sonica-backend)

## ⚙️ How It Works

The pipeline follows the constellation-map method Wang described for Shazam (ISMIR 2003).

1.  **Preprocessing.** FFmpeg converts each input (MP3, AAC, OGG, FLAC or M4A) to 16 kHz mono WAV with a 100 Hz high-pass filter. Symphonia decodes the WAV into floating-point samples.
2.  **STFT.** Frames of N = 4096 samples with a 2048-sample hop (50% overlap) and a Hann window. At 16 kHz the bin spacing is 3.906 Hz. A five-second clip gives 37 frames.
3.  **Peak detection.** The spectrum is split into four bands: 0–195, 195–781, 781–1953 and 1953–8000 Hz. In each frame, the strongest bin in a band is kept if it is above an absolute floor, above twice the band mean, and a local maximum over three frames on either side.
4.  **Combinatorial hashing.** Each anchor peak is paired with target peaks 5 to 50 frames later (0.64–6.4 s), keeping at most **five pairs per anchor**. Each pair is packed into one 32-bit integer:

    ```
    hash = (f1 << 23) | (f2 << 14) | (t2 - t1)
    ```

    `f1` and `f2` are 9-bit frequency indices (0–511, so 0–1996 Hz) and the time delta uses 14 bits. A hash is emitted only when both peaks are below 2 kHz.
5.  **Offset-histogram matching.** For every hash shared by the query and the database, the matcher computes `offset = db_time - query_time` and builds a histogram per song. A true match piles its votes onto one offset. A song wins when its best bin has more than **5** matching fingerprints, and confidence is `1 - e^(-score/5)`.

## 📊 Results

Measured with release builds on an Intel Core i5-12500H laptop (12 cores, 16 threads, 16 GB RAM, Windows 11) against a 50-song collection of MP3, M4A and FLAC files.

### Performance (50-song database)

| Metric | Value |
| --- | --- |
| Recognition latency (5 s clip, server side) | 70–130 ms |
| Client round trip (`curl`) | ~0.3 s |
| Indexing 50 songs | ~8 s (parallel, via Rayon) |
| Fingerprint database | 670,002 fingerprints, 20.7 MB (0.414 MB per song) |
| Idle memory (database attached) | ~9 MB |
| Release binary size | 8.4 MB |

### Recognition test

Four five-second clips cut from indexed tracks (MP3, M4A and FLAC) all matched, with scores from 9 to 55. Three clips from tracks not in the database were all rejected, with scores of 3 or 4.

| Query (format) | Score | Confidence | Result |
| --- | --- | --- | --- |
| Indexed, MP3 | 49 | 100% | Match |
| Indexed, M4A | 55 | 100% | Match |
| Indexed, FLAC | 21 | 98% | Match |
| Indexed, MP3 | 9 | 84% | Match |
| Unindexed track A | 3 | – | Rejected |
| Unindexed track B | 4 | – | Rejected |

### Noise robustness (24 songs, additive white noise)

| SNR (dB) | 10 | 5 | 0 | −5 | −10 | −15 | −20 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Accuracy (%) | 100 | 100 | 96 | 88 | 38 | 4 | 0 |

At 0 dB the noise is as loud as the music, and 23 of 24 clips still matched.

### Scalability

The same eight query clips were run as the database grew. Seven matched at every size. One quiet 45–50 s segment never matched, because it produced too few peak pairs.

| Songs | Fingerprints | Server latency | DB size |
| --- | --- | --- | --- |
| 50 | 0.67 M | ~126 ms | 20.7 MB |
| 125 | 1.90 M | ~156 ms | 58.9 MB |
| 200 | 3.16 M | ~216 ms | 100 MB |

### Why the fan-out cap matters

Capping each anchor at five pairs cut the 50-song store from 11.5 million fingerprints (370 MB) to 670,002 fingerprints (20.7 MB). No recognition result changed: the same four indexed clips matched and the same three unindexed clips were rejected.

## ⚠️ Limitations

-   Hashes use absolute frequencies, so pitch shifts and time stretching break matches.
-   SQLite has one writer, which limits batch-indexing throughput before the FFT does.
-   It expects one audio source and cannot separate overlapping tracks.
-   It matches the exact recording, not cover versions.
-   Quiet, sparse passages can yield too few peak pairs to match.
-   The evaluation used a small collection and white noise only. Café and traffic recordings and a larger labeled set are planned.

## 🛠️ Tech Stack

### Backend (Rust)

| Crate | Version | Purpose |
| --- | --- | --- |
| axum | 0.7 | HTTP and WebSocket server |
| tokio | 1.x | Async runtime |
| rustfft | 6.2 | FFT |
| symphonia | 0.5 | Audio decoding |
| rusqlite | 0.31 | SQLite (WAL journaling, indexed hash column) |
| rayon | 1.8 | Parallel batch indexing |
| lofty | 0.22 | Title and artist metadata |
| tower-http | 0.5 | CORS |

Endpoints: `GET /health`, `GET /songs`, `POST /recognize`, `POST /upload` and the `GET /ws` WebSocket. The service allows 50 concurrent requests and rejects uploads over 10 MB. A filesystem watcher indexes songs added to `songs/`. The backend runs in Docker on Hugging Face Spaces.

### Frontend (React)

React 18.3 and Vite 5.4, styled with TailwindCSS, with Framer Motion and Lucide for transitions and icons. The `useAudioRecorder` hook captures audio with the MediaRecorder API and sends it over REST, and `useWebSocketAudio` streams it for lower latency. The frontend is deployed on Netlify.

## 📦 Installation & Setup

### Prerequisites
-   [Rust & Cargo](https://www.rust-lang.org/tools/install)
-   [Node.js & npm](https://nodejs.org/)
-   [FFmpeg](https://ffmpeg.org/download.html), which must be on your `PATH` because the backend calls it to decode and resample audio
-   [Git LFS](https://git-lfs.github.com/), needed to fetch the bundled fingerprint database (`backend/songs.db`)

### 1. Clone the Repository
```bash
git clone https://github.com/Vishal4742/Sonica_1.git
cd Sonica_1
git lfs install
git lfs pull
```

### 2. Backend Setup
```bash
cd backend
# Put your reference audio files in the 'songs/' directory
mkdir -p songs
# Run the server (it scans and indexes the songs on startup)
cargo run --release
```
The server starts on `http://localhost:8000`. To rebuild the database from scratch, run the `reindex` binary.

### 3. Frontend Setup
```bash
cd frontend
npm install
npm run dev
```
The app runs at `http://localhost:5173`.

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
MIT License. The song collection used in the paper is not redistributed with Sonica.
