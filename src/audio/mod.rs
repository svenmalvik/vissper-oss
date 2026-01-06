//! Audio capture module using cpal for cross-platform microphone access
//!
//! Captures audio from the default input device at the specified sample rate
//! in mono PCM format, optimal for realtime transcription services.

mod resampler;
mod types;

pub use types::{AudioCaptureError, AudioCaptureHandle, AudioChunk};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use resampler::{process_samples, CHUNK_SIZE};
use rubato::{SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Target sample rate for Azure OpenAI STT (16kHz)
pub const AZURE_SAMPLE_RATE: u32 = 16000;

/// Target sample rate for OpenAI STT (24kHz)
pub const OPENAI_SAMPLE_RATE: u32 = 24000;

/// Default target sample rate (Azure, for backward compatibility)
pub const TARGET_SAMPLE_RATE: u32 = AZURE_SAMPLE_RATE;

/// Start audio capture on a dedicated thread with default sample rate (16kHz for Azure)
///
/// Initializes the default audio input device and begins capturing microphone audio.
/// Audio is resampled to the target sample rate in mono PCM format.
///
/// # Returns
/// A tuple containing:
/// - `AudioCaptureHandle` - Used to stop capture and check status
/// - `mpsc::Receiver<AudioChunk>` - Receives audio chunks for streaming to STT
///
/// # Errors
/// Returns `AudioCaptureError` if:
/// - No audio input device is available
/// - The audio device configuration is not supported
/// - The audio stream cannot be started
///
/// # Example
/// ```ignore
/// let (mut handle, audio_rx) = start_capture()?;
/// // Process audio chunks from audio_rx
/// handle.stop(); // Stop when done
/// ```
#[allow(dead_code)]
pub(crate) fn start_capture(
) -> Result<(AudioCaptureHandle, mpsc::Receiver<AudioChunk>), AudioCaptureError> {
    start_capture_with_sample_rate(TARGET_SAMPLE_RATE)
}

/// Start audio capture on a dedicated thread with specified sample rate
///
/// Initializes the default audio input device and begins capturing microphone audio.
/// Audio is resampled to the specified target sample rate in mono PCM format.
///
/// # Arguments
/// - `target_sample_rate` - Target sample rate in Hz (e.g., 16000 for Azure, 24000 for OpenAI)
///
/// # Returns
/// A tuple containing:
/// - `AudioCaptureHandle` - Used to stop capture and check status
/// - `mpsc::Receiver<AudioChunk>` - Receives audio chunks for streaming to STT
pub(crate) fn start_capture_with_sample_rate(
    target_sample_rate: u32,
) -> Result<(AudioCaptureHandle, mpsc::Receiver<AudioChunk>), AudioCaptureError> {
    let is_capturing = Arc::new(AtomicBool::new(true));
    let is_capturing_clone = is_capturing.clone();

    // Create async channel for audio chunks
    let (chunk_tx, chunk_rx) = mpsc::channel(600);

    let thread_handle = thread::spawn(move || {
        if let Err(e) = run_capture(is_capturing_clone, chunk_tx, target_sample_rate) {
            error!("Audio capture error: {}", e);
        }
    });

    let handle = AudioCaptureHandle {
        is_capturing,
        thread_handle: Some(thread_handle),
    };

    Ok((handle, chunk_rx))
}

/// Run audio capture on the current thread (blocking)
fn run_capture(
    is_capturing: Arc<AtomicBool>,
    chunk_tx: mpsc::Sender<AudioChunk>,
    target_sample_rate: u32,
) -> Result<(), AudioCaptureError> {
    let host = cpal::default_host();

    let device = host
        .default_input_device()
        .ok_or(AudioCaptureError::NoInputDevice)?;

    let device_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
    info!("Using audio input device: {}", device_name);

    // Get supported configs and find one closest to our target
    let supported_configs = device
        .supported_input_configs()
        .map_err(|e| AudioCaptureError::ConfigError(e.to_string()))?;

    // Try to find a config with target rate, or fall back to any supported rate
    let mut best_config = None;
    let mut found_target_rate = false;

    for config in supported_configs {
        let channels = config.channels();
        if channels > 0 {
            if config.min_sample_rate().0 <= target_sample_rate
                && config.max_sample_rate().0 >= target_sample_rate
            {
                best_config = Some(config.with_sample_rate(cpal::SampleRate(target_sample_rate)));
                found_target_rate = true;
                break;
            } else if best_config.is_none() {
                best_config = Some(config.with_max_sample_rate());
            }
        }
    }

    let supported_config = best_config.ok_or(AudioCaptureError::NoSupportedConfig)?;

    if !found_target_rate {
        warn!(
            "{}Hz not supported, using {}Hz instead",
            target_sample_rate,
            supported_config.sample_rate().0
        );
    }

    let config: cpal::StreamConfig = supported_config.into();
    let sample_rate = config.sample_rate.0;
    let channels = config.channels as usize;

    info!("Audio config: {} channels, {} Hz", channels, sample_rate);

    // Create resampler if sample rate doesn't match target
    let (resampler, input_chunk_size): (Option<Arc<Mutex<SincFixedIn<f32>>>>, usize) =
        if sample_rate != target_sample_rate {
            info!(
                "Creating resampler: {} Hz -> {} Hz",
                sample_rate, target_sample_rate
            );
            let params = SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Linear,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris2,
            };
            // Calculate chunk size that will produce target sample rate chunks
            let input_frames = (CHUNK_SIZE as f64 * sample_rate as f64 / target_sample_rate as f64)
                .ceil() as usize;
            match SincFixedIn::<f32>::new(
                target_sample_rate as f64 / sample_rate as f64,
                2.0,
                params,
                input_frames,
                1, // mono
            ) {
                Ok(resampler) => {
                    info!(
                        "Resampler configured: input {} samples -> output {} samples",
                        input_frames, CHUNK_SIZE
                    );
                    (Some(Arc::new(Mutex::new(resampler))), input_frames)
                }
                Err(e) => {
                    error!("Failed to create resampler: {}", e);
                    (None, CHUNK_SIZE)
                }
            }
        } else {
            (None, CHUNK_SIZE)
        };

    // Buffer for accumulating resampled output samples (after resampling)
    let output_buffer: Arc<Mutex<Vec<i16>>> =
        Arc::new(Mutex::new(Vec::with_capacity(CHUNK_SIZE * 2)));
    let output_buffer_clone = output_buffer.clone();

    // Buffer for accumulating input samples (before resampling)
    let input_buffer: Arc<Mutex<Vec<i16>>> =
        Arc::new(Mutex::new(Vec::with_capacity(input_chunk_size * 2)));
    let input_buffer_clone = input_buffer.clone();

    let resampler_clone = resampler.clone();

    let is_capturing_stream = is_capturing.clone();
    let chunk_tx_clone = chunk_tx.clone();

    let err_callback = |err| {
        error!("Audio stream error: {}", err);
    };

    // Build the input stream based on sample format
    let stream = match device.default_input_config()?.sample_format() {
        SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _| {
                if !is_capturing_stream.load(Ordering::SeqCst) {
                    return;
                }
                process_samples(
                    data,
                    channels,
                    &input_buffer_clone,
                    input_chunk_size,
                    &output_buffer_clone,
                    &chunk_tx_clone,
                    &resampler_clone,
                );
            },
            err_callback,
            None,
        )?,
        SampleFormat::F32 => {
            let is_capturing_f32 = is_capturing.clone();
            let input_buffer_f32 = input_buffer.clone();
            let output_buffer_f32 = output_buffer.clone();
            let chunk_tx_f32 = chunk_tx.clone();
            let resampler_f32 = resampler.clone();
            device.build_input_stream(
                &config,
                move |data: &[f32], _| {
                    if !is_capturing_f32.load(Ordering::SeqCst) {
                        return;
                    }
                    // Convert f32 to i16
                    let samples: Vec<i16> = data
                        .iter()
                        .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                        .collect();
                    process_samples(
                        &samples,
                        channels,
                        &input_buffer_f32,
                        input_chunk_size,
                        &output_buffer_f32,
                        &chunk_tx_f32,
                        &resampler_f32,
                    );
                },
                err_callback,
                None,
            )?
        }
        sample_format => {
            return Err(AudioCaptureError::UnsupportedFormat(format!(
                "{:?}",
                sample_format
            )));
        }
    };

    stream.play()?;
    info!("Audio capture started");

    // Keep the stream alive until capture is stopped
    while is_capturing.load(Ordering::SeqCst) {
        thread::sleep(std::time::Duration::from_millis(100));
    }

    drop(stream);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_capture_creation() {
        // This test will only pass on machines with audio input
        match start_capture() {
            Ok((handle, _rx)) => {
                assert!(handle.is_capturing());
                println!("Audio capture started successfully");
                // Clean up
                drop(handle);
            }
            Err(AudioCaptureError::NoInputDevice) => {
                println!("No audio input device available (expected in CI)");
            }
            Err(e) => {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}
