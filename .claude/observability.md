# Observability and Logging

## Rust - Structured Logging

### Use `tracing` crate
- Add spans around async/concurrent work to correlate events
- Use appropriate log levels:
  - `error`: user-facing failures that require attention
  - `warn`: degraded states or recoverable issues
  - `info`: significant state changes or milestones
  - `debug`: detailed diagnostic information
  - `trace`: very verbose, low-level details

```rust
use tracing::{info, error, instrument, span, Level};

#[instrument(skip(audio_data), fields(size = audio_data.len()))]
async fn transcribe_audio(meeting_id: &str, audio_data: &[u8]) -> Result<String> {
    info!("Starting transcription for meeting {}", meeting_id);
    
    let result = call_stt_api(audio_data).await;
    
    match result {
        Ok(transcript) => {
            info!("Transcription completed successfully");
            Ok(transcript)
        }
        Err(e) => {
            error!("Transcription failed: {:?}", e);
            Err(e)
        }
    }
}
```

### Avoid Logging Sensitive Data
- Don't log audio data, transcripts, or user PII
- Use IDs or hashes for correlation
- Redact sensitive fields

```rust
// Bad: logging sensitive data
info!("User email: {}, password: {}", email, password);

// Good: logging IDs only
info!(user_id = %user_id, "User authenticated");
```

## TypeScript/Deno - Console Logging

### Use structured logging
- Keep logs JSON-friendly for cloud aggregation
- Include context (user_id, request_id, timestamp)
- Use console methods appropriately:
  - `console.error`: for errors
  - `console.warn`: for warnings
  - `console.info` or `console.log`: for informational messages

```typescript
// Good: structured logging with context
console.info({
  event: 'transcript_processed',
  userId: user.id,
  transcriptLength: transcript.length,
  tokensUsed: response.usage.total_tokens,
  timestamp: new Date().toISOString()
});

// Good: error logging with details
console.error({
  error: error.message,
  userId: user.id,
  function: 'process-transcript',
  stack: error.stack,
  timestamp: new Date().toISOString()
});
```

### Avoid
- Don't log request/response bodies containing sensitive data
- Don't log API keys or tokens
- Don't log excessive details in production

## Metrics and Performance

### Track Key Metrics
- Transcription latency and throughput
- Token usage per user and per request
- API call success/failure rates
- Memory usage for audio buffers
- Subscription status changes

```rust
// Example: timing critical operations
let start = std::time::Instant::now();
let result = expensive_operation().await?;
let duration = start.elapsed();

info!(
    operation = "transcription",
    duration_ms = duration.as_millis(),
    "Operation completed"
);
```

## Error Context
- Log errors once at the boundary (main, HTTP handler)
- Include sufficient context for debugging
- Avoid duplicate logging in error propagation chains

```rust
// Good: log at boundary with full context
match process_meeting(meeting_id).await {
    Ok(_) => info!(meeting_id = %meeting_id, "Meeting processed successfully"),
    Err(e) => error!(
        meeting_id = %meeting_id,
        error = %e,
        "Failed to process meeting"
    ),
}
```

## Privacy-Focused Logging
- Default to NOT logging transcript content
- Log metadata only (lengths, timestamps, IDs)
- Provide opt-in verbose logging for debugging only
- Clear logs containing sensitive data appropriately
