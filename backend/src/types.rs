use once_cell::sync::OnceCell;
use std::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongFp {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub path: String,
    pub fingerprint: Vec<f32>,
}

#[derive(Debug, Deserialize)]
pub struct ClipReq {
    // Audio file will be handled via multipart form
}

#[derive(Debug, Serialize)]
pub struct RecognitionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#match: Option<MatchResult>,
}

#[derive(Debug, Serialize)]
pub struct MatchResult {
    pub title: String,
    pub artist: String,
    pub score: f32,
}

#[derive(Debug, Serialize)]
pub struct SongMetadata {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub path: String,
    pub created_at: String,
}

// Global in-memory cache for fingerprints
pub static CACHE: OnceCell<RwLock<Vec<SongFp>>> = OnceCell::new();
