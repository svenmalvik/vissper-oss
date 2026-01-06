# Rust Error Handling

## Library Code
- Define error enums and implement `std::error::Error` (e.g., with `thiserror`)
- Avoid panics in library code

## Application Code
- Use `anyhow::Result` for ergonomic error propagation
- Add context with `.with_context(...)` to provide meaningful error chains

## General Rules
- Avoid `unwrap`/`expect` outside tests and initialization paths
- Prefer `?` operator for propagation
- Provide meaningful error messages when using `expect`
- Log errors once at the boundary (e.g., CLI, HTTP handler) to avoid duplicate noise

## Example Pattern

```rust
use anyhow::{Context, Result};

fn process_audio(path: &Path) -> Result<AudioData> {
    let file = std::fs::read(path)
        .with_context(|| format!("Failed to read audio file: {}", path.display()))?;
    
    parse_audio(&file)
        .context("Failed to parse audio data")?
}
```

## Avoid
```rust
// DON'T: unwrap without context
let data = read_file(path).unwrap();

// DON'T: generic error messages
let data = read_file(path).expect("error");
```

## Do
```rust
// DO: meaningful context
let data = read_file(path)
    .with_context(|| format!("Failed to read config from {}", path.display()))?;
```
