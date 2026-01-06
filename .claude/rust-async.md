# Rust Async and Concurrency

## Runtime
- Use `tokio` as the async runtime for the desktop application
- Do not block inside async code
- Use `spawn_blocking` for CPU-bound work

## Structured Concurrency
- Prefer channels (`mpsc`, `oneshot`) over manual thread management
- Use structured concurrency patterns: `join!`, `try_join!`
- Ensure shared types are `Send`/`Sync` where required
- Wrap shared mutable state in `Arc<Mutex/RwLock>` only when necessary

## Cancellation and Timeouts
- Use timeouts and cancellation for all async operations
- Propagate cancellation via `select!` or `tokio::time::timeout`
- Handle graceful shutdown with cancellation tokens

## Example Patterns

```rust
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

// Good: channels for communication
async fn process_audio_stream(mut rx: mpsc::Receiver<AudioChunk>) {
    while let Some(chunk) = rx.recv().await {
        process_chunk(chunk).await;
    }
}

// Good: timeout for external calls
async fn transcribe_with_timeout(audio: &[u8]) -> Result<String> {
    timeout(
        Duration::from_secs(30),
        call_stt_api(audio)
    )
    .await
    .context("STT request timed out")?
}
```

## Observability
- Use `tracing` for structured logs
- Add spans around async/concurrent work to correlate events
- Avoid logging sensitive data (prefer IDs or hashes)

```rust
use tracing::{info, instrument};

#[instrument(skip(audio_data))]
async fn process_meeting_audio(meeting_id: &str, audio_data: &[u8]) -> Result<Transcript> {
    info!("Processing audio for meeting");
    // ... processing logic
}
```
