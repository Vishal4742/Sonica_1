use crate::error::{AppError, Result};
use rustfft::{num_complex::Complex, FftPlanner};
use std::path::Path;
use std::process::Command;
use symphonia::core::audio::Signal;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use symphonia::default::get_codecs;
use symphonia::default::get_probe;

const SAMPLE_RATE: u32 = 16000;
const WINDOW_SIZE: usize = 4096; // Better frequency resolution
const HOP_SIZE: usize = 2048; // 50% overlap

/// Preprocess audio using FFmpeg (convert to 16kHz mono WAV)
pub fn preprocess_audio(input_path: &str, output_path: &str) -> Result<()> {
    let ffmpeg_cmd = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };

    let output = Command::new(ffmpeg_cmd)
        .args([
            "-i",
            input_path,
            "-ar",
            &SAMPLE_RATE.to_string(),
            "-ac",
            "1",
            "-f",
            "wav",
            "-y",
            output_path,
        ])
        .output()
        .map_err(|e| AppError::Ffmpeg(format!("Failed to execute FFmpeg: {}", e)))?;

    if !output.status.success() {
        return Err(AppError::Ffmpeg(format!(
            "FFmpeg failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(())
}

/// Decode audio file to float samples
pub fn decode_audio(path: &str) -> Result<Vec<f32>> {
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension() {
        hint.with_extension(&ext.to_string_lossy());
    }

    let probe = get_probe();
    let mut probed = probe
        .format(&hint, mss, &Default::default(), &Default::default())
        .map_err(|e| AppError::Audio(format!("Failed to probe audio: {}", e)))?;

    let track = probed
        .format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| AppError::Audio("No audio track found".to_string()))?;

    let track_id = track.id;
    let codec_params = &track.codec_params;
    let mut decoder = get_codecs()
        .make(codec_params, &Default::default())
        .map_err(|e| AppError::Audio(format!("Failed to create decoder: {}", e)))?;

    let mut samples = Vec::new();
    let track = &mut probed.format;

    loop {
        let packet = match track.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(_) => break,
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                use symphonia::core::audio::AudioBufferRef;
                match decoded {
                    AudioBufferRef::F32(buf) => samples.extend(buf.chan(0).iter().cloned()),
                    AudioBufferRef::U8(buf) => {
                        samples.extend(buf.chan(0).iter().map(|&s| s as f32 / 128.0 - 1.0))
                    }
                    AudioBufferRef::S16(buf) => {
                        samples.extend(buf.chan(0).iter().map(|&s| s as f32 / 32768.0))
                    }
                    _ => continue, // Handle other formats if needed
                }
            }
            Err(_) => continue,
        }
    }

    if samples.is_empty() {
        return Err(AppError::Audio("No audio samples decoded".to_string()));
    }

    Ok(samples)
}

/// Generate spectrogram (STFT)
fn spectrogram(samples: &[f32]) -> Vec<Vec<f32>> {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(WINDOW_SIZE);

    if samples.len() < WINDOW_SIZE {
        return Vec::new();
    }

    let num_frames = (samples.len() - WINDOW_SIZE) / HOP_SIZE;
    let mut spectrogram = Vec::with_capacity(num_frames);

    // Hanning window
    let window: Vec<f32> = (0..WINDOW_SIZE)
        .map(|i| {
            0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (WINDOW_SIZE - 1) as f32).cos())
        })
        .collect();

    for i in 0..num_frames {
        let start = i * HOP_SIZE;
        let end = start + WINDOW_SIZE;
        let chunk = &samples[start..end];

        let mut buffer: Vec<Complex<f32>> = chunk
            .iter()
            .zip(&window)
            .map(|(&s, &w)| Complex::new(s * w, 0.0))
            .collect();

        fft.process(&mut buffer);

        // Keep magnitude of first half (Nyquist)
        let magnitude: Vec<f32> = buffer[0..WINDOW_SIZE / 2]
            .iter()
            .map(|c| c.norm())
            .collect();

        spectrogram.push(magnitude);
    }

    spectrogram
}

/// Find peaks in spectrogram (Constellation Map)
fn find_peaks(spectrogram: &[Vec<f32>]) -> Vec<(usize, usize)> {
    let rows = spectrogram.len();
    if rows == 0 {
        return Vec::new();
    }
    let cols = spectrogram[0].len();
    let mut peaks = Vec::new();

    // Divide into frequency bands to ensure peaks across spectrum
    // e.g., Low, Mid, High
    let bands = [(0, 50), (50, 200), (200, 500), (500, cols)];

    for (start_bin, end_bin) in bands {
        for t in 0..rows {
            let mut max_val = 0.0;
            let mut max_freq = 0;

            // Find max in this band for this time frame
            for f in start_bin..end_bin.min(cols) {
                if spectrogram[t][f] > max_val {
                    max_val = spectrogram[t][f];
                    max_freq = f;
                }
            }

            // Simple local maximum check (time axis)
            // Check if it's a peak compared to neighbors
            if max_val > 1.0 {
                // Noise threshold
                let mut is_peak = true;
                // Check +/- 2 frames
                for dt in 1..=2 {
                    if t >= dt && spectrogram[t - dt][max_freq] > max_val {
                        is_peak = false;
                        break;
                    }
                    if t + dt < rows && spectrogram[t + dt][max_freq] > max_val {
                        is_peak = false;
                        break;
                    }
                }

                if is_peak {
                    peaks.push((t, max_freq));
                }
            }
        }
    }

    peaks
}

/// Generate hashes from peaks (Combinatorial Hashing)
/// Returns: (hash, time_offset)
pub fn generate_fingerprints(samples: &[f32]) -> Vec<(u32, u32)> {
    let spec = spectrogram(samples);
    let mut peaks = find_peaks(&spec);
    // Sort peaks by time (t) to ensure t2 > t1 in the loop
    peaks.sort_by_key(|k| k.0);

    let mut fingerprints = Vec::new();

    // Target zone: look ahead in time
    let target_zone_start = 5; // frames ahead
    let target_zone_end = 50; // frames ahead

    for i in 0..peaks.len() {
        let (t1, f1) = peaks[i];

        for j in (i + 1)..peaks.len() {
            let (t2, f2) = peaks[j];
            let dt = t2 - t1;

            if dt < target_zone_start {
                continue;
            }
            if dt > target_zone_end {
                break;
            } // Peaks are sorted by time usually

            // Hash: [f1: 9 bits] [f2: 9 bits] [dt: 14 bits]
            // f1, f2 are bin indices. WINDOW=4096, bins=2048.
            // We care mostly about lower 512 bins (0-2kHz) where music energy is.
            // Let's mask to 9 bits (511).

            if f1 < 512 && f2 < 512 {
                let hash = ((f1 as u32) << 23) | ((f2 as u32) << 14) | (dt as u32);
                fingerprints.push((hash, t1 as u32));
            }
        }
    }

    fingerprints
}
