//! Audio resampling and sample processing

use super::types::AudioChunk;
use super::TARGET_SAMPLE_RATE;
use rubato::{Resampler, SincFixedIn};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{error, warn};

/// Chunk size in samples (0.1 seconds of audio at 16kHz = 1600 samples)
pub(crate) const CHUNK_SIZE: usize = 1600;

/// Process incoming audio samples: convert to mono, optionally resample, buffer, and send chunks
pub(crate) fn process_samples(
    data: &[i16],
    channels: usize,
    input_buffer: &Arc<Mutex<Vec<i16>>>,
    input_chunk_size: usize,
    output_buffer: &Arc<Mutex<Vec<i16>>>,
    sender: &mpsc::Sender<AudioChunk>,
    resampler: &Option<Arc<Mutex<SincFixedIn<f32>>>>,
) {
    // Convert to mono by averaging channels
    let mono_samples: Vec<i16> = if channels > 1 {
        data.chunks(channels)
            .map(|frame| {
                let sum: i32 = frame.iter().map(|&s| s as i32).sum();
                (sum / channels as i32) as i16
            })
            .collect()
    } else {
        data.to_vec()
    };

    // Handle resampling if configured
    if let Some(resampler_arc) = resampler {
        process_with_resampler(
            &mono_samples,
            input_buffer,
            input_chunk_size,
            output_buffer,
            sender,
            resampler_arc,
        );
    } else {
        // No resampling needed - direct buffering
        process_direct(&mono_samples, output_buffer, sender);
    }
}

/// Process samples with resampling
fn process_with_resampler(
    mono_samples: &[i16],
    input_buffer: &Arc<Mutex<Vec<i16>>>,
    input_chunk_size: usize,
    output_buffer: &Arc<Mutex<Vec<i16>>>,
    sender: &mpsc::Sender<AudioChunk>,
    resampler_arc: &Arc<Mutex<SincFixedIn<f32>>>,
) {
    // Add to input buffer
    if let Ok(mut input_buf) = input_buffer.lock() {
        input_buf.extend(mono_samples);

        // Process complete chunks through the resampler
        while input_buf.len() >= input_chunk_size {
            let input_chunk: Vec<i16> = input_buf.drain(..input_chunk_size).collect();

            // Convert i16 to f32 for resampling
            let input_f32: Vec<f32> = input_chunk.iter().map(|&s| s as f32 / 32768.0).collect();

            // Resample
            if let Ok(mut resampler) = resampler_arc.lock() {
                match resampler.process(&[input_f32], None) {
                    Ok(resampled) => {
                        // Convert back to i16
                        let output_i16: Vec<i16> = resampled[0]
                            .iter()
                            .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                            .collect();

                        // Add to output buffer
                        if let Ok(mut output_buf) = output_buffer.lock() {
                            output_buf.extend(&output_i16);
                        }
                    }
                    Err(e) => {
                        error!("Resampling error: {}", e);
                    }
                }
            }
        }
    }

    // Send chunks from output buffer
    send_chunks(output_buffer, sender);
}

/// Process samples directly without resampling
fn process_direct(
    mono_samples: &[i16],
    output_buffer: &Arc<Mutex<Vec<i16>>>,
    sender: &mpsc::Sender<AudioChunk>,
) {
    if let Ok(mut output_buf) = output_buffer.lock() {
        output_buf.extend(mono_samples);

        while output_buf.len() >= CHUNK_SIZE {
            let chunk: Vec<i16> = output_buf.drain(..CHUNK_SIZE).collect();
            let audio_chunk = AudioChunk {
                samples: chunk,
                sample_rate: TARGET_SAMPLE_RATE, // Should already be 16kHz
            };
            // Use try_send to avoid blocking the audio callback
            match sender.try_send(audio_chunk) {
                Ok(_) => {}
                Err(e) => {
                    warn!("Audio buffer overflow - chunk dropped: {}", e);
                    return;
                }
            }
        }
    }
}

/// Send complete chunks from the output buffer
fn send_chunks(output_buffer: &Arc<Mutex<Vec<i16>>>, sender: &mpsc::Sender<AudioChunk>) {
    if let Ok(mut output_buf) = output_buffer.lock() {
        while output_buf.len() >= CHUNK_SIZE {
            let chunk: Vec<i16> = output_buf.drain(..CHUNK_SIZE).collect();
            let audio_chunk = AudioChunk {
                samples: chunk,
                sample_rate: TARGET_SAMPLE_RATE,
            };
            // Use try_send to avoid blocking the audio callback
            match sender.try_send(audio_chunk) {
                Ok(_) => {}
                Err(e) => {
                    warn!("Audio buffer overflow - chunk dropped: {}", e);
                    return;
                }
            }
        }
    }
}
