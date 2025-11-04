pub mod api;
pub mod error;
pub mod fingerprint;
pub mod storage;
pub mod types;
pub mod watcher;

use crate::error::{AppError, Result};
use crate::storage::Database;
use crate::types::SongFp;
use crate::api::{AppState, create_router};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Sonica Backend...");

    // Initialize database
    let db = Arc::new(Database::new("songs.db")?);
    info!("Database initialized");

    // Load existing songs and scan songs/ directory
    info!("Loading existing songs...");
    load_and_process_songs(&db).await?;

    // Initialize cache
    let all_songs = db.get_all_fingerprints()?;
    let cache = crate::types::CACHE.get_or_init(|| {
        std::sync::RwLock::new(all_songs)
    });
    info!("Cache initialized with {} songs", cache.read().unwrap().len());

    // Start file watcher
    let db_watcher = Arc::clone(&db);
    let watch_handler: Arc<dyn Fn(String) + Send + Sync> = Arc::new(move |path: String| {
        let db = Arc::clone(&db_watcher);
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

    // Create router
    let app = create_router(app_state);

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await
        .map_err(|e| AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to bind to port 8000: {}", e),
        )))?;

    info!("Server listening on http://0.0.0.0:8000");
    
    axum::serve(listener, app).await
        .map_err(|e| AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Server error: {}", e),
        )))?;

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

    let entries = fs::read_dir(songs_dir).await?;
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
            match tokio::runtime::Handle::try_current() {
                Ok(handle) => {
                    let db_clone = Arc::clone(&db_arc);
                    handle.block_on(process_song(&db_clone, path, &title, &artist))
                        .map(|_| ())
                        .map_err(|e| {
                            error!("Error processing {}: {}", path, e);
                            e
                        })
                        .ok()
                }
                Err(_) => {
                    error!("No tokio runtime available for {}", path);
                    None
                }
            }
        })
        .collect();

    info!("Processed {} new songs", processed.len());
    Ok(())
}

async fn process_song(
    db: &Arc<Database>,
    path: &str,
    title: &str,
    artist: &str,
) -> Result<()> {
    // Preprocess with FFmpeg
    let temp_output = format!("temp/{}_processed.wav", uuid::Uuid::new_v4());
    
    fingerprint::preprocess_audio(path, &temp_output)?;

    // Extract fingerprint
    let fingerprint = fingerprint::extract_mfcc(&temp_output)?;
    let normalized_fp = fingerprint::normalize_vector(&fingerprint);

    // Clean up temp file
    let _ = fs::remove_file(&temp_output);

    // Save to database
    let song_id = db.insert_song(title, artist, path, &normalized_fp)?;

    // Update cache
    if let Some(cache) = crate::types::CACHE.get() {
        let mut cache_guard = cache.write()
            .map_err(|e| AppError::Fingerprint(format!("Cache lock error: {}", e)))?;
        
        cache_guard.push(SongFp {
            id: song_id,
            title: title.to_string(),
            artist: artist.to_string(),
            path: path.to_string(),
            fingerprint: normalized_fp,
        });
    }

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
