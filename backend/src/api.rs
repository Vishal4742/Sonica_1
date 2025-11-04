use axum::{
    extract::{Multipart, State},
    response::Json,
    routing::{get, post},
    Router,
};
use crate::error::{AppError, Result};
use crate::types::{RecognitionResponse, SongMetadata};
use crate::storage::Database;
use crate::fingerprint::{extract_mfcc, find_best_match, normalize_vector, preprocess_audio};
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use uuid::Uuid;

pub struct AppState {
    pub db: Arc<Database>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/songs", get(list_songs))
        .route("/recognize", post(recognize))
        .route("/upload", post(upload))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn list_songs(
    State(state): State<AppState>,
) -> Result<Json<Vec<SongMetadata>>> {
    let songs = state.db.get_all_songs()?;
    Ok(Json(songs))
}

async fn recognize(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<RecognitionResponse>> {
    let mut audio_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;

    while let Some(field) = multipart.next_field().await
        .map_err(|e| AppError::InvalidRequest(format!("Multipart error: {}", e)))?
    {
        let field_name = field.name().unwrap_or("");
        
        if field_name == "audio" || field_name == "file" {
            filename = field.file_name().map(|s| s.to_string());
            let data = field.bytes().await
                .map_err(|e| AppError::InvalidRequest(format!("Failed to read file: {}", e)))?;
            audio_data = Some(data.to_vec());
        }
    }

    let audio_data = audio_data.ok_or_else(|| AppError::InvalidRequest("No audio file provided".to_string()))?;
    
    if audio_data.len() < 1000 {
        return Err(AppError::InvalidRequest("Audio file too small".to_string()));
    }

    // Save to temp file
    let temp_input = format!("temp/{}", Uuid::new_v4());
    let temp_output = format!("temp/{}_processed.wav", Uuid::new_v4());
    
    fs::create_dir_all("temp").await?;
    fs::write(&temp_input, &audio_data).await?;

    // Preprocess with FFmpeg
    preprocess_audio(&temp_input, &temp_output)?;

    // Extract fingerprint
    let fingerprint = extract_mfcc(&temp_output)?;
    let normalized_fp = normalize_vector(&fingerprint);

    // Clean up temp files
    let _ = fs::remove_file(&temp_input).await;
    let _ = fs::remove_file(&temp_output).await;

    // Get cache
    let cache = crate::types::CACHE
        .get()
        .ok_or_else(|| AppError::Fingerprint("Cache not initialized".to_string()))?;
    
    let cache_guard = cache.read()
        .map_err(|e| AppError::Fingerprint(format!("Cache lock error: {}", e)))?;

    // Find best match
    let match_result = find_best_match(&normalized_fp, &cache_guard)
        .map(|(m, _)| m);

    Ok(Json(RecognitionResponse {
        r#match: match_result,
    }))
}

async fn upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>> {
    let mut audio_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut title: Option<String> = None;
    let mut artist: Option<String> = None;

    while let Some(field) = multipart.next_field().await
        .map_err(|e| AppError::InvalidRequest(format!("Multipart error: {}", e)))?
    {
        let field_name = field.name().unwrap_or("");
        
        match field_name {
            "audio" | "file" => {
                filename = field.file_name().map(|s| s.to_string());
                let data = field.bytes().await
                    .map_err(|e| AppError::InvalidRequest(format!("Failed to read file: {}", e)))?;
                audio_data = Some(data.to_vec());
            }
            "title" => {
                let data = field.text().await
                    .map_err(|e| AppError::InvalidRequest(format!("Failed to read title: {}", e)))?;
                title = Some(data);
            }
            "artist" => {
                let data = field.text().await
                    .map_err(|e| AppError::InvalidRequest(format!("Failed to read artist: {}", e)))?;
                artist = Some(data);
            }
            _ => {}
        }
    }

    let audio_data = audio_data.ok_or_else(|| AppError::InvalidRequest("No audio file provided".to_string()))?;
    let filename = filename.ok_or_else(|| AppError::InvalidRequest("No filename provided".to_string()))?;
    
    let title = title.unwrap_or_else(|| {
        Path::new(&filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string()
    });
    let artist = artist.unwrap_or_else(|| "Unknown".to_string());

    // Save to songs directory
    let song_path = format!("songs/{}", filename);
    fs::write(&song_path, &audio_data).await?;

    // Check if already exists
    if state.db.song_exists_by_path(&song_path)? {
        return Ok(Json(serde_json::json!({
            "message": "Song already exists",
            "path": song_path
        })));
    }

    // Process asynchronously
    let db_clone = Arc::clone(&state.db);
    let path_clone = song_path.clone();
    let title_clone = title.clone();
    let artist_clone = artist.clone();

    tokio::spawn(async move {
        if let Err(e) = process_new_song(&db_clone, &path_clone, &title_clone, &artist_clone).await {
            eprintln!("Error processing song {}: {}", path_clone, e);
        }
    });

    Ok(Json(serde_json::json!({
        "message": "Song uploaded and processing started",
        "path": song_path,
        "title": title,
        "artist": artist
    })))
}

async fn process_new_song(
    db: &Arc<Database>,
    path: &str,
    title: &str,
    artist: &str,
) -> Result<()> {
    // Preprocess with FFmpeg
    let temp_output = format!("temp/{}_processed.wav", Uuid::new_v4());
    fs::create_dir_all("temp").await?;
    
    preprocess_audio(path, &temp_output)?;

    // Extract fingerprint
    let fingerprint = extract_mfcc(&temp_output)?;
    let normalized_fp = normalize_vector(&fingerprint);

    // Clean up temp file
    let _ = fs::remove_file(&temp_output).await;

    // Save to database
    let song_id = db.insert_song(title, artist, path, &normalized_fp)?;

    // Update cache
    if let Some(cache) = crate::types::CACHE.get() {
        let mut cache_guard = cache.write()
            .map_err(|e| AppError::Fingerprint(format!("Cache lock error: {}", e)))?;
        
        cache_guard.push(crate::types::SongFp {
            id: song_id,
            title: title.to_string(),
            artist: artist.to_string(),
            path: path.to_string(),
            fingerprint: normalized_fp,
        });
    }

    Ok(())
}
