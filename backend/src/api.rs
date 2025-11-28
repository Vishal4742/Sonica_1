use crate::error::{AppError, Result};
use crate::fingerprint::{generate_fingerprints, preprocess_audio};
use crate::storage::Database;
use crate::types::{MatchResult, RecognitionResponse, SongMetadata};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Multipart, State,
    },
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tracing::{info, warn};
use uuid::Uuid;

const MATCH_THRESHOLD: i64 = 8;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/songs", get(list_songs))
        .route("/recognize", post(recognize))
        .route("/upload", post(upload))
        .route("/ws", get(ws_handler))
        .with_state(state)
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    info!("New WebSocket connection");

    while let Some(msg) = socket.recv().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                warn!("WebSocket error: {}", e);
                return;
            }
        };

        match msg {
            Message::Binary(data) => {
                info!("Received binary data size: {} bytes", data.len());
                // Security: Max message size check (e.g., 1MB)
                if data.len() > 1_000_000 {
                    warn!("Message too large, closing connection");
                    return;
                }

                let state_clone = state.clone();

                let task_result =
                    tokio::task::spawn_blocking(move || -> Result<Option<MatchResult>> {
                        // 1. Save to temp
                        let temp_id = Uuid::new_v4();
                        let temp_path =
                            std::env::temp_dir().join(format!("upload_{}.wav", temp_id));
                        std::fs::write(&temp_path, &data)
                            .map_err(|e| AppError::Internal(e.to_string()))?;

                        // 2. Preprocess
                        let processed_path =
                            std::env::temp_dir().join(format!("processed_{}.wav", temp_id));

                        if let Err(e) = preprocess_audio(
                            temp_path.to_str().unwrap(),
                            processed_path.to_str().unwrap(),
                        ) {
                            warn!("FFmpeg error: {}", e);
                            let _ = std::fs::remove_file(&temp_path);
                            return Err(e);
                        }

                        // Now decode the processed file
                        let samples = match crate::fingerprint::decode_audio(
                            processed_path.to_str().unwrap(),
                        ) {
                            Ok(s) => s,
                            Err(e) => {
                                warn!("Decode error: {}", e);
                                let _ = std::fs::remove_file(&temp_path);
                                let _ = std::fs::remove_file(&processed_path);
                                return Err(e);
                            }
                        };

                        let _ = std::fs::remove_file(temp_path);
                        let _ = std::fs::remove_file(processed_path);

                        // 3. Fingerprint
                        let fingerprints = generate_fingerprints(&samples);
                        info!("Generated {} fingerprints", fingerprints.len());

                        if fingerprints.is_empty() {
                            return Ok(None);
                        }

                        // 4. Match
                        let matches = state_clone.db.find_matches(&fingerprints)?;

                        // Histogram logic
                        let mut best_song_id = -1;
                        let mut best_score = 0;

                        for (song_id, offsets) in matches {
                            let mut histogram = HashMap::new();
                            let mut max_count = 0;
                            for (db_offset, query_offset) in offsets {
                                let relative_offset = (db_offset as i64) - (query_offset as i64);
                                let count = histogram.entry(relative_offset).or_insert(0);
                                *count += 1;
                                if *count > max_count {
                                    max_count = *count;
                                }
                            }
                            if max_count > best_score {
                                best_score = max_count;
                                best_song_id = song_id;
                            }
                        }

                        info!("Best score: {}", best_score);

                        if best_score > MATCH_THRESHOLD {
                            if let Some(metadata) =
                                state_clone.db.get_song_metadata(best_song_id)?
                            {
                                let scale_factor = 15.0;
                                let normalized_score =
                                    1.0 - (-(best_score as f32) / scale_factor).exp();
                                let clamped_score = normalized_score.min(1.0).max(0.0);

                                return Ok(Some(MatchResult {
                                    title: metadata.title,
                                    artist: metadata.artist,
                                    score: clamped_score,
                                }));
                            }
                        }

                        Ok(None)
                    })
                    .await;

                match task_result {
                    Ok(Ok(Some(match_result))) => {
                        info!(
                            "Match found: {} - {}",
                            match_result.artist, match_result.title
                        );
                        // Found a match! Send it back
                        let response = serde_json::json!({
                            "type": "match",
                            "match": match_result
                        });
                        if let Err(e) = socket.send(Message::Text(response.to_string())).await {
                            warn!("Failed to send match: {}", e);
                            break;
                        }
                    }
                    Ok(Err(e)) => {
                        warn!("Processing error: {}", e);
                    }
                    _ => {} // No match or task error
                }
            }
            Message::Close(_) => {
                return;
            }
            _ => {}
        }
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn list_songs(State(state): State<AppState>) -> Result<Json<Vec<SongMetadata>>> {
    info!("List songs request received");
    let songs = state.db.get_all_songs()?;
    info!("Returning {} songs", songs.len());
    Ok(Json(songs))
}

async fn recognize(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<RecognitionResponse>> {
    info!("Recognition request received");
    let mut audio_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::InvalidRequest(format!("Multipart error: {}", e)))?
    {
        let field_name = field.name().unwrap_or("");
        if field_name == "audio" || field_name == "file" {
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::InvalidRequest(format!("Failed to read file: {}", e)))?;
            audio_data = Some(data.to_vec());
        }
    }

    let audio_data =
        audio_data.ok_or_else(|| AppError::InvalidRequest("No audio file provided".to_string()))?;

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

    // Decode and Fingerprint
    let samples = crate::fingerprint::decode_audio(&temp_output)?;
    let fingerprints = generate_fingerprints(&samples);

    // Clean up temp files
    let _ = fs::remove_file(&temp_input).await;
    let _ = fs::remove_file(&temp_output).await;

    if fingerprints.is_empty() {
        return Ok(Json(RecognitionResponse { r#match: None }));
    }

    // Find matches in DB
    let matches = state.db.find_matches(&fingerprints)?;

    // Histogram of Offsets Algorithm
    let mut best_song_id = -1;
    let mut best_score = 0;

    for (song_id, offsets) in matches {
        let mut histogram = HashMap::new();
        let mut max_count = 0;

        for (db_offset, query_offset) in offsets {
            // relative_offset = db_offset - query_offset
            // We use wrapping arithmetic or offset to avoid negative numbers if needed,
            // but here we can just use i64.
            let relative_offset = (db_offset as i64) - (query_offset as i64);
            let count = histogram.entry(relative_offset).or_insert(0);
            *count += 1;
            if *count > max_count {
                max_count = *count;
            }
        }

        if max_count > best_score {
            best_score = max_count;
            best_song_id = song_id;
        }
    }

    // Threshold for a match
    // Need at least X matching points aligned in time
    if best_score > MATCH_THRESHOLD {
        if let Some(metadata) = state.db.get_song_metadata(best_song_id)? {
            // Normalize score to 0-1 range (frontend will multiply by 100 for percentage)
            // The score is the count of matching fingerprint points aligned in time
            // Use a logarithmic scale to map to 0-1 range more naturally
            // Formula: normalized = 1 - exp(-score / scale_factor)
            // This ensures scores are always between 0 and 1

            // Scale factor: higher values = slower growth toward 1.0
            // For score of 25, we want ~0.85, so: 1 - exp(-25/15) ≈ 0.81
            // For score of 40, we want ~0.95, so: 1 - exp(-40/15) ≈ 0.93
            let scale_factor = 15.0;
            let normalized_score = 1.0 - (-(best_score as f32) / scale_factor).exp();

            // Clamp to ensure it's always between 0 and 1
            let clamped_score = normalized_score.min(1.0).max(0.0);

            info!(
                "Match found: {} - {} (raw score: {}, confidence: {:.1}%)",
                metadata.title,
                metadata.artist,
                best_score,
                clamped_score * 100.0
            );
            return Ok(Json(RecognitionResponse {
                r#match: Some(MatchResult {
                    title: metadata.title,
                    artist: metadata.artist,
                    score: clamped_score, // Return as 0-1 range (frontend multiplies by 100)
                }),
            }));
        }
    }

    warn!(
        "No match found (best score: {}, threshold: {})",
        best_score, MATCH_THRESHOLD
    );
    Ok(Json(RecognitionResponse { r#match: None }))
}

async fn upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>> {
    info!("Upload request received");
    let mut audio_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut title: Option<String> = None;
    let mut artist: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::InvalidRequest(format!("Multipart error: {}", e)))?
    {
        let field_name = field.name().unwrap_or("");

        match field_name {
            "audio" | "file" => {
                filename = field.file_name().map(|s| s.to_string());
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::InvalidRequest(format!("Failed to read file: {}", e)))?;
                audio_data = Some(data.to_vec());
            }
            "title" => {
                let data = field.text().await.map_err(|e| {
                    AppError::InvalidRequest(format!("Failed to read title: {}", e))
                })?;
                title = Some(data);
            }
            "artist" => {
                let data = field.text().await.map_err(|e| {
                    AppError::InvalidRequest(format!("Failed to read artist: {}", e))
                })?;
                artist = Some(data);
            }
            _ => {}
        }
    }

    let audio_data =
        audio_data.ok_or_else(|| AppError::InvalidRequest("No audio file provided".to_string()))?;
    let filename =
        filename.ok_or_else(|| AppError::InvalidRequest("No filename provided".to_string()))?;

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
        if let Err(e) = process_new_song(&db_clone, &path_clone, &title_clone, &artist_clone).await
        {
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

async fn process_new_song(db: &Arc<Database>, path: &str, title: &str, artist: &str) -> Result<()> {
    // Preprocess with FFmpeg
    let temp_output = format!("temp/{}_processed.wav", Uuid::new_v4());
    fs::create_dir_all("temp").await?;

    preprocess_audio(path, &temp_output)?;

    // Extract fingerprint
    let samples = crate::fingerprint::decode_audio(&temp_output)?;
    let fingerprints = generate_fingerprints(&samples);

    // Clean up temp file
    let _ = fs::remove_file(&temp_output).await;

    // Save to database
    db.insert_song(title, artist, path, &fingerprints)?;

    Ok(())
}
