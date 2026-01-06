# Vissper OSS

Real-time speech-to-text transcription for macOS with AI-powered transcript polishing. Built with Rust and native macOS frameworks.

## Overview

Vissper OSS is an open-source desktop application that captures microphone audio and transcribes it in real-time using Azure OpenAI's Realtime API. It runs discretely in the macOS menu bar with a transparent overlay window for live transcription display.

**Key highlights:**
- Direct connection to Azure OpenAI (no intermediary servers)
- Native macOS UI via objc2 bindings to AppKit
- Real-time WebSocket streaming for instant transcription
- Secure credential storage in macOS Keychain

## Features

### Transcription
- Real-time speech-to-text via Azure OpenAI Realtime API (GPT-4o Transcribe)
- Multi-language support: English, Norwegian, Danish, Finnish, German
- Live partial and final transcript display
- Automatic reconnection with retry logic

### AI-Powered Polishing
- **Basic Polish**: Copyediting for grammar and readability
- **Meeting Notes**: Structured summaries with action items, decisions, and key points
- Preserves original language and meaning

### User Interface
- Menu bar integration (NSStatusBar)
- Transparent overlay window that floats above other applications
- Multi-tab view: Raw transcript, Basic polish, Meeting notes
- Customizable transparency and appearance

### Screenshot Integration
- Full-screen and region-based screenshot capture
- Screenshots embedded in transcripts as markdown images
- Timestamped filenames for organization

### Export Options
- Copy to clipboard (automatic on stop)
- Save as Markdown files
- Export to PDF

## Requirements

- **macOS 12.0+** (Monterey or later)
- **Azure OpenAI** account with:
  - GPT-4o Transcribe deployment (for speech-to-text)
  - GPT-4o or similar deployment (for polishing)
- **Rust** toolchain (latest stable) - [Install Rust](https://rustup.rs/)

## Quick Start

### 1. Clone and Build

```bash
git clone https://github.com/vissper/vissper-oss.git
cd vissper-oss
cargo build --release
```

### 2. Run

```bash
cargo run --release
```

Or run the binary directly:

```bash
./target/release/vissper
```

### 3. Configure Azure OpenAI

1. Click the Vissper icon in the menu bar
2. Select **Settings**
3. Enter your Azure OpenAI credentials:
   - **Endpoint URL**: `https://your-resource.openai.azure.com`
   - **STT Deployment**: Your GPT-4o Transcribe deployment name
   - **Polish Deployment**: Your GPT-4o deployment name
   - **API Key**: Your Azure OpenAI API key
4. Click **Save**

Credentials are stored securely in the macOS Keychain.

### 4. Start Recording

- Click **Start Recording** in the menu bar, or
- Press **Control + Space** (global hotkey)

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Control + Space | Start/Stop recording (raw transcript) |
| Control + Shift + 1 | Stop with basic polishing |
| Control + Shift + 2 | Stop with meeting notes |
| Control + Shift + 0 | Full-screen screenshot |
| Control + Shift + 9 | Region screenshot |

## Project Structure

```
vissper-oss/
├── src/
│   ├── main.rs                    # Application entry point
│   ├── audio/                     # CoreAudio microphone capture
│   │   ├── mod.rs                 # Audio capture implementation
│   │   ├── types.rs               # AudioChunk, AudioCaptureHandle
│   │   └── resampler.rs           # 16kHz resampling
│   ├── menubar/                   # macOS menu bar (NSStatusBar)
│   │   ├── mod.rs                 # Menu bar manager
│   │   ├── builder.rs             # Menu item construction
│   │   ├── delegate.rs            # Objective-C delegate
│   │   ├── state.rs               # App state and callbacks
│   │   └── updates/               # Dynamic menu updates
│   ├── transcription_window/      # Transparent overlay window
│   │   ├── mod.rs                 # Window manager
│   │   ├── window.rs              # Window creation/layout
│   │   ├── state.rs               # Tab types, window state
│   │   ├── components/            # UI components
│   │   └── api/                   # Public window APIs
│   ├── recording/                 # Recording session management
│   │   ├── mod.rs                 # Start/stop logic
│   │   ├── transcription_task.rs  # Background transcription
│   │   ├── polish.rs              # Transcript polishing
│   │   └── clipboard.rs           # Clipboard operations
│   ├── transcription/             # Azure OpenAI Realtime API
│   │   ├── mod.rs                 # TranscriptionClient
│   │   ├── azure_connection.rs    # WebSocket management
│   │   └── azure_messages.rs      # Message serialization
│   ├── azure_openai.rs            # Azure OpenAI Chat API client
│   ├── keychain.rs                # macOS Keychain storage
│   ├── settings_window/           # Settings UI
│   ├── hotkeys.rs                 # Global keyboard shortcuts
│   ├── screenshot.rs              # Screenshot capture
│   ├── storage.rs                 # Local file storage
│   └── preferences.rs             # User preferences
├── config.toml                    # Application configuration
├── Cargo.toml                     # Rust dependencies
└── README.md
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Vissper (Rust + objc2)                  │
├─────────────────────────────────────────────────────────────┤
│  Menu Bar          │  Transcription Window  │  Settings     │
│  (NSStatusBar)     │  (NSWindow overlay)    │  (NSWindow)   │
├────────────────────┴───────────────────────┴────────────────┤
│                     Recording Manager                        │
├─────────────────────────────────────────────────────────────┤
│  Audio Capture     │  Transcription Client  │  Polish API   │
│  (CoreAudio)       │  (WebSocket)           │  (REST)       │
├────────────────────┴───────────────────────┴────────────────┤
│                   Azure OpenAI Services                      │
│         Realtime API (STT)    │    Chat API (Polish)        │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **Audio Capture**: CoreAudio captures microphone input, resampled to 16kHz mono PCM
2. **Streaming**: Audio chunks sent via WebSocket to Azure OpenAI Realtime API
3. **Transcription**: Partial and final transcripts received in real-time
4. **Display**: Transcripts shown in transparent overlay window
5. **Polishing**: On stop, optionally polish via Azure OpenAI Chat API
6. **Output**: Copy to clipboard and save to local storage

## Development

### Build Commands

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Lint
cargo clippy --all-targets --all-features

# Format code
cargo fmt
```

### Key Dependencies

| Category | Crate | Purpose |
|----------|-------|---------|
| Async | tokio | Async runtime |
| HTTP | reqwest | REST API client |
| WebSocket | tokio-tungstenite | Realtime API |
| Audio | cpal, rubato | Capture and resampling |
| macOS UI | objc2, objc2-app-kit | Native AppKit bindings |
| Security | security-framework | Keychain integration |
| Hotkeys | global-hotkey | System-wide shortcuts |

### Thread Safety

All AppKit operations must run on the main thread. The codebase uses `MainThreadMarker` to prove main thread execution:

```rust
if let Some(mtm) = MainThreadMarker::new() {
    // On main thread - safe to call AppKit
    Self::update_ui(mtm);
} else {
    // Dispatch to main thread
    dispatch::Queue::main().exec_async(|| {
        if let Some(mtm) = MainThreadMarker::new() {
            Self::update_ui(mtm);
        }
    });
}
```

## Configuration

### config.toml

```toml
[version_check]
url = "https://vissper.com/version.json"
enabled = true
```

### User Preferences

Stored in `~/.config/Vissper/preferences.json`:
- Language preference
- Transcript/screenshot storage paths
- Overlay transparency (0.3-1.0)

### Azure Credentials

Stored securely in macOS Keychain under service `com.vissper.desktop`.

## Azure OpenAI Setup

1. Create an Azure OpenAI resource in the [Azure Portal](https://portal.azure.com)
2. Deploy the required models:
   - **GPT-4o Transcribe** - for real-time speech-to-text
   - **GPT-4o** (or similar) - for transcript polishing
3. Copy your API key from the Azure Portal
4. Enter credentials in Vissper Settings

## Security

- Azure credentials stored in macOS Keychain (encrypted)
- No data sent to Vissper servers (direct Azure connection)
- Audio and transcripts stored only locally
- Sensitive data cleared from memory using `zeroize`
- No API keys or credentials in code or logs

## License

Dual licensed under MIT and Apache 2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

## Contributing

Contributions welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting pull requests.

## Support

- [GitHub Issues](https://github.com/vissper/vissper-oss/issues) - Bug reports and feature requests
- [Discussions](https://github.com/vissper/vissper-oss/discussions) - Questions and community support
