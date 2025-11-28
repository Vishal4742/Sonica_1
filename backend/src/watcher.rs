use crate::error::{AppError, Result};
use notify::{Watcher, RecommendedWatcher, RecursiveMode, Event, EventKind};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct FileWatcher {
    #[allow(dead_code)]
    watcher: RecommendedWatcher,
    event_rx: mpsc::Receiver<notify::Result<Event>>,
}

impl FileWatcher {
    pub fn new(watch_path: &str, _event_tx: mpsc::Sender<notify::Result<Event>>) -> Result<Self> {
        let (tx, rx) = mpsc::channel(100);
        
        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.try_send(res);
        })
        .map_err(|e| AppError::Io(std::io::Error::other(
            format!("Failed to create watcher: {}", e),
        )))?;

        watcher
            .watch(Path::new(watch_path), RecursiveMode::NonRecursive)
            .map_err(|e| AppError::Io(std::io::Error::other(
                format!("Failed to watch path: {}", e),
            )))?;

        Ok(FileWatcher {
            watcher,
            event_rx: rx,
        })
    }

    pub async fn handle_events<F>(mut self, mut handler: F)
    where
        F: FnMut(String) + Send + 'static,
    {
        while let Some(event_result) = self.event_rx.recv().await {
            match event_result {
                Ok(event) => {
                    if let EventKind::Create(_) | EventKind::Modify(_) = event.kind {
                        for path in event.paths {
                            if is_audio_file(&path) {
                                if let Some(path_str) = path.to_str() {
                                    handler(path_str.to_string());
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Watcher error: {}", e);
                }
            }
        }
    }
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

pub async fn start_watcher(
    watch_path: &str,
    handler: Arc<dyn Fn(String) + Send + Sync>,
) -> Result<()> {
    let (tx, mut rx) = mpsc::channel(100);
    
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.try_send(res);
    })
    .map_err(|e| AppError::Io(std::io::Error::other(
        format!("Failed to create watcher: {}", e),
    )))?;

    watcher
        .watch(Path::new(watch_path), RecursiveMode::NonRecursive)
        .map_err(|e| AppError::Io(std::io::Error::other(
            format!("Failed to watch path: {}", e),
        )))?;

    tokio::spawn(async move {
        while let Some(event_result) = rx.recv().await {
            match event_result {
                Ok(event) => {
                    if let EventKind::Create(_) | EventKind::Modify(_) = event.kind {
                        for path in event.paths {
                            if is_audio_file(&path) {
                                if let Some(path_str) = path.to_str() {
                                    handler(path_str.to_string());
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Watcher error: {}", e);
                }
            }
        }
    });

    Ok(())
}
