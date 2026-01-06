# Security Best Practices

## General Principles
- Validate all external inputs; parse with strict types
- Reject on ambiguity; fail fast for invalid data
- Never log or debug sensitive data (credentials, tokens, PII)
- Keep dependencies minimal; prefer well-maintained packages
- Audit dependencies regularly

## Secrets Management

### Rust/Desktop App
- Never embed API keys or secrets in the client application
- All LLM API calls must route through Supabase Edge Functions
- Use OS keychain/credential store for user tokens if needed
- Zeroize secrets in memory when stored (use `zeroize` crate)

```rust
use zeroize::Zeroize;

fn handle_sensitive_data(mut api_key: String) {
    // ... use api_key ...
    api_key.zeroize(); // Clear from memory
}
```

### TypeScript/Deno Functions
- Use `Deno.env.get()` for environment variables
- Never commit `.env` files to version control
- Store secrets in Supabase project settings (encrypted)
- Do not log environment variables or secrets

```typescript
// Good: safe environment variable usage
const OPENAI_API_KEY = Deno.env.get('OPENAI_API_KEY');
if (!OPENAI_API_KEY) {
  throw new Error('Missing required API key');
}
```

## Authentication & Authorization
- Use JWT-based authentication via Supabase Auth
- Implement Row Level Security (RLS) on all database tables
- Verify JWT tokens in Edge Functions before processing requests
- Check subscription status and token limits before expensive operations
- Use role-based access control where appropriate

```typescript
// Good: verify auth before processing
const authHeader = req.headers.get('Authorization');
if (!authHeader) {
  return new Response('Unauthorized', { status: 401 });
}

const { data: { user }, error } = await supabaseClient.auth.getUser(
  authHeader.replace('Bearer ', '')
);

if (error || !user) {
  return new Response('Invalid token', { status: 401 });
}
```

## Data Handling
- Prefer IDs or hashes over raw sensitive data in logs
- Avoid storing sensitive meeting content longer than necessary
- Implement data retention policies
- Support user data deletion (GDPR compliance)
- Encrypt sensitive data at rest when applicable

## Network Security
- Use HTTPS for all external communications
- Validate SSL certificates (don't disable verification)
- Implement timeouts for all network requests
- Rate limit API endpoints to prevent abuse
- Use AbortSignal for cancellable operations

## Input Validation
- Validate and sanitize all user inputs
- Use typed parsing (don't trust raw JSON)
- Reject oversized payloads early
- Validate file types and sizes for uploads
- Sanitize file paths to prevent directory traversal

```rust
// Good: validate and bound inputs
fn process_audio_chunk(data: &[u8]) -> Result<()> {
    const MAX_CHUNK_SIZE: usize = 10 * 1024 * 1024; // 10MB
    
    if data.is_empty() {
        return Err(anyhow!("Empty audio chunk"));
    }
    
    if data.len() > MAX_CHUNK_SIZE {
        return Err(anyhow!("Audio chunk exceeds maximum size"));
    }
    
    // ... process data
}
```

## Testing Security
- Never commit test credentials or API keys
- Use mock services for testing API integrations
- Test error paths and edge cases
- Verify that secrets are not leaked in error messages
- Include security-focused tests (invalid tokens, oversized inputs, etc.)
