use crate::error::{AppError, Result};
use rusqlite::{Connection, params};
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db = Database { conn: Mutex::new(conn) };
        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS songs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT,
                artist TEXT,
                path TEXT UNIQUE,
                fingerprint TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        Ok(())
    }

    pub fn insert_song(
        &self,
        title: &str,
        artist: &str,
        path: &str,
        fingerprint: &[f32],
    ) -> Result<i64> {
        let fingerprint_json = serde_json::to_string(fingerprint)?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO songs (title, artist, path, fingerprint) VALUES (?1, ?2, ?3, ?4)",
            params![title, artist, path, fingerprint_json],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_all_songs(&self) -> Result<Vec<crate::types::SongMetadata>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, artist, path, created_at FROM songs ORDER BY created_at DESC"
        )?;
        
        let songs = stmt.query_map([], |row| {
            Ok(crate::types::SongMetadata {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                path: row.get(3)?,
                created_at: row.get::<_, String>(4)?,
            })
        })?;

        let mut result = Vec::new();
        for song in songs {
            result.push(song?);
        }
        Ok(result)
    }

    pub fn get_song_by_id(&self, id: i64) -> Result<Option<(String, String, String, Vec<f32>)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT title, artist, path, fingerprint FROM songs WHERE id = ?1"
        )?;
        
        let mut rows = stmt.query_map(params![id], |row| {
            let fingerprint_json: String = row.get(3)?;
            let fingerprint: Vec<f32> = serde_json::from_str(&fingerprint_json)
                .map_err(|e| rusqlite::Error::InvalidColumnType(3, "fingerprint".to_string(), rusqlite::types::Type::Text))?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                fingerprint,
            ))
        })?;

        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    pub fn song_exists_by_path(&self, path: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT 1 FROM songs WHERE path = ?1 LIMIT 1")?;
        let exists = stmt.exists(params![path])?;
        Ok(exists)
    }

    pub fn get_all_fingerprints(&self) -> Result<Vec<crate::types::SongFp>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, artist, path, fingerprint FROM songs"
        )?;
        
        let songs = stmt.query_map([], |row| {
            let fingerprint_json: String = row.get(4)?;
            let fingerprint: Vec<f32> = serde_json::from_str(&fingerprint_json)
                .map_err(|e| rusqlite::Error::InvalidColumnType(4, "fingerprint".to_string(), rusqlite::types::Type::Text))?;
            
            Ok(crate::types::SongFp {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                path: row.get(3)?,
                fingerprint,
            })
        })?;

        let mut result = Vec::new();
        for song in songs {
            result.push(song?);
        }
        Ok(result)
    }
}
