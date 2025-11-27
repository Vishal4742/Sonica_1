use crate::error::Result;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db = Database {
            conn: Mutex::new(conn),
        };
        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Songs table (Metadata)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS songs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT,
                artist TEXT,
                path TEXT UNIQUE,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Fingerprints table (Hashes)
        // hash: 32-bit integer (freq + time delta)
        // song_id: Foreign key
        // offset: Absolute time offset in the song
        conn.execute(
            "CREATE TABLE IF NOT EXISTS fingerprints (
                hash INTEGER NOT NULL,
                song_id INTEGER NOT NULL,
                offset INTEGER NOT NULL,
                FOREIGN KEY(song_id) REFERENCES songs(id)
            )",
            [],
        )?;

        // Index for fast lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_fingerprints_hash ON fingerprints(hash)",
            [],
        )?;

        Ok(())
    }

    pub fn insert_song(
        &self,
        title: &str,
        artist: &str,
        path: &str,
        fingerprints: &[(u32, u32)], // (hash, offset)
    ) -> Result<i64> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        // 1. Insert Song Metadata
        tx.execute(
            "INSERT INTO songs (title, artist, path) VALUES (?1, ?2, ?3)",
            params![title, artist, path],
        )?;
        let song_id = tx.last_insert_rowid();

        // 2. Batch Insert Fingerprints
        // Prepare statement for performance
        {
            let mut stmt =
                tx.prepare("INSERT INTO fingerprints (hash, song_id, offset) VALUES (?1, ?2, ?3)")?;

            for (hash, offset) in fingerprints {
                stmt.execute(params![hash, song_id, offset])?;
            }
        }

        tx.commit()?;
        Ok(song_id)
    }

    pub fn get_all_songs(&self) -> Result<Vec<crate::types::SongMetadata>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, artist, path, created_at FROM songs ORDER BY created_at DESC",
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

    pub fn song_exists_by_path(&self, path: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT 1 FROM songs WHERE path = ?1 LIMIT 1")?;
        let exists = stmt.exists(params![path])?;
        Ok(exists)
    }

    /// Find matching fingerprints in the database
    /// Returns a map of song_id -> list of (db_offset, query_offset)
    pub fn find_matches(
        &self,
        query_hashes: &[(u32, u32)],
    ) -> Result<HashMap<i64, Vec<(u32, u32)>>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT song_id, offset FROM fingerprints WHERE hash = ?1")?;

        let mut matches: HashMap<i64, Vec<(u32, u32)>> = HashMap::new();

        for (hash, query_offset) in query_hashes {
            let rows = stmt.query_map(params![hash], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, u32>(1)?))
            });

            if let Ok(rows) = rows {
                for row in rows {
                    if let Ok((song_id, db_offset)) = row {
                        matches
                            .entry(song_id)
                            .or_default()
                            .push((db_offset, *query_offset));
                    }
                }
            }
        }

        Ok(matches)
    }

    pub fn get_song_metadata(&self, song_id: i64) -> Result<Option<crate::types::SongMetadata>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, title, artist, path, created_at FROM songs WHERE id = ?1")?;

        let mut rows = stmt.query_map(params![song_id], |row| {
            Ok(crate::types::SongMetadata {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                path: row.get(3)?,
                created_at: row.get::<_, String>(4)?,
            })
        })?;

        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    // Helper to clear database for re-indexing
    pub fn clear_database(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM fingerprints", [])?;
        conn.execute("DELETE FROM songs", [])?;
        Ok(())
    }
}
