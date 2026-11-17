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
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tracing::{info, warn};
use uuid::Uuid;

const MATCH_THRESHOLD: i64 = 5;
const MAX_UPLOAD_SIZE: usize = 10_000_000; // 10MB limit for uploads
const SESSION_SECS: usize = 12; // a live session gives up after this many 1 s slices

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

    // A client that sends "start" streams one continuous recording in 1 s slices.
    // Only the first slice carries the container header, so slices are matched
    // together, and the growing session is retried until a match or SESSION_SECS.
    // Clients that never send "start" (older frontends) send one complete clip per message.
    let mut uses_sessions = false;
    let mut session: Option<Vec<u8>> = None;
    let mut slices = 0;

    while let Some(msg) = socket.recv().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                warn!("WebSocket error: {}", e);
                return;
            }
        };

        let audio = match msg {
            Message::Text(_) => {
                uses_sessions = true;
                session = Some(Vec::new());
                slices = 0;
                continue;
            }
            Message::Binary(data) => {
                // Security: Max message size check (e.g., 1MB)
                if data.len() > 1_000_000 {
                    warn!("Message too large, closing connection");
                    return;
                }
                match session.as_mut() {
                    Some(buf) => {
                        buf.extend_from_slice(&data);
                        slices += 1;
                        if slices < 2 {
                            continue; // one second is too short to match
                        }
                        buf.clone()
                    }
                    None if uses_sessions => continue, // slice still in flight after a reply
                    None => data,
                }
            }
            Message::Close(_) => return,
            _ => continue,
        };

        let db = Arc::clone(&state.db);
        let found = match tokio::task::spawn_blocking(move || identify(&db, &audio)).await {
            Ok(Ok(found)) => found,
            Ok(Err(e)) => {
                warn!("Processing error: {}", e);
                None
            }
            Err(e) => {
                warn!("Recognition task failed: {}", e);
                None
            }
        };

        let reply = match found {
            Some(m) => serde_json::json!({ "type": "match", "match": m }),
            None if session.is_some() && slices >= SESSION_SECS => {
                serde_json::json!({ "type": "no_match" })
            }
            None => continue,
        };
        session = None; // a reply ends the session
        if let Err(e) = socket.send(Message::Text(reply.to_string())).await {
            warn!("Failed to send reply: {}", e);
            break;
        }
    }
}

/// Deletes the file when dropped, so error paths clean up too.
struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Decode, fingerprint and match one clip. Blocking work: call it from spawn_blocking.
fn identify(db: &Database, audio: &[u8]) -> Result<Option<MatchResult>> {
    let id = Uuid::new_v4();
    let input = TempFile(std::env::temp_dir().join(format!("upload_{}", id)));
    let output = TempFile(std::env::temp_dir().join(format!("processed_{}.wav", id)));
    std::fs::write(&input.0, audio)?;
    preprocess_audio(input.0.to_str().unwrap(), output.0.to_str().unwrap())?;
    let samples = crate::fingerprint::decode_audio(output.0.to_str().unwrap())?;

    let fingerprints = generate_fingerprints(&samples);
    info!("Generated {} fingerprints", fingerprints.len());
    if fingerprints.is_empty() {
        return Ok(None);
    }

    // Histogram of offsets: a real match lines up many hashes at one time offset
    let mut best_song_id = -1;
    let mut best_score = 0;
    for (song_id, offsets) in db.find_matches(&fingerprints)? {
        let mut histogram = HashMap::new();
        let mut max_count = 0;
        for (db_offset, query_offset) in offsets {
            let count = histogram
                .entry(db_offset as i64 - query_offset as i64)
                .or_insert(0);
            *count += 1;
            max_count = max_count.max(*count);
        }
        if max_count > best_score {
            best_score = max_count;
            best_song_id = song_id;
        }
    }
    info!("Best score: {} (threshold {})", best_score, MATCH_THRESHOLD);

    if best_score <= MATCH_THRESHOLD {
        return Ok(None);
    }
    let Some(song) = db.get_song_metadata(best_song_id)? else {
        return Ok(None);
    };
    info!("Match: {} - {} ({} aligned hashes)", song.artist, song.title, best_score);
    Ok(Some(MatchResult {
        title: song.title,
        artist: song.artist,
        // Map the aligned-hash count onto 0-1 for display (frontend shows a percentage)
        score: (1.0 - (-(best_score as f32) / 5.0).exp()).clamp(0.0, 1.0),
    }))
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

    // Security: Validate file size
    if audio_data.len() < 1000 {
        return Err(AppError::InvalidRequest("Audio file too small".to_string()));
    }
    if audio_data.len() > MAX_UPLOAD_SIZE {
        return Err(AppError::InvalidRequest(format!(
            "Audio file too large. Maximum size is {} bytes",
            MAX_UPLOAD_SIZE
        )));
    }

    let db = Arc::clone(&state.db);
    let found = tokio::task::spawn_blocking(move || identify(&db, &audio_data))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))??;
    Ok(Json(RecognitionResponse { r#match: found }))
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

    // Security: Validate file size
    if audio_data.len() > MAX_UPLOAD_SIZE {
        return Err(AppError::InvalidRequest(format!(
            "Audio file too large. Maximum size is {} bytes",
            MAX_UPLOAD_SIZE
        )));
    }

    // Security: Sanitize filename to prevent path traversal attacks
    let safe_filename = Path::new(&filename)
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or_else(|| AppError::InvalidRequest("Invalid filename".to_string()))?;

    let title = title.unwrap_or_else(|| {
        Path::new(safe_filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string()
    });
    let artist = artist.unwrap_or_else(|| "Unknown".to_string());

    // Save to songs directory with sanitized filename
    let song_path = format!("songs/{}", safe_filename);
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
