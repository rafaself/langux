use langux_core::{
    CancellationToken, GoogleTranslationProvider, SecretStore, SecretStoreError, TranslationError,
    TranslationProvider, TranslationRequest, TranslationResult,
};
use langux_secret_service::SecretServiceStore;

/// Loads the configured API key from Secret Service for each translation.
///
/// The lookup and provider request are run together on the background worker,
/// so a keyring prompt or network wait cannot block GTK's main loop.
pub struct SecretTranslationProvider;

impl TranslationProvider for SecretTranslationProvider {
    fn translate(
        &self,
        request: &TranslationRequest,
        cancellation: &CancellationToken,
    ) -> Result<TranslationResult, TranslationError> {
        if cancellation.is_cancelled() {
            return Err(TranslationError::Cancelled);
        }

        let store = SecretServiceStore::new().map_err(map_secret_store_error)?;
        let credential = store
            .retrieve()
            .map_err(map_secret_store_error)?
            .ok_or(TranslationError::MissingCredential)?;
        if cancellation.is_cancelled() {
            return Err(TranslationError::Cancelled);
        }

        let provider = GoogleTranslationProvider::new(credential.expose_secret())?;

        provider.translate(request, cancellation)
    }
}

fn map_secret_store_error(error: SecretStoreError) -> TranslationError {
    match error {
        SecretStoreError::NotConfigured => TranslationError::MissingCredential,
        SecretStoreError::Unavailable
        | SecretStoreError::Locked
        | SecretStoreError::Failed
        | SecretStoreError::AlreadyConfigured => TranslationError::ProviderFailure,
    }
}
