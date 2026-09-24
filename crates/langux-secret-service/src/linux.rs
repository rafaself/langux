use dbus_secret_service::Error as SecretServiceError;
use keyring::{Entry, Error as KeyringError, secret_service::SsCredential};
use langux_core::{
    GOOGLE_TRANSLATION_ACCOUNT_ID, LANGUX_SECRET_SERVICE_ID, SecretCredential, SecretStore,
    SecretStoreError,
};
use rustix::fs::{FlockOperation, Mode, OFlags, flock, open};
use std::{
    env, fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use crate::{SecretServiceBackend, SecretStoreAdapter};

const SECRET_SERVICE_COLLECTION: &str = "Langux";

/// Stable label shown by Secret Service managers for Langux's stored key.
pub const GOOGLE_TRANSLATION_SECRET_LABEL: &str = "Langux Google Cloud Translation API key";

/// Stores Langux credentials using the current user's Linux Secret Service.
///
/// Calls are serialized across processes while mutating credentials. Because
/// Secret Service calls can wait on inter-process communication or an unlock
/// prompt, callers should not invoke these methods on the GTK main thread.
/// There is no file or configuration fallback.
pub struct SecretServiceStore(SecretStoreAdapter<KeyringBackend>);

impl SecretServiceStore {
    /// Constructs the Secret Service entry descriptor without connecting to
    /// the user's session bus.
    pub fn new() -> Result<Self, SecretStoreError> {
        Ok(Self(SecretStoreAdapter::new(KeyringBackend {
            entry: create_entry()?,
        })))
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

struct KeyringBackend {
    entry: Entry,
}

impl SecretServiceBackend for KeyringBackend {
    fn retrieve(&self) -> Result<Option<String>, SecretStoreError> {
        match self.entry.get_password() {
            Ok(credential) => Ok(Some(credential)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(map_keyring_error(error)),
        }
    }

    fn exists(&self) -> Result<bool, SecretStoreError> {
        match self.entry.get_attributes() {
            Ok(_) => Ok(true),
            Err(KeyringError::NoEntry) => Ok(false),
            Err(error) => Err(map_keyring_error(error)),
        }
    }

    fn save(&self, credential: &str) -> Result<(), SecretStoreError> {
        let _lock = MutationLock::acquire()?;
        match self.entry.get_attributes() {
            Ok(_) => return Err(SecretStoreError::AlreadyConfigured),
            Err(KeyringError::NoEntry) => {}
            Err(error) => return Err(map_keyring_error(error)),
        }
        self.entry
            .set_password(credential)
            .map_err(map_keyring_error)
    }

    fn replace(&self, credential: &str) -> Result<(), SecretStoreError> {
        let _lock = MutationLock::acquire()?;
        match self.entry.get_attributes() {
            Ok(_) => {}
            Err(KeyringError::NoEntry) => return Err(SecretStoreError::NotConfigured),
            Err(error) => return Err(map_keyring_error(error)),
        }
        self.entry
            .set_password(credential)
            .map_err(map_keyring_error)
    }

    fn remove(&self) -> Result<(), SecretStoreError> {
        let _lock = MutationLock::acquire()?;
        match self.entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(KeyringError::NoEntry) => Err(SecretStoreError::NotConfigured),
            Err(error) => Err(map_keyring_error(error)),
        }
    }
}

/// A per-login-session lock shared by Langux processes; it holds no secret data.
struct MutationLock {
    _file: fs::File,
}

impl MutationLock {
    fn acquire() -> Result<Self, SecretStoreError> {
        let runtime_dir = env::var_os("XDG_RUNTIME_DIR").ok_or(SecretStoreError::Failed)?;
        let runtime_dir = PathBuf::from(runtime_dir);
        if !runtime_dir.is_absolute() {
            return Err(SecretStoreError::Failed);
        }

        Self::acquire_at(&runtime_dir.join("langux-secret-service.lock"))
    }

    fn acquire_at(path: &Path) -> Result<Self, SecretStoreError> {
        let parent = path.parent().ok_or(SecretStoreError::Failed)?;
        let directory_metadata = fs::metadata(parent).map_err(|_| SecretStoreError::Failed)?;
        if !directory_metadata.is_dir() || directory_metadata.mode() & 0o077 != 0 {
            return Err(SecretStoreError::Failed);
        }

        let fd = open(
            path,
            OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )
        .map_err(|_| SecretStoreError::Failed)?;
        let file = fs::File::from(fd);
        let metadata = file.metadata().map_err(|_| SecretStoreError::Failed)?;
        if !metadata.is_file()
            || metadata.mode() & 0o077 != 0
            || metadata.uid() != directory_metadata.uid()
        {
            return Err(SecretStoreError::Failed);
        }

        loop {
            match flock(&file, FlockOperation::LockExclusive) {
                Ok(()) => return Ok(Self { _file: file }),
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return Err(SecretStoreError::Failed),
            }
        }
    }
}

fn create_entry() -> Result<Entry, SecretStoreError> {
    let mut credential = SsCredential::new_with_target(
        Some(SECRET_SERVICE_COLLECTION),
        LANGUX_SECRET_SERVICE_ID,
        GOOGLE_TRANSLATION_ACCOUNT_ID,
    )
    .map_err(map_keyring_error)?;

    credential.label = GOOGLE_TRANSLATION_SECRET_LABEL.to_owned();
    credential.attributes.insert(
        "application".to_owned(),
        LANGUX_SECRET_SERVICE_ID.to_owned(),
    );

    Ok(Entry::new_with_credential(Box::new(credential)))
}

fn map_keyring_error(error: KeyringError) -> SecretStoreError {
    match error {
        KeyringError::NoEntry => SecretStoreError::NotConfigured,
        KeyringError::NoStorageAccess(source) | KeyringError::PlatformFailure(source) => {
            map_secret_service_error(source.as_ref())
        }
        _ => SecretStoreError::Failed,
    }
}

fn map_secret_service_error(
    error: &(dyn std::error::Error + Send + Sync + 'static),
) -> SecretStoreError {
    match error.downcast_ref::<SecretServiceError>() {
        Some(SecretServiceError::Unavailable) => SecretStoreError::Unavailable,
        Some(SecretServiceError::Locked | SecretServiceError::Prompt) => SecretStoreError::Locked,
        Some(_) | None => SecretStoreError::Failed,
    }
}

#[cfg(test)]
mod tests {
    use dbus_secret_service::Error as SecretServiceError;
    use keyring::Error as KeyringError;
    use langux_core::{GOOGLE_TRANSLATION_ACCOUNT_ID, LANGUX_SECRET_SERVICE_ID, SecretStoreError};
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        sync::mpsc,
        thread,
        time::Duration,
    };

    use super::{
        GOOGLE_TRANSLATION_SECRET_LABEL, MutationLock, SECRET_SERVICE_COLLECTION, create_entry,
        map_keyring_error,
    };

    #[test]
    fn entry_uses_stable_langux_attributes_collection_and_label() {
        let entry = create_entry().expect("create entry descriptor");
        let credential = entry
            .get_credential()
            .downcast_ref::<keyring::secret_service::SsCredential>()
            .expect("Secret Service credential");

        assert_eq!(credential.label, GOOGLE_TRANSLATION_SECRET_LABEL);
        assert_eq!(
            credential.attributes.get("service").map(String::as_str),
            Some(LANGUX_SECRET_SERVICE_ID)
        );
        assert_eq!(
            credential.attributes.get("username").map(String::as_str),
            Some(GOOGLE_TRANSLATION_ACCOUNT_ID)
        );
        assert_eq!(
            credential.attributes.get("target").map(String::as_str),
            Some(SECRET_SERVICE_COLLECTION)
        );
        assert_eq!(
            credential.attributes.get("application").map(String::as_str),
            Some(LANGUX_SECRET_SERVICE_ID)
        );
    }

    #[test]
    fn keyring_failures_are_normalized_without_backend_details() {
        assert_eq!(
            map_keyring_error(KeyringError::PlatformFailure(Box::new(
                SecretServiceError::Unavailable,
            ))),
            SecretStoreError::Unavailable
        );
        assert_eq!(
            map_keyring_error(KeyringError::NoStorageAccess(Box::new(
                SecretServiceError::Locked,
            ))),
            SecretStoreError::Locked
        );
        assert_eq!(
            map_keyring_error(KeyringError::NoStorageAccess(Box::new(
                SecretServiceError::Prompt,
            ))),
            SecretStoreError::Locked
        );
        assert_eq!(
            map_keyring_error(KeyringError::PlatformFailure(Box::new(
                SecretServiceError::Path("do-not-leak-this-key".to_owned()),
            ))),
            SecretStoreError::Failed
        );
        assert_eq!(
            map_keyring_error(KeyringError::NoEntry),
            SecretStoreError::NotConfigured
        );
        assert_eq!(
            SecretStoreError::Unavailable.to_string(),
            "Secret storage is unavailable."
        );
        assert_eq!(
            SecretStoreError::Locked.to_string(),
            "Secret storage is locked."
        );
        assert_eq!(
            SecretStoreError::Failed.to_string(),
            "Secret storage could not complete the operation."
        );
        assert!(
            !SecretStoreError::Failed
                .to_string()
                .contains("do-not-leak-this-key")
        );
    }

    #[test]
    fn mutation_lock_serializes_independent_handles() {
        static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(0);

        let dir = std::env::temp_dir().join(format!(
            "langux-secret-service-test-{}-{}",
            std::process::id(),
            NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).expect("create private lock directory");
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))
            .expect("secure lock directory");
        let path = PathBuf::from(&dir).join("lock");
        let first = MutationLock::acquire_at(&path).expect("acquire first lock");
        let (started_tx, started_rx) = mpsc::channel();
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let second_path = path.clone();
        let second = thread::spawn(move || {
            started_tx.send(()).expect("notify lock attempt");
            let _lock = MutationLock::acquire_at(&second_path).expect("acquire second lock");
            acquired_tx.send(()).expect("notify lock acquired");
        });

        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("second lock attempt started");
        assert!(
            acquired_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err()
        );
        drop(first);
        acquired_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("second lock acquired after release");
        second.join().expect("join lock worker");
        fs::remove_dir_all(dir).expect("remove test lock directory");
    }
}
