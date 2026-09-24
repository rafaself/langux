//! Linux Secret Service storage for Langux's Google Translation credential.
//!
//! This crate stores credentials through the desktop Secret Service. It has
//! no plaintext fallback and normalizes backend errors before returning them
//! to callers.

use std::sync::Mutex;

use langux_core::{SecretCredential, SecretStore, SecretStoreError};

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::{GOOGLE_TRANSLATION_SECRET_LABEL, SecretServiceStore};

#[cfg(not(target_os = "linux"))]
pub use unsupported::SecretServiceStore;

trait SecretServiceBackend: Send + Sync {
    fn retrieve(&self) -> Result<Option<String>, SecretStoreError>;
    fn exists(&self) -> Result<bool, SecretStoreError>;
    fn set(&self, credential: &str) -> Result<(), SecretStoreError>;
    fn remove(&self) -> Result<(), SecretStoreError>;
}

static SECRET_STORE_OPERATION_LOCK: Mutex<()> = Mutex::new(());

struct SecretStoreAdapter<B> {
    backend: B,
}

impl<B> SecretStoreAdapter<B>
where
    B: SecretServiceBackend,
{
    fn new(backend: B) -> Self {
        Self { backend }
    }

    fn lock_operation(&self) -> Result<std::sync::MutexGuard<'_, ()>, SecretStoreError> {
        SECRET_STORE_OPERATION_LOCK
            .lock()
            .map_err(|_| SecretStoreError::Failed)
    }
}

impl<B> SecretStore for SecretStoreAdapter<B>
where
    B: SecretServiceBackend,
{
    fn save(&self, credential: &SecretCredential) -> Result<(), SecretStoreError> {
        let _guard = self.lock_operation()?;
        if self.backend.exists()? {
            return Err(SecretStoreError::AlreadyConfigured);
        }
        self.backend.set(credential.expose_secret())
    }

    fn retrieve(&self) -> Result<Option<SecretCredential>, SecretStoreError> {
        let _guard = self.lock_operation()?;
        self.backend
            .retrieve()
            .map(|credential| credential.map(SecretCredential::new))
    }

    fn exists(&self) -> Result<bool, SecretStoreError> {
        let _guard = self.lock_operation()?;
        self.backend.exists()
    }

    fn replace(&self, credential: &SecretCredential) -> Result<(), SecretStoreError> {
        let _guard = self.lock_operation()?;
        if !self.backend.exists()? {
            return Err(SecretStoreError::NotConfigured);
        }
        self.backend.set(credential.expose_secret())
    }

    fn remove(&self) -> Result<(), SecretStoreError> {
        let _guard = self.lock_operation()?;
        match self.backend.remove() {
            Err(SecretStoreError::NotConfigured) => Ok(()),
            result => result,
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod unsupported_impl {
    use super::{SecretServiceBackend, SecretStoreError};

    pub(super) struct UnsupportedBackend;

    impl SecretServiceBackend for UnsupportedBackend {
        fn retrieve(&self) -> Result<Option<String>, SecretStoreError> {
            Err(SecretStoreError::Unavailable)
        }

        fn exists(&self) -> Result<bool, SecretStoreError> {
            Err(SecretStoreError::Unavailable)
        }

        fn set(&self, _: &str) -> Result<(), SecretStoreError> {
            Err(SecretStoreError::Unavailable)
        }

        fn remove(&self) -> Result<(), SecretStoreError> {
            Err(SecretStoreError::Unavailable)
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod unsupported {
    use langux_core::{SecretCredential, SecretStore, SecretStoreError};

    use crate::{SecretStoreAdapter, unsupported_impl::UnsupportedBackend};

    /// A placeholder for targets without Linux Secret Service.
    pub struct SecretServiceStore(SecretStoreAdapter<UnsupportedBackend>);

    impl SecretServiceStore {
        /// Constructs a backend that returns an explicit unavailable error.
        pub fn new() -> Self {
            Self(SecretStoreAdapter::new(UnsupportedBackend))
        }
    }

    impl SecretStore for SecretServiceStore {
        fn save(&self, credential: &SecretCredential) -> Result<(), SecretStoreError> {
            self.0.save(credential)
        }

        fn retrieve(&self) -> Result<Option<SecretCredential>, SecretStoreError> {
            self.0.retrieve()
        }

        fn exists(&self) -> Result<bool, SecretStoreError> {
            self.0.exists()
        }

        fn replace(&self, credential: &SecretCredential) -> Result<(), SecretStoreError> {
            self.0.replace(credential)
        }

        fn remove(&self) -> Result<(), SecretStoreError> {
            self.0.remove()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use langux_core::{SecretCredential, SecretStore, SecretStoreError};

    use crate::{SecretServiceBackend, SecretStoreAdapter};

    #[derive(Default)]
    struct MemoryBackend(Mutex<Option<String>>);

    impl SecretServiceBackend for MemoryBackend {
        fn retrieve(&self) -> Result<Option<String>, SecretStoreError> {
            Ok(self.0.lock().expect("memory backend lock").clone())
        }

        fn exists(&self) -> Result<bool, SecretStoreError> {
            Ok(self.0.lock().expect("memory backend lock").is_some())
        }

        fn set(&self, credential: &str) -> Result<(), SecretStoreError> {
            *self.0.lock().expect("memory backend lock") = Some(credential.to_owned());
            Ok(())
        }

        fn remove(&self) -> Result<(), SecretStoreError> {
            match self.0.lock().expect("memory backend lock").take() {
                Some(_) => Ok(()),
                None => Err(SecretStoreError::NotConfigured),
            }
        }
    }

    #[test]
    fn adapter_saves_detects_replaces_and_removes_credentials() {
        let store = SecretStoreAdapter::new(MemoryBackend::default());
        let first = SecretCredential::new("first-key");
        let replacement = SecretCredential::new("replacement-key");

        assert!(!store.exists().expect("check empty store"));
        assert!(
            store
                .retrieve()
                .expect("retrieve from empty store")
                .is_none()
        );
        assert_eq!(
            store.replace(&replacement),
            Err(SecretStoreError::NotConfigured)
        );

        store.save(&first).expect("save initial key");
        assert!(store.exists().expect("check configured store"));
        assert_eq!(
            store.save(&replacement),
            Err(SecretStoreError::AlreadyConfigured)
        );
        assert_eq!(
            store
                .retrieve()
                .expect("retrieve saved key")
                .expect("credential is saved")
                .expose_secret(),
            "first-key"
        );

        store.replace(&replacement).expect("replace configured key");
        assert_eq!(
            store
                .retrieve()
                .expect("retrieve replacement")
                .expect("replacement is saved")
                .expose_secret(),
            "replacement-key"
        );

        store.remove().expect("remove configured key");
        store.remove().expect("removing absent key is idempotent");
        assert!(!store.exists().expect("check removed key"));
    }
}
