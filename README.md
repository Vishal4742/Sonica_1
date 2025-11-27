# Sonica

Sonica is a powerful audio recognition system inspired by Shazam. It allows users to identify songs by analyzing audio fingerprints. The project consists of a high-performance Rust backend and a modern React frontend.

## Features

-   **Audio Fingerprinting**: Uses advanced signal processing (STFT, peak detection) to generate unique fingerprints for audio tracks.
-   **Fast Matching**: Implements a combinatorial hashing and "histogram of offsets" algorithm for rapid and accurate song identification.
-   **Real-time Recognition**: Capable of recognizing songs from short audio snippets.
-   **Modern UI**: A sleek, responsive interface built with React, Vite, and TailwindCSS.

## Tech Stack

### Backend
-   **Language**: Rust
-   **Framework**: Axum (Web Framework)
-   **Database**: SQLite (via Rusqlite)
-   **Audio Processing**: Symphonia, RustFFT
-   **Concurrency**: Tokio, Rayon

### Frontend
-   **Framework**: React
-   **Build Tool**: Vite
-   **Styling**: TailwindCSS
-   **Animations**: Framer Motion
-   **Icons**: Lucide React

## Getting Started

### Prerequisites
-   [Rust & Cargo](https://www.rust-lang.org/tools/install)
-   [Node.js & npm](https://nodejs.org/)

### Installation & Running

#### Backend

1.  Navigate to the backend directory:
    ```bash
    cd backend
    ```

2.  Run the server:
    ```bash
    cargo run
    ```
    The backend server will start (default port: 3000 or 8000, check logs).

#### Frontend

1.  Navigate to the frontend directory:
    ```bash
    cd frontend
    ```

2.  Install dependencies:
    ```bash
    npm install
    ```

3.  Start the development server:
    ```bash
    npm run dev
    ```
    The application will be available at `http://localhost:5173`.

## Usage
1.  Ensure both backend and frontend servers are running.
2.  Open the web application in your browser.
3.  Use the interface to upload an audio file or record audio to identify the song.
