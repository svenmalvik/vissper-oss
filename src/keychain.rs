//! Secure credential storage using macOS Keychain
//!
//! Provides secure storage for Azure OpenAI credentials using OS-native
//! credential storage. On macOS, uses the Keychain. On Windows, would
//! use DPAPI (Data Protection API) - currently not implemented.
//!
//! # Security
//! - Credentials are stored encrypted in the OS keychain
//! - Only the Vissper application can access these credentials

use crate::error::KeychainError;
use serde::{Deserialize, Serialize};

#[cfg(target_os = "macos")]
use security_framework::passwords::*;

const SERVICE_NAME: &str = "com.vissper.desktop";

/// Azure OpenAI credentials for direct connection.
///
/// Stored encrypted in OS Keychain. Users provide their own Azure OpenAI
/// resources for STT and transcript polishing.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AzureCredentials {
    /// Azure OpenAI API key
    pub(crate) api_key: String,
    /// Azure OpenAI endpoint URL (e.g., "https://myresource.openai.azure.com")
    pub(crate) endpoint_url: String,
    /// Deployment name for STT (e.g., "gpt-4o-transcribe")
    pub(crate) stt_deployment: String,
    /// Deployment name for transcript polishing (e.g., "gpt-5.1")
    pub(crate) polish_deployment: String,
}

/// OpenAI credentials for direct connection.
///
/// Stored encrypted in OS Keychain. Users provide their own OpenAI API key.
/// Unlike Azure, OpenAI only requires an API key (no endpoint or deployment names).
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct OpenAICredentials {
    /// OpenAI API key
    pub(crate) api_key: String,
}

/// Store Azure credentials securely in the keychain.
#[cfg(target_os = "macos")]
pub(crate) fn store_azure_credentials(creds: &AzureCredentials) -> Result<(), KeychainError> {
    let json = serde_json::to_string(creds).map_err(|e| {
        KeychainError::Store(format!("Failed to serialize Azure credentials: {}", e))
    })?;

    // Delete existing item if present
    let _ = delete_generic_password(SERVICE_NAME, "azure_credentials");

    // Store the credentials
    set_generic_password(SERVICE_NAME, "azure_credentials", json.as_bytes())
        .map_err(|e| KeychainError::Store(e.to_string()))
}

/// Retrieve Azure credentials from keychain.
#[cfg(target_os = "macos")]
pub(crate) fn get_azure_credentials() -> Result<AzureCredentials, KeychainError> {
    let password = get_generic_password(SERVICE_NAME, "azure_credentials")
        .map_err(|e| KeychainError::Retrieve(e.to_string()))?;

    let json = String::from_utf8(password.to_vec())
        .map_err(|e| KeychainError::InvalidData(e.to_string()))?;

    serde_json::from_str(&json).map_err(|e| {
        KeychainError::InvalidData(format!("Failed to deserialize Azure credentials: {}", e))
    })
}

/// Delete Azure credentials from keychain.
#[cfg(target_os = "macos")]
pub(crate) fn delete_azure_credentials() -> Result<(), KeychainError> {
    delete_generic_password(SERVICE_NAME, "azure_credentials")
        .map_err(|e| KeychainError::Delete(e.to_string()))
}

/// Store OpenAI credentials securely in the keychain.
#[cfg(target_os = "macos")]
pub(crate) fn store_openai_credentials(creds: &OpenAICredentials) -> Result<(), KeychainError> {
    let json = serde_json::to_string(creds).map_err(|e| {
        KeychainError::Store(format!("Failed to serialize OpenAI credentials: {}", e))
    })?;

    // Delete existing item if present
    let _ = delete_generic_password(SERVICE_NAME, "openai_credentials");

    // Store the credentials
    set_generic_password(SERVICE_NAME, "openai_credentials", json.as_bytes())
        .map_err(|e| KeychainError::Store(e.to_string()))
}

/// Retrieve OpenAI credentials from keychain.
#[cfg(target_os = "macos")]
pub(crate) fn get_openai_credentials() -> Result<OpenAICredentials, KeychainError> {
    let password = get_generic_password(SERVICE_NAME, "openai_credentials")
        .map_err(|e| KeychainError::Retrieve(e.to_string()))?;

    let json = String::from_utf8(password.to_vec())
        .map_err(|e| KeychainError::InvalidData(e.to_string()))?;

    serde_json::from_str(&json).map_err(|e| {
        KeychainError::InvalidData(format!("Failed to deserialize OpenAI credentials: {}", e))
    })
}

/// Delete OpenAI credentials from keychain.
#[cfg(target_os = "macos")]
pub(crate) fn delete_openai_credentials() -> Result<(), KeychainError> {
    delete_generic_password(SERVICE_NAME, "openai_credentials")
        .map_err(|e| KeychainError::Delete(e.to_string()))
}

// Stub implementations for non-macOS platforms
// In the future, implement Windows DPAPI here
#[cfg(not(target_os = "macos"))]
pub(crate) fn store_azure_credentials(_creds: &AzureCredentials) -> Result<(), KeychainError> {
    Err(KeychainError::NotImplemented)
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn get_azure_credentials() -> Result<AzureCredentials, KeychainError> {
    Err(KeychainError::NotImplemented)
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn delete_azure_credentials() -> Result<(), KeychainError> {
    Err(KeychainError::NotImplemented)
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn store_openai_credentials(_creds: &OpenAICredentials) -> Result<(), KeychainError> {
    Err(KeychainError::NotImplemented)
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn get_openai_credentials() -> Result<OpenAICredentials, KeychainError> {
    Err(KeychainError::NotImplemented)
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn delete_openai_credentials() -> Result<(), KeychainError> {
    Err(KeychainError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn test_azure_credentials_storage() {
        let test_creds = AzureCredentials {
            api_key: "test_key_12345".to_string(),
            endpoint_url: "https://test.openai.azure.com".to_string(),
            stt_deployment: "gpt-4o-transcribe".to_string(),
            polish_deployment: "gpt-4o".to_string(),
        };

        // Store credentials
        store_azure_credentials(&test_creds).expect("Failed to store credentials");

        // Retrieve credentials
        let retrieved = get_azure_credentials().expect("Failed to retrieve credentials");
        assert_eq!(retrieved.api_key, test_creds.api_key);
        assert_eq!(retrieved.endpoint_url, test_creds.endpoint_url);

        // Delete credentials
        delete_azure_credentials().expect("Failed to delete credentials");

        // Verify deletion
        assert!(get_azure_credentials().is_err());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_openai_credentials_storage() {
        let test_creds = OpenAICredentials {
            api_key: "sk-test_openai_key_12345".to_string(),
        };

        // Store credentials
        store_openai_credentials(&test_creds).expect("Failed to store credentials");

        // Retrieve credentials
        let retrieved = get_openai_credentials().expect("Failed to retrieve credentials");
        assert_eq!(retrieved.api_key, test_creds.api_key);

        // Delete credentials
        delete_openai_credentials().expect("Failed to delete credentials");

        // Verify deletion
        assert!(get_openai_credentials().is_err());
    }
}
