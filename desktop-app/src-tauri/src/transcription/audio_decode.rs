//! Audio decoding and preprocessing for whisper.cpp.
//!
//! whisper.cpp requires 16 kHz mono f32 samples. This module decodes the
//! supported containers/codecs with symphonia, mixes down to mono, and
//! resamples with linear interpolation.

use std::path::Path;

use symphonia::core::audio::{SampleBuffer, SignalSpec};
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;

use crate::error::{RecapError, Result};

/// Sample rate whisper.cpp operates at.
pub const WHISPER_SAMPLE_RATE: u32 = 16_000;

/// Decode an audio file into 16 kHz mono f32 samples ready for whisper.cpp.
pub fn decode_for_whisper(path: &Path) -> Result<Vec<f32>> {
    if !path.exists() {
        return Err(RecapError::AudioInput(format!(
            "Audio file not found: {}",
            path.display()
        )));
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    if ext == "webm" {
        return Err(RecapError::AudioInput(
            "WebM audio is not supported for local transcription yet. \
             Please convert the recording to WAV, MP3, M4A, OGG, or FLAC."
                .to_string(),
        ));
    }

    let (samples, sample_rate) = decode_to_mono(path)?;
    if samples.is_empty() {
        return Err(RecapError::AudioInput(
            "No audio samples found in file".to_string(),
        ));
    }

    Ok(resample_linear(&samples, sample_rate, WHISPER_SAMPLE_RATE))
}

/// Decode the first audio track of a file to mono f32 at its native rate.
/// Returns `(samples, sample_rate)`.
fn decode_to_mono(path: &Path) -> Result<(Vec<f32>, u32)> {
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probe = symphonia::default::get_probe();
    let probed = probe
        .format(&hint, mss, &Default::default(), &Default::default())
        .map_err(|e| RecapError::AudioInput(format!("Unrecognized audio format: {}", e)))?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| RecapError::AudioInput("No audio track found in file".to_string()))?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.ok_or_else(|| {
        RecapError::AudioInput("Audio track has no sample rate information".to_string())
    })?;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &Default::default())
        .map_err(|e| RecapError::AudioInput(format!("Unsupported audio codec: {}", e)))?;

    let mut samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(e) => {
                return Err(RecapError::AudioInput(format!(
                    "Failed to read audio packet: {}",
                    e
                )));
            }
        };
        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec: SignalSpec = *decoded.spec();
                let channels = spec.channels.count().max(1);
                let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
                sample_buf.copy_interleaved_ref(decoded);

                // Mix down to mono.
                for frame in sample_buf.samples().chunks(channels) {
                    let sum: f32 = frame.iter().sum();
                    samples.push(sum / channels as f32);
                }
            }
            // Skip malformed packets rather than failing the whole file.
            Err(_) => continue,
        }
    }

    Ok((samples, sample_rate))
}

/// Linear-interpolation resampler. Good enough for speech pipelines; a
/// higher-quality sinc resampler is a future improvement.
pub fn resample_linear(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if input.is_empty() || from_rate == 0 || to_rate == 0 {
        return Vec::new();
    }
    if from_rate == to_rate {
        return input.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (input.len() as f64 / ratio) as usize;

    (0..out_len)
        .map(|i| {
            let src = i as f64 * ratio;
            let idx = src as usize;
            let frac = (src - idx as f64) as f32;
            let a = input[idx];
            let b = input.get(idx + 1).copied().unwrap_or(a);
            a + (b - a) * frac
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal 16-bit PCM WAV in memory.
    fn build_wav(sample_rate: u32, samples: &[i16]) -> Vec<u8> {
        let byte_rate = sample_rate * 2 * 1; // mono, 16-bit
        let data_size = (samples.len() * 2) as u32;
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_size).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
        out.extend_from_slice(&1u16.to_le_bytes()); // PCM
        out.extend_from_slice(&1u16.to_le_bytes()); // mono
        out.extend_from_slice(&sample_rate.to_le_bytes());
        out.extend_from_slice(&byte_rate.to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes()); // block align
        out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_size.to_le_bytes());
        for s in samples {
            out.extend_from_slice(&s.to_le_bytes());
        }
        out
    }

    fn write_temp_wav(sample_rate: u32, samples: &[i16]) -> tempfile::NamedTempFile {
        let mut file = tempfile::Builder::new().suffix(".wav").tempfile().unwrap();
        use std::io::Write;
        file.write_all(&build_wav(sample_rate, samples)).unwrap();
        file
    }

    #[test]
    fn test_resample_linear_identity() {
        let input = vec![0.0_f32, 1.0, 2.0, 3.0];
        assert_eq!(resample_linear(&input, 16000, 16000), input);
        assert!(resample_linear(&[], 8000, 16000).is_empty());
    }

    #[test]
    fn test_resample_linear_doubles_length() {
        // Constant signal stays constant under resampling.
        let input = vec![0.5_f32; 100];
        let out = resample_linear(&input, 8000, 16000);
        assert_eq!(out.len(), 200);
        assert!(out.iter().all(|s| (*s - 0.5).abs() < 1e-6));
    }

    #[test]
    fn test_decode_wav_and_resample_to_whisper_input() {
        // 0.1 s of a low sine at 8 kHz.
        let samples: Vec<i16> = (0..800)
            .map(|i| ((i as f32 * 0.05).sin() * 10_000.0) as i16)
            .collect();
        let file = write_temp_wav(8000, &samples);

        let out = decode_for_whisper(file.path()).unwrap();
        // 0.1 s at 16 kHz.
        assert_eq!(out.len(), 1600);
        // Signal must be normalized-ish, not garbage.
        let peak = out.iter().fold(0.0_f32, |a, b| a.max(b.abs()));
        assert!(peak > 0.1 && peak <= 1.0);
    }

    #[test]
    fn test_decode_missing_file() {
        let err = decode_for_whisper(Path::new("/nonexistent/file.wav")).unwrap_err();
        assert!(matches!(err, crate::error::RecapError::AudioInput(_)));
    }
}
