use serde::Serialize;

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
