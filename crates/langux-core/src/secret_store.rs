use std::fmt;

/// Stable Secret Service identifier for Langux.
///
/// Keep this value stable after credentials have been stored on user systems.
pub const LANGUX_SECRET_SERVICE_ID: &str = "io.github.rafaself.Langux";

/// Stable account identifier for the Google Cloud Translation API key.
///
/// Keep this value stable after credentials have been stored on user systems.
pub const GOOGLE_TRANSLATION_ACCOUNT_ID: &str = "google-translation-api-key";

/// An in-memory Google Translation credential.
///
/// This type deliberately has no serialization or `Display` implementation,
/// does not implement `Clone`, and redacts its contents from `Debug` output.
/// Call [`expose_secret`](Self::expose_secret) only when passing the value to
/// the translation provider. The secret store contract never writes a
/// credential to application configuration or another plaintext fallback.
pub struct SecretCredential(String);

impl SecretCredential {
    /// Wraps a credential returned by a user or secret store.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrows the credential for use by an authenticated provider request.
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretCredential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretCredential([REDACTED])")
    }
}

/// A normalized secret-store failure that never carries backend or secret data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SecretStoreError {
    /// No compatible secret service is available to the application.
    Unavailable,
    /// The secret collection is locked and cannot perform the requested action.
    Locked,
    /// The operation failed for another backend reason.
    Failed,
    /// A credential already exists and the caller must use [`SecretStore::replace`].
    AlreadyConfigured,
    /// No credential exists to replace.
    NotConfigured,
}

impl fmt::Display for SecretStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "Secret storage is unavailable.",
            Self::Locked => "Secret storage is locked.",
            Self::Failed => "Secret storage could not complete the operation.",
            Self::AlreadyConfigured => "A credential is already configured.",
            Self::NotConfigured => "No credential is configured.",
        })
    }
}

impl std::error::Error for SecretStoreError {}

/// Stores the Google Translation credential without exposing backend details.
///
/// `retrieve` returns `Ok(None)` when there is no credential; backend failures
/// are represented by [`SecretStoreError`]. Implementations must never log or
/// include credential contents in errors. `save` configures a credential only
/// when none exists, while `replace` overwrites an existing credential.
/// Removing an absent credential should succeed, making cleanup idempotent.
pub trait SecretStore: Send + Sync {
    /// Saves the first configured credential.
    fn save(&self, credential: &SecretCredential) -> Result<(), SecretStoreError>;

    /// Retrieves the credential for an authenticated provider request.
    fn retrieve(&self) -> Result<Option<SecretCredential>, SecretStoreError>;

    /// Reports whether a credential is configured without returning its value.
    fn exists(&self) -> Result<bool, SecretStoreError>;

    /// Replaces a configured credential.
    fn replace(&self, credential: &SecretCredential) -> Result<(), SecretStoreError>;

    /// Removes the configured credential. Removing an absent credential succeeds.
    fn remove(&self) -> Result<(), SecretStoreError>;
}
