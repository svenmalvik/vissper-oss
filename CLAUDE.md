# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Vissper OSS is an open-source macOS desktop application for real-time transcription and AI-powered transcript polishing. Users provide their own Azure OpenAI credentials. Built with Rust + objc2 for native macOS integration.

## Essential Commands

### Build and Run
```bash
# Development build and run
cargo build
cargo run

# Release build
cargo build --release

# Run tests
cargo test

# Linting and formatting
cargo clippy --all-targets --all-features
cargo fmt
```

## Architecture

### Core Design Pattern
```
Desktop App (Rust + objc2)
    ↓ Audio capture
CoreAudio (microphone input)
    ↓ PCM audio chunks
Azure OpenAI Realtime API (WebSocket)
    ↓ Transcribed text
Transcription Window (overlay)
    ↓ On stop
Azure OpenAI Chat API (polish/summarize)
    ↓ Polished transcript
Clipboard + File storage
```

### Module Organization

**Desktop Application** (Rust):
- `main.rs` - Entry point, initializes app state and menu bar
- `menubar/` - Native macOS menu bar (NSStatusBar) using objc2
  - `state.rs` - Global app state (recording, processing, Azure credentials)
  - `builder.rs` - Menu item construction
  - `delegate.rs` - Objective-C delegate for menu actions
  - `updates/` - Dynamic menu state updates
- `transcription_window/` - Transparent overlay window (NSWindow) using objc2
  - `window.rs` - Window creation and layout
  - `api/` - Public API modules (recording, save, tabs, text, window)
  - `components.rs` - UI components (tabs, text views, buttons)
- `recording/` - Recording session management
  - `mod.rs` - Start/stop recording, session state
  - `transcription_task.rs` - Azure WebSocket transcription
  - `polish.rs` - Transcript polishing via Azure OpenAI
- `transcription/` - Azure OpenAI Realtime API connection
  - `mod.rs` - TranscriptionClient with reconnection support
  - `azure_connection.rs` - WebSocket connection management
  - `azure_messages.rs` - Message serialization
- `audio/` - Audio capture (CoreAudio) and resampling
- `azure_openai/` - Azure OpenAI Chat API client for polishing
- `keychain.rs` - Secure Azure credential storage (macOS Keychain)
- `settings_window/` - Settings UI for Azure credentials
- `callbacks/` - Menu bar and hotkey callback handlers
- `hotkeys.rs` - Global keyboard shortcuts

### Key Architectural Principles

1. **Azure-Only Backend**: All AI services use user-provided Azure OpenAI credentials
2. **Native UI**: Direct objc2 bindings to AppKit (macOS) for menu bar and overlay window
3. **Security**: Azure credentials stored in macOS Keychain
4. **Async Throughout**: Tokio runtime for all I/O operations
5. **Global State with Callbacks**: OnceCell-based global state with callback patterns for UI updates

## Critical Implementation Details

### Thread Safety and Main Thread Requirements

- All AppKit operations MUST run on the main thread
- Use `MainThreadMarker` from objc2_foundation to prove main thread execution
- Dispatch to main thread using `dispatch::Queue::main()` or `NSOperationQueue::mainQueue()`
- Never call UI methods from background threads

Example pattern:
```rust
pub fn show() {
    if let Some(mtm) = MainThreadMarker::new() {
        Self::show_on_main_thread(mtm);
        return;
    }
    // Not on main thread - dispatch
    dispatch::Queue::main().exec_async(|| {
        if let Some(mtm) = MainThreadMarker::new() {
            Self::show_on_main_thread(mtm);
        }
    });
}
```

### Azure Credentials Flow

1. User enters Azure credentials in Settings window
2. Credentials stored securely in macOS Keychain
3. On startup, check for credentials → enable/disable recording
4. Recording reads credentials from Keychain → connects to Azure

### Menu Bar State Management

The menu bar uses a callback pattern where `MenuCallbacks` are set during initialization:
- `has_azure_credentials` - Controls whether recording is enabled
- `is_recording` - Current recording state
- `is_processing` - Polishing in progress

### Recording Flow

1. User starts recording → `recording::start_recording()`
2. Get Azure credentials from Keychain
3. Start audio capture → `audio::start_capture()`
4. Connect to Azure OpenAI Realtime → `transcription::start_azure()`
5. Stream audio chunks → Receive transcripts → Update UI
6. User stops → Polish transcript → Copy to clipboard

## Development Guidelines

### Rust Code
- Use `anyhow::Result` for application code, add context with `.with_context(...)`
- Use `tracing` for structured logging
- Never log sensitive data (audio, transcripts, PII, API keys)
- Keep modules small (<300 LOC), extract submodules when larger
- Use `#![deny(clippy::all)]` and fix all clippy warnings

### Objective-C Interop (objc2)
- Use `Retained<T>` for all NSObject references
- Prove main thread execution with `MainThreadMarker`
- Use `unsafe` blocks only for FFI calls to AppKit/Foundation
- Store Objective-C objects in OnceCell or static Mutex for global access

### Security
- Never commit API keys or credentials
- Azure credentials stored only in Keychain
- Zeroize sensitive data in memory after use

### Testing
- Co-locate unit tests in module files
- Test async code with `#[tokio::test]`
- Use table-driven tests for coverage

## Common Tasks

### Adding a New Menu Item
1. Add menu item creation in `menubar/builder.rs`
2. Add retained reference to `MenuBarInner` struct in `menubar/mod.rs`
3. Add action handler in `menubar/delegate.rs`
4. Wire up callback in `menubar/state.rs` MenuCallbacks
5. Add state update logic in `menubar/updates/` if dynamic

### Modifying Azure Credentials
1. Update `keychain.rs` AzureCredentials struct if needed
2. Update `settings_window/controls/azure.rs` UI fields
3. Update `recording/mod.rs` credential retrieval

### Adding a New Transcription Window Tab
1. Add tab type to `TabType` enum in `transcription_window/state.rs`
2. Create NSTextView in `transcription_window/components.rs`
3. Add tab switching logic in `transcription_window/api/tabs.rs`
4. Wire up in `transcription_window/window.rs` layout

## Configuration

Configuration is loaded from embedded `config.toml` at compile time:
```toml
[version_check]
url = "https://vissper.com/version.json"
enabled = true
```

Azure credentials are stored in macOS Keychain, not in config files.

## Platform-Specific Notes

### macOS
- Uses CoreAudio for audio capture (in `audio/mod.rs`)
- Uses Keychain for secure credential storage (`keychain.rs`)
- Native menu bar via NSStatusBar
- Transparent overlay via NSWindow with custom styling

### Windows (Planned)
- Will use WASAPI for audio capture
- Will use DPAPI for secure credential storage
- System tray integration
