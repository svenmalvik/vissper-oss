//! Version checking module
//!
//! Automatically checks for app updates by fetching version information from a remote JSON file.
//! Checks occur once per 24 hours and display a menu item when updates are available.

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Global version checker configuration
static VERSION_CHECK_URL: OnceCell<String> = OnceCell::new();

/// Global storage for the latest version info (for callback access)
static LATEST_VERSION_INFO: Mutex<Option<VersionInfo>> = Mutex::new(None);

/// Version information from remote JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub download_url: String,
    pub release_notes: Option<String>,
}

/// Version check errors
#[derive(Debug, thiserror::Error)]
pub enum VersionCheckError {
    #[error("Network request failed: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Invalid JSON response: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("Invalid version format: {0}")]
    InvalidVersion(#[from] semver::Error),

    #[error("Version check URL not configured")]
    NotConfigured,
}

/// Initialize the version checker with the configured URL
pub fn initialize(url: String) {
    if VERSION_CHECK_URL.set(url.clone()).is_err() {
        warn!("Version check URL already initialized");
    } else {
        info!("Version checker initialized with URL: {}", url);
    }
}

/// Check for updates from the remote JSON file
///
/// Returns Ok(Some(version_info)) if a new version is available,
/// Ok(None) if no update is available,
/// Err if the check failed.
///
/// If `force` is true, bypasses the 24-hour interval check.
pub async fn check_for_updates_internal(
    force: bool,
) -> Result<Option<VersionInfo>, VersionCheckError> {
    // Check if we should perform a check based on preferences
    if !force && !crate::preferences::should_check_for_updates() {
        info!("Skipping version check (checked recently)");
        return Ok(None);
    }

    // Get the check URL
    let check_url = VERSION_CHECK_URL
        .get()
        .ok_or(VersionCheckError::NotConfigured)?;

    info!("Fetching version info from: {}", check_url);

    // Fetch version info from URL
    let client = reqwest::Client::new();
    let response = client
        .get(check_url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    info!("Received response, parsing JSON...");
    let version_info: VersionInfo = response.json().await?;
    info!(
        "Fetched version info: version={}, download_url={}",
        version_info.version, version_info.download_url
    );

    // Update last check time
    if let Err(e) = crate::preferences::update_version_check_time() {
        warn!("Failed to update version check time: {}", e);
    }

    // Compare versions
    let current_version = env!("CARGO_PKG_VERSION");
    match compare_versions(current_version, &version_info.version)? {
        Ordering::Less => {
            info!(
                "Update available: {} -> {}",
                current_version, version_info.version
            );

            // Cache the version info
            if let Err(e) = crate::preferences::set_latest_known_version(&version_info.version) {
                warn!("Failed to cache version: {}", e);
            }
            if let Err(e) = crate::preferences::set_latest_download_url(&version_info.download_url)
            {
                warn!("Failed to cache download URL: {}", e);
            }

            // Store version info globally for callback access
            if let Ok(mut info) = LATEST_VERSION_INFO.lock() {
                *info = Some(version_info.clone());
            }

            Ok(Some(version_info))
        }
        Ordering::Equal => {
            debug!("App is up to date ({})", current_version);
            Ok(None)
        }
        Ordering::Greater => {
            debug!(
                "Current version ({}) is newer than remote ({})",
                current_version, version_info.version
            );
            Ok(None)
        }
    }
}

/// Check for updates from the remote JSON file (respects 24h interval)
///
/// Returns Ok(Some(version_info)) if a new version is available,
/// Ok(None) if no update is available,
/// Err if the check failed.
#[allow(dead_code)]
pub async fn check_for_updates() -> Result<Option<VersionInfo>, VersionCheckError> {
    check_for_updates_internal(false).await
}

/// Compare two semantic version strings
///
/// Returns:
/// - Ordering::Less if current < latest (update available)
/// - Ordering::Equal if current == latest (up to date)
/// - Ordering::Greater if current > latest (dev version)
fn compare_versions(current: &str, latest: &str) -> Result<Ordering, VersionCheckError> {
    let current_ver = semver::Version::parse(current)?;
    let latest_ver = semver::Version::parse(latest)?;
    Ok(current_ver.cmp(&latest_ver))
}

/// Get the download URL from the cached version info
///
/// Used by the callback when the user clicks the "Update Available" menu item.
pub fn get_download_url_from_cache() -> Option<String> {
    crate::preferences::get_latest_download_url()
}

/// Start the background version checker task
///
/// This spawns a tokio task that checks for updates:
/// - Immediately on startup (if 24h elapsed)
/// - Every hour thereafter (but only performs actual check if 24h elapsed)
pub fn start_update_checker() {
    info!("Starting background version checker");

    tokio::spawn(async {
        info!("Version checker task spawned, waiting 2 seconds for UI initialization...");

        // Wait 2 seconds for the UI event loop to start before initial check
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        info!("Performing initial version check...");
        perform_check_and_update().await;
        info!("Initial version check completed");

        // Check every hour, but respect 24h preference limit
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            info!("Hourly version check triggered");
            perform_check_and_update().await;
        }
    });
}

/// Perform a version check and update the menu bar
async fn perform_check_and_update() {
    info!("Starting version check...");

    let current_version = env!("CARGO_PKG_VERSION");

    // First, check if we have a cached update that should still be shown
    let mut missing_download_url = false;

    if let Some(cached_version) = crate::preferences::get_latest_known_version() {
        match compare_versions(current_version, &cached_version) {
            Ok(Ordering::Less) => {
                // Cached version is still newer - keep showing the menu item
                info!(
                    "Cached update available: {} -> {}",
                    current_version, cached_version
                );
                crate::menubar::MenuBar::show_update_available(&cached_version);

                // Check if we have the download URL cached
                if crate::preferences::get_latest_download_url().is_none() {
                    warn!("Cached version exists but download URL is missing");
                    missing_download_url = true;
                }
            }
            Ok(_) => {
                // User has updated or cached version is no longer newer - clear cache
                info!("User has updated or cached version is no longer valid, clearing cache");
                let _ = crate::preferences::set_latest_known_version("");
                let _ = crate::preferences::set_latest_download_url("");
                crate::menubar::MenuBar::hide_update_available();
            }
            Err(e) => {
                warn!("Failed to compare cached version: {}", e);
            }
        }
    }

    // Now perform the actual network check
    // Force check if we have a cached version but missing download URL
    let force_check = missing_download_url;
    if force_check {
        info!("Forcing version check to retrieve missing download URL");
    }

    match check_for_updates_internal(force_check).await {
        Ok(Some(version_info)) => {
            // Update available - show menu item
            info!(
                "Update detected! Showing menu item for version {}",
                version_info.version
            );
            crate::menubar::MenuBar::show_update_available(&version_info.version);
            info!("Menu item update requested");
        }
        Ok(None) => {
            // Network check skipped or no update found
            // Don't hide if we're showing a cached update - only hide if we actually checked
            if crate::preferences::should_check_for_updates() {
                // This means we didn't check due to interval - keep current state
                info!("Skipped network check, keeping current menu state");
            }
        }
        Err(e) => {
            // Error checking - log but don't disrupt user or hide existing notification
            warn!("Version check failed: {}", e);
        }
    }
    info!("Version check completed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        // Current < Latest (update available)
        assert_eq!(compare_versions("0.1.0", "0.2.0").unwrap(), Ordering::Less);
        assert_eq!(compare_versions("0.1.0", "1.0.0").unwrap(), Ordering::Less);
        assert_eq!(compare_versions("1.0.0", "1.0.1").unwrap(), Ordering::Less);

        // Current == Latest (up to date)
        assert_eq!(compare_versions("1.0.0", "1.0.0").unwrap(), Ordering::Equal);

        // Current > Latest (dev version)
        assert_eq!(
            compare_versions("0.2.0", "0.1.0").unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_versions("1.0.0", "0.9.9").unwrap(),
            Ordering::Greater
        );
    }

    #[test]
    fn test_version_info_parsing() {
        let json = r#"{
            "version": "0.2.0",
            "download_url": "https://example.com/download",
            "release_notes": "Bug fixes"
        }"#;

        let info: VersionInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.version, "0.2.0");
        assert_eq!(info.download_url, "https://example.com/download");
        assert_eq!(info.release_notes, Some("Bug fixes".to_string()));
    }

    #[test]
    fn test_version_info_parsing_minimal() {
        let json = r#"{
            "version": "0.2.0",
            "download_url": "https://example.com/download"
        }"#;

        let info: VersionInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.version, "0.2.0");
        assert_eq!(info.download_url, "https://example.com/download");
        assert_eq!(info.release_notes, None);
    }

    #[test]
    fn test_invalid_version_format() {
        let result = compare_versions("invalid", "0.1.0");
        assert!(result.is_err());

        let result = compare_versions("0.1.0", "not.a.version");
        assert!(result.is_err());
    }
}
