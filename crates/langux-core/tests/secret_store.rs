use std::sync::Mutex;

use langux_core::{
    GOOGLE_TRANSLATION_ACCOUNT_ID, LANGUX_SECRET_SERVICE_ID, SecretCredential, SecretStore,
    SecretStoreError,
};

#[derive(Default)]
struct MemorySecretStore {
    credential: Mutex<Option<String>>,
    failure: Mutex<Option<SecretStoreError>>,
}

impl MemorySecretStore {
    fn failing_with(error: SecretStoreError) -> Self {
        Self {
            credential: Mutex::new(None),
            failure: Mutex::new(Some(error)),
        }
    }

    fn check_failure(&self) -> Result<(), SecretStoreError> {
        match *self.failure.lock().expect("fake failure lock") {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl SecretStore for MemorySecretStore {
    fn save(&self, credential: &SecretCredential) -> Result<(), SecretStoreError> {
        self.check_failure()?;
        let mut stored = self.credential.lock().expect("fake credential lock");
        if stored.is_some() {
            return Err(SecretStoreError::AlreadyConfigured);
        }
        *stored = Some(credential.expose_secret().to_owned());
        Ok(())
    }

    fn retrieve(&self) -> Result<Option<SecretCredential>, SecretStoreError> {
        self.check_failure()?;
        Ok(self
            .credential
            .lock()
            .expect("fake credential lock")
            .as_ref()
            .map(|value| SecretCredential::new(value.clone())))
    }

    fn exists(&self) -> Result<bool, SecretStoreError> {
        self.check_failure()?;
        Ok(self
            .credential
            .lock()
            .expect("fake credential lock")
            .is_some())
    }

    fn replace(&self, credential: &SecretCredential) -> Result<(), SecretStoreError> {
        self.check_failure()?;
        let mut stored = self.credential.lock().expect("fake credential lock");
        if stored.is_none() {
            return Err(SecretStoreError::NotConfigured);
        }
        *stored = Some(credential.expose_secret().to_owned());
        Ok(())
    }

    fn remove(&self) -> Result<(), SecretStoreError> {
        self.check_failure()?;
        *self.credential.lock().expect("fake credential lock") = None;
        Ok(())
    }
}

#[test]
fn identifiers_are_stable_and_separate_service_from_account() {
    assert_eq!(LANGUX_SECRET_SERVICE_ID, "io.github.rafaself.Langux");
    assert_eq!(GOOGLE_TRANSLATION_ACCOUNT_ID, "google-translation-api-key");
    assert_ne!(LANGUX_SECRET_SERVICE_ID, GOOGLE_TRANSLATION_ACCOUNT_ID);
}

#[test]
fn memory_store_supports_configure_lookup_replace_and_remove() {
    let store = MemorySecretStore::default();
    let initial = SecretCredential::new("first-key");
    let replacement = SecretCredential::new("second-key");

    assert!(store.retrieve().expect("lookup absent key").is_none());
    assert!(!store.exists().expect("check absent key"));

    store.save(&initial).expect("configure key");
    assert!(store.exists().expect("check configured key"));
    assert_eq!(
        store
            .retrieve()
            .expect("retrieve configured key")
            .expect("credential exists")
            .expose_secret(),
        "first-key"
    );
    assert_eq!(
        store.save(&replacement),
        Err(SecretStoreError::AlreadyConfigured)
    );

    store.replace(&replacement).expect("replace key");
    assert_eq!(
        store
            .retrieve()
            .expect("retrieve replacement")
            .expect("replacement exists")
            .expose_secret(),
        "second-key"
    );

    store.remove().expect("remove key");
    store.remove().expect("removing absent key is safe");
    assert!(store.retrieve().expect("lookup removed key").is_none());
    assert!(!store.exists().expect("check removed key"));
}

#[test]
fn replacement_without_a_configured_credential_is_distinct_from_backend_failure() {
    let store = MemorySecretStore::default();
    assert_eq!(
        store.replace(&SecretCredential::new("new-key")),
        Err(SecretStoreError::NotConfigured)
    );
}

#[test]
fn unavailable_locked_and_failed_storage_are_normalized() {
    for error in [
        SecretStoreError::Unavailable,
        SecretStoreError::Locked,
        SecretStoreError::Failed,
    ] {
        let store = MemorySecretStore::failing_with(error);

        assert_eq!(store.exists(), Err(error));
        assert!(matches!(store.retrieve(), Err(actual) if actual == error));
        assert_eq!(store.save(&SecretCredential::new("secret")), Err(error));
        assert_eq!(store.replace(&SecretCredential::new("secret")), Err(error));
        assert_eq!(store.remove(), Err(error));
    }
}

#[test]
fn credential_debug_output_never_contains_its_value() {
    let credential = SecretCredential::new("do-not-print-this-key");

    assert_eq!(format!("{credential:?}"), "SecretCredential([REDACTED])");
    assert!(!format!("{credential:?}").contains("do-not-print-this-key"));
}
