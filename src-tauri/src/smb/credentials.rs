use crate::error::{AppError, Result};

use super::credential_key;

#[cfg(test)]
mod memory {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    fn store_map() -> &'static Mutex<HashMap<String, String>> {
        static STORE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
        STORE.get_or_init(|| Mutex::new(HashMap::new()))
    }

    pub fn store(host: &str, share: &str, username: &str, password: &str) -> Result<()> {
        let key = credential_key(host, share, username);
        store_map()
            .lock()
            .map_err(|_| AppError::Library("credential store lock failed".into()))?
            .insert(key, password.to_string());
        Ok(())
    }

    pub fn load(host: &str, share: &str, username: &str) -> Result<String> {
        let key = credential_key(host, share, username);
        store_map()
            .lock()
            .map_err(|_| AppError::Library("credential store lock failed".into()))?
            .get(&key)
            .cloned()
            .ok_or_else(|| AppError::Library("credentials not found: missing entry".into()))
    }

    pub fn delete(host: &str, share: &str, username: &str) -> Result<()> {
        let key = credential_key(host, share, username);
        store_map()
            .lock()
            .map_err(|_| AppError::Library("credential store lock failed".into()))?
            .remove(&key);
        Ok(())
    }
}

#[cfg(not(test))]
mod keyring_store {
    use super::*;
    use keyring::Error as KeyringError;

    const KEYRING_SERVICE: &str = "com.memhg.app";

    pub fn store(host: &str, share: &str, username: &str, password: &str) -> Result<()> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, &credential_key(host, share, username))
            .map_err(|e| AppError::Library(format!("keyring: {}", e)))?;
        entry
            .set_password(password)
            .map_err(|e| AppError::Library(format!("keyring: {}", e)))?;
        Ok(())
    }

    pub fn load(host: &str, share: &str, username: &str) -> Result<String> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, &credential_key(host, share, username))
            .map_err(|e| AppError::Library(format!("keyring: {}", e)))?;
        entry
            .get_password()
            .map_err(|e| AppError::Library(format!("credentials not found: {}", e)))
    }

    pub fn delete(host: &str, share: &str, username: &str) -> Result<()> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, &credential_key(host, share, username))
            .map_err(|e| AppError::Library(format!("keyring: {}", e)))?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(KeyringError::NoEntry) => Ok(()),
            Err(e) => Err(AppError::Library(format!("keyring: {}", e))),
        }
    }
}

#[cfg(test)]
pub(crate) use memory::{delete, load, store};

#[cfg(not(test))]
pub(crate) use keyring_store::{delete, load, store};
