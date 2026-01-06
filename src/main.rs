#![deny(clippy::all)]

mod audio;
mod azure_openai;
mod callbacks;
mod error;
mod hotkeys;
mod keychain;
mod menubar;
mod openai;
mod preferences;
mod recording;
mod region_selection;
mod response;
mod screenshot;
mod screenshot_flash;
mod settings_window;
mod storage;
mod transcription;
mod transcription_window;
mod version_check;

use std::sync::{Arc, Mutex};
use tracing::info;

// Re-export error types (used by other modules)
#[allow(unused_imports)]
pub use error::*;

/// Application configuration
#[derive(serde::Deserialize)]
struct Config {
    version_check: VersionCheckConfig,
}

#[derive(serde::Deserialize)]
struct VersionCheckConfig {
    url: String,
    enabled: bool,
}

/// Load configuration from embedded config.toml
fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    const CONFIG_TOML: &str = include_str!("../config.toml");
    let config: Config = toml::from_str(CONFIG_TOML)?;
    Ok(config)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for structured logging
    tracing_subscriber::fmt::init();

    // Load configuration from embedded config.toml
    let config = load_config()?;

    // Create shared application state
    let app_state = Arc::new(menubar::AppState::new());

    // Check for stored credentials based on selected provider
    let ai_provider = preferences::get_ai_provider();
    let has_credentials = match ai_provider {
        preferences::AiProvider::Azure => {
            let has_creds = keychain::get_azure_credentials().is_ok();
            if has_creds {
                info!("Azure credentials found in keychain");
            } else {
                info!("No Azure credentials found - user needs to configure in Settings");
            }
            has_creds
        }
        preferences::AiProvider::OpenAI => {
            let has_creds = keychain::get_openai_credentials().is_ok();
            if has_creds {
                info!("OpenAI credentials found in keychain");
            } else {
                info!("No OpenAI credentials found - user needs to configure in Settings");
            }
            has_creds
        }
    };
    info!("Selected AI provider: {:?}", ai_provider);

    // Create shared state for recording session
    let recording_state: Arc<Mutex<Option<recording::RecordingSession>>> =
        Arc::new(Mutex::new(None));

    // Initialize transcription window callbacks
    let window_callbacks = transcription_window::WindowCallbacks {
        on_hide: Arc::new(|| {
            info!("Transcription window hidden via button");
        }),
        on_request_basic_polish: Arc::new(move |transcript: String| {
            tokio::spawn(async move {
                recording::polish_transcript_on_demand(
                    transcript,
                    transcription_window::TabType::BasicPolish,
                )
                .await;
            });
        }),
        on_request_meeting_notes: Arc::new(move |transcript: String| {
            tokio::spawn(async move {
                recording::polish_transcript_on_demand(
                    transcript,
                    transcription_window::TabType::MeetingNotes,
                )
                .await;
            });
        }),
    };
    transcription_window::TranscriptionWindow::init(window_callbacks);
    transcription_window::TranscriptionWindow::load_appearance_preferences();

    // Create callback configuration
    let callback_config = callbacks::CallbackConfig { recording_state };

    // Create and initialize menu bar with callbacks
    let menu_callbacks = callbacks::create_menu_callbacks(&callback_config);
    menubar::MenuBar::init(app_state.clone(), menu_callbacks);

    // Set initial credentials state based on selected provider
    menubar::MenuBar::set_azure_credentials(has_credentials);

    // Initialize global hotkeys
    let hotkey_manager = hotkeys::init_hotkeys()?;
    info!("Global hotkeys initialized successfully");

    // Setup hotkey callbacks
    let runtime_handle = tokio::runtime::Handle::current();
    callbacks::setup_hotkey_callbacks(&callback_config, app_state, runtime_handle);

    // Keep hotkey manager alive
    std::mem::forget(hotkey_manager);

    // Initialize and start version update checker
    if config.version_check.enabled {
        info!("Version checker enabled, initializing...");
        version_check::initialize(config.version_check.url);
        version_check::start_update_checker();
    } else {
        info!("Version checker disabled in configuration");
    }

    // Run the application event loop
    menubar::MenuBar::run();

    Ok(())
}
