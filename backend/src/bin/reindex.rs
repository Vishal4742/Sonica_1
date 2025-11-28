use sonica_backend::error::Result;
use sonica_backend::fingerprint::{decode_audio, generate_fingerprints, preprocess_audio};
use sonica_backend::storage::Database;
use std::path::Path;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<()> {
    let db = Database::new("songs.db")?;

    println!("🧹 Clearing database...");
    db.clear_database()?;

    let songs_dir = "songs";
    let mut entries = fs::read_dir(songs_dir).await?;

    println!("🚀 Starting re-indexing...");

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            let filename = path.file_name().unwrap().to_string_lossy().to_string();
            let path_str = format!("songs/{}", filename);

            println!("Processing: {}", filename);

            // Preprocess
            let temp_output = format!("temp/{}_reindex.wav", uuid::Uuid::new_v4());
            fs::create_dir_all("temp").await?;

            if let Err(e) = preprocess_audio(&path_str, &temp_output) {
                eprintln!("  ❌ Preprocess failed: {}", e);
                continue;
            }

            // Fingerprint
            let samples = match decode_audio(&temp_output) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("  ❌ Decode failed: {}", e);
                    let _ = fs::remove_file(&temp_output).await;
                    continue;
                }
            };

            let fingerprints = generate_fingerprints(&samples);
            let _ = fs::remove_file(&temp_output).await;

            // Insert
            let title = Path::new(&filename)
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();
            let artist = "Unknown"; // Simple default for re-indexing

            match db.insert_song(&title, artist, &path_str, &fingerprints) {
                Ok(_) => println!("  ✅ Indexed {} hashes", fingerprints.len()),
                Err(e) => eprintln!("  ❌ Insert failed: {}", e),
            }
        }
    }

    println!("✨ Re-indexing complete!");
    Ok(())
}
