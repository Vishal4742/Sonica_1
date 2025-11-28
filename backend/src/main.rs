pub mod api;
pub mod error;
pub mod fingerprint;
pub mod storage;
pub mod types;
pub mod watcher;

use crate::api::{create_router, AppState};
use crate::error::{AppError, Result};
use crate::storage::Database;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::fs as tokio_fs;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with better visibility
    tracing_subscriber::fmt()
        .with_target(false) // Don't show module paths
        .with_thread_ids(false) // Don't show thread IDs
        .with_file(false) // Don't show file names
        .with_line_number(false) // Don't show line numbers
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sonica_backend=info,tower_http=info".into()),
        )
        .with_ansi(true) // Enable colors
        .init();

    info!("Starting Sonica Backend (Shazam Engine)...");

    // Initialize database
    let db = Arc::new(Database::new("songs.db")?);
    let song_count = db.get_all_songs()?.len();
    info!("Database initialized with {} songs", song_count);

    // Load existing songs and scan songs/ directory
    info!("Scanning songs...");
    load_and_process_songs(&db).await?;

    // Start file watcher
    let db_watcher = Arc::clone(&db);
    let watch_handler: Arc<dyn Fn(String) + Send + Sync> = Arc::new(move |path: String| {
        let db = Arc::clone(&db_watcher);
        let path_clone = path.clone();

        tokio::spawn(async move {
            info!("New file detected: {}", path_clone);

            // Extract metadata from filename
            let filename = Path::new(&path_clone)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown");

            let title = Path::new(filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string();
            let artist = "Unknown".to_string();

            // Check if already processed
            if db.song_exists_by_path(&path_clone).unwrap_or(false) {
                info!("Song already processed: {}", path_clone);
                return;
            }

            // Process the song
            if let Err(e) = process_song(&Arc::clone(&db), &path_clone, &title, &artist).await {
                error!("Error processing song {}: {}", path_clone, e);
            } else {
                info!("Successfully processed: {}", path_clone);
            }
        });
    });

    watcher::start_watcher("songs", watch_handler).await?;
    info!("File watcher started for songs/ directory");

    // Create app state
    let app_state = AppState {
        db: Arc::clone(&db),
    };

    // Create router with logging middleware
    let cors = tower_http::cors::CorsLayer::permissive();
    let trace_layer = tower_http::trace::TraceLayer::new_for_http()
        .make_span_with(tower_http::trace::DefaultMakeSpan::new().level(tracing::Level::INFO))
        .on_request(|_request: &axum::http::Request<_>, _span: &tracing::Span| {
            tracing::info!("→ {} {}", _request.method(), _request.uri().path());
        })
        .on_response(
            |_response: &axum::http::Response<_>,
             latency: std::time::Duration,
             _span: &tracing::Span| {
                tracing::info!("← {} {}ms", _response.status(), latency.as_millis());
            },
        );

    let app = create_router(app_state).layer(cors).layer(trace_layer);

    // Start server
    let port = std::env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let bind_addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| {
            AppError::Io(std::io::Error::other(format!(
                "Failed to bind to {}: {}",
                bind_addr, e
            )))
        })?;

    info!("Server listening on http://{}", bind_addr);

    axum::serve(listener, app)
        .await
        .map_err(|e| AppError::Io(std::io::Error::other(format!("Server error: {}", e))))?;

    Ok(())
}

async fn load_and_process_songs(db: &Arc<Database>) -> Result<()> {
    // Ensure songs directory exists
    fs::create_dir_all("songs")?;
    fs::create_dir_all("temp")?;

    // Scan songs directory
    let songs_dir = Path::new("songs");
    if !songs_dir.exists() {
        return Ok(());
    }

    let entries = tokio_fs::read_dir(songs_dir).await?;
    let mut paths = Vec::new();

    let mut entries = entries;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() && is_audio_file(&path) {
            if let Some(path_str) = path.to_str() {
                paths.push(path_str.to_string());
            }
        }
    }

    info!("Found {} audio files in songs/ directory", paths.len());

    // Process files in parallel
    use rayon::prelude::*;

    let db_arc = Arc::clone(db);
    let handle = tokio::runtime::Handle::current();

    let processed: Vec<_> = paths
        .par_iter()
        .filter_map(|path| {
            // Check if already processed
            if db_arc.song_exists_by_path(path).unwrap_or(false) {
                return None;
            }

            // Extract metadata from filename
            let filename = Path::new(path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown");

            let title = Path::new(filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string();
            let artist = "Unknown".to_string();

            // Process synchronously (we're in rayon thread)
            let db_clone = Arc::clone(&db_arc);
            handle
                .block_on(process_song(&db_clone, path, &title, &artist))
                .map(|_| ())
                .map_err(|e| {
                    error!("Error processing {}: {}", path, e);
                    e
                })
                .ok()
        })
        .collect();

    info!("Processed {} new songs", processed.len());
    Ok(())
}

async fn process_song(db: &Arc<Database>, path: &str, title: &str, artist: &str) -> Result<()> {
    // Preprocess with FFmpeg
    let temp_output = format!("temp/{}_processed.wav", uuid::Uuid::new_v4());

    fingerprint::preprocess_audio(path, &temp_output)?;

    // Decode and Fingerprint
    let samples = fingerprint::decode_audio(&temp_output)?;
    let fingerprints = fingerprint::generate_fingerprints(&samples);

    // Clean up temp file
    let _ = fs::remove_file(&temp_output);

    // Save to database
    db.insert_song(title, artist, path, &fingerprints)?;

    Ok(())
}

fn is_audio_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_lower = ext.to_string_lossy().to_lowercase();
        matches!(
            ext_lower.as_str(),
            "mp3" | "wav" | "flac" | "m4a" | "aac" | "ogg" | "opus" | "wma"
        )
    } else {
        false
    }
}
