use crate::error::{AppError, Result};
use std::path::Path;
use std::process::Command;
use symphonia::core::audio::AudioBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::get_codecs;
use symphonia::default::get_probe;

const SAMPLE_RATE: u32 = 16000;
const MFCC_COEFFS: usize = 13;
const WINDOW_SIZE: usize = 512;
const HOP_SIZE: usize = 256;

pub fn preprocess_audio(input_path: &str, output_path: &str) -> Result<()> {
    let output = Command::new("ffmpeg")
        .args(&[
            "-i", input_path,
            "-ar", "16000",
            "-ac", "1",
            "-f", "wav",
            "-y",
            output_path,
        ])
        .output()
        .map_err(|e| AppError::Ffmpeg(format!("Failed to execute FFmpeg: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Ffmpeg(format!("FFmpeg failed: {}", stderr)));
    }

    Ok(())
}

pub fn extract_mfcc(audio_path: &str) -> Result<Vec<f32>> {
    // Decode audio file
    let audio_samples = decode_audio(audio_path)?;

    // Compute MFCC features
    let mfcc_features = compute_mfcc(&audio_samples)?;

    Ok(mfcc_features)
}

fn decode_audio(path: &str) -> Result<Vec<f32>> {
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    
    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension() {
        hint.with_extension(&ext.to_string_lossy());
    }

    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();
    let probe = get_probe();

    let mut probed = probe
        .format(&hint, mss, &fmt_opts, &meta_opts)
        .map_err(|e| AppError::Audio(format!("Failed to probe audio: {}", e)))?;

    let track = probed
        .format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| AppError::Audio("No audio track found".to_string()))?;

    let track_id = track.id;
    let codec_params = &track.codec_params;

    let dec_opts: DecoderOptions = Default::default();
    let codecs = get_codecs();
    let mut decoder = codecs
        .make(&codec_params, &dec_opts)
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
                let spec = *decoded.spec();
                let duration = decoded.capacity() as u64;

                // Convert to f32 samples
                let mut buffer = AudioBuffer::<f32>::new(duration, spec);
                buffer.convert(decoded);

                for sample in buffer.planes().planes()[0].iter() {
                    samples.push(*sample);
                }
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(AppError::Audio(format!("Decode error: {}", e))),
        }
    }

    // Resample to 16kHz if needed
    if samples.is_empty() {
        return Err(AppError::Audio("No audio samples decoded".to_string()));
    }

    Ok(samples)
}

fn compute_mfcc(samples: &[f32]) -> Result<Vec<f32>> {
    // Simple spectral feature extraction (simplified MFCC-like features)
    // For MVP: compute average spectral energy across frequency bands
    
    let num_windows = (samples.len() as f32 / HOP_SIZE as f32).ceil() as usize;
    let mut features = Vec::with_capacity(MFCC_COEFFS);

    for i in 0..num_windows {
        let start = i * HOP_SIZE;
        let end = (start + WINDOW_SIZE).min(samples.len());
        
        if end <= start {
            break;
        }

        let window = &samples[start..end];
        
        // Apply window function (Hamming)
        let windowed: Vec<f32> = window
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                let hamming = 0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / (WINDOW_SIZE - 1) as f32).cos();
                s * hamming
            })
            .collect();

        // Compute power spectrum (simplified FFT)
        let fft_size = WINDOW_SIZE;
        let mut power = vec![0.0f32; fft_size / 2];
        
        for k in 0..fft_size / 2 {
            let mut real = 0.0;
            let mut imag = 0.0;
            
            for n in 0..windowed.len() {
                let angle = -2.0 * std::f32::consts::PI * k as f32 * n as f32 / fft_size as f32;
                real += windowed[n] * angle.cos();
                imag += windowed[n] * angle.sin();
            }
            
            power[k] = real * real + imag * imag;
        }

        // Group into MFCC_COEFFS bands (mel-scale approximation)
        let mut mfcc_frame = vec![0.0f32; MFCC_COEFFS];
        let bands_per_coeff = (power.len() / MFCC_COEFFS).max(1);
        
        for (i, mfcc) in mfcc_frame.iter_mut().enumerate() {
            let band_start = i * bands_per_coeff;
            let band_end = ((i + 1) * bands_per_coeff).min(power.len());
            
            let energy: f32 = power[band_start..band_end].iter().sum();
            *mfcc = energy.ln().max(0.0);
        }

        features.extend(mfcc_frame);
    }

    // Average across all frames to get a single feature vector
    if features.is_empty() {
        return Err(AppError::Fingerprint("No features extracted".to_string()));
    }

    let mut avg_features = vec![0.0f32; MFCC_COEFFS];
    let num_frames = features.len() / MFCC_COEFFS;
    
    for i in 0..MFCC_COEFFS {
        let sum: f32 = (0..num_frames)
            .map(|f| features[f * MFCC_COEFFS + i])
            .sum();
        avg_features[i] = sum / num_frames as f32;
    }

    Ok(avg_features)
}

pub fn normalize_vector(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.iter().map(|x| x / norm).collect()
    } else {
        v.to_vec()
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}

pub fn find_best_match(
    query_fp: &[f32],
    cache: &[crate::types::SongFp],
) -> Option<(crate::types::MatchResult, usize)> {
    use rayon::prelude::*;

    let similarities: Vec<(f32, usize)> = cache
        .par_iter()
        .enumerate()
        .map(|(idx, song)| {
            let similarity = cosine_similarity(query_fp, &song.fingerprint);
            (similarity, idx)
        })
        .collect();

    let (best_score, best_idx) = similarities
        .into_iter()
        .max_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))?;

    if best_score > 0.7 {
        // Threshold for match
        let song = &cache[best_idx];
        Some((
            crate::types::MatchResult {
                title: song.title.clone(),
                artist: song.artist.clone(),
                score: best_score,
            },
            best_idx,
        ))
    } else {
        None
    }
}
