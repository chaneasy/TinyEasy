use crate::{config, error::Error};
use chacha20poly1305::{
    ChaCha20Poly1305, KeyInit,
    aead::{Aead, OsRng, rand_core::RngCore},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path};
use zeroize::{Zeroize, Zeroizing};

#[derive(Clone, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct Account {
    pub key: String,
    pub note: String,
}
impl std::fmt::Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Account { key: [REDACTED], note: [REDACTED] }")
    }
}
pub fn normalize(accounts: &[Account]) -> Result<Vec<Account>, (usize, Error)> {
    if accounts.len() > 50 {
        return Err((0, Error::KeyLimit));
    }
    let mut seen = HashSet::new();
    accounts
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let key = a.key.trim();
            let note = a.note.trim();
            if key.is_empty() || key.len() > 256 || !key.bytes().all(|b| b.is_ascii_alphanumeric())
            {
                return Err((i, Error::KeyFormat));
            }
            if !seen.insert(key) {
                return Err((i, Error::KeyDuplicate));
            }
            if note.chars().count() > 100 {
                return Err((i, Error::NoteLimit));
            }
            Ok(Account {
                key: key.to_owned(),
                note: if note.is_empty() {
                    format!("Key {}", i + 1)
                } else {
                    note.to_owned()
                },
            })
        })
        .collect()
}
pub trait Vault {
    fn master(&self, create: bool) -> Result<Zeroizing<Vec<u8>>, Error>;
}
pub struct SystemVault;
impl Vault for SystemVault {
    fn master(&self, create: bool) -> Result<Zeroizing<Vec<u8>>, Error> {
        let entry = keyring::Entry::new("com.tinyeasy.imagecompressor.rust", "keys-master-v1")
            .map_err(|_| Error::SecureStorage)?;
        match entry.get_secret() {
            Ok(secret) if secret.len() == 32 => Ok(Zeroizing::new(secret)),
            Err(keyring::Error::NoEntry) if create => {
                let mut secret = Zeroizing::new(vec![0; 32]);
                OsRng.fill_bytes(&mut secret);
                entry
                    .set_secret(&secret)
                    .map_err(|_| Error::SecureStorage)?;
                // Verify the platform provider really persisted it.
                let readback =
                    Zeroizing::new(entry.get_secret().map_err(|_| Error::SecureStorage)?);
                if readback.as_slice() != secret.as_slice() {
                    return Err(Error::SecureStorage);
                }
                Ok(secret)
            }
            _ => Err(Error::SecureStorage),
        }
    }
}
const MAGIC: &[u8] = b"TinyEasyKeys\x01";
fn seal(accounts: &[Account], master: &[u8]) -> Result<Vec<u8>, Error> {
    let data = Zeroizing::new(serde_json::to_vec(accounts).map_err(|_| Error::Configuration)?);
    let cipher = ChaCha20Poly1305::new_from_slice(master).map_err(|_| Error::SecureStorage)?;
    let mut nonce = [0; 12];
    OsRng.fill_bytes(&mut nonce);
    let encrypted = cipher
        .encrypt((&nonce).into(), data.as_slice())
        .map_err(|_| Error::SecureStorage)?;
    Ok([MAGIC, &nonce, &encrypted].concat())
}
fn unseal(data: &[u8], master: &[u8]) -> Result<Vec<Account>, Error> {
    if !data.starts_with(MAGIC) || data.len() < MAGIC.len() + 12 + 16 {
        return Err(Error::Configuration);
    }
    let cipher = ChaCha20Poly1305::new_from_slice(master).map_err(|_| Error::SecureStorage)?;
    let offset = MAGIC.len();
    let plaintext = Zeroizing::new(
        cipher
            .decrypt(data[offset..offset + 12].into(), &data[offset + 12..])
            .map_err(|_| Error::Configuration)?,
    );
    let accounts: Vec<Account> =
        serde_json::from_slice(&plaintext).map_err(|_| Error::Configuration)?;
    normalize(&accounts).map_err(|(_, e)| e)
}
pub fn environment(multiple: &str, single: &str) -> Result<Vec<Account>, Error> {
    let mut seen = HashSet::new();
    let accounts = multiple
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .chain(single.split(|c: char| c.is_whitespace() || c == ',' || c == ';'))
        .filter(|key| !key.is_empty() && seen.insert(*key))
        .map(|key| Account {
            key: key.to_owned(),
            note: String::new(),
        })
        .collect::<Vec<_>>();
    normalize(&accounts).map_err(|(_, e)| e)
}
pub fn load(
    path: &Path,
    vault: &impl Vault,
    env: impl FnOnce() -> Result<Vec<Account>, Error>,
) -> Result<Vec<Account>, Error> {
    match std::fs::read(path) {
        Ok(data) => unseal(&data, &vault.master(false)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => env(),
        Err(_) => Err(Error::Configuration),
    }
}
pub fn load_system(path: &Path) -> Result<Vec<Account>, Error> {
    load(path, &SystemVault, || {
        if config::legacy_exists() {
            return Err(Error::Legacy);
        }
        let multiple = Zeroizing::new(std::env::var("TINIFY_API_KEYS").unwrap_or_default());
        let single = Zeroizing::new(std::env::var("TINIFY_API_KEY").unwrap_or_default());
        environment(&multiple, &single)
    })
}
pub fn save(path: &Path, accounts: &[Account], vault: &impl Vault) -> Result<Vec<Account>, Error> {
    let normalized = normalize(accounts).map_err(|(_, e)| e)?;
    // Even an empty configuration is encrypted and overrides environment fallback.
    let master = vault.master(!path.exists())?;
    if path.exists() {
        let old = std::fs::read(path)?;
        if unseal(&old, &master).is_err() {
            // Preserve unreadable data before an explicit user-initiated repair.
            let parent = path.parent().ok_or(Error::File)?;
            let mut backup = tempfile::Builder::new()
                .prefix("keys-recovery-")
                .tempfile_in(parent)?;
            use std::io::Write;
            backup.write_all(&old)?;
            backup.as_file().sync_all()?;
            backup.keep().map_err(|_| Error::File)?;
        }
    }
    config::atomic_write(path, &seal(&normalized, &master)?)?;
    Ok(normalized)
}
#[cfg(test)]
mod tests {
    use super::*;
    struct MemoryVault {
        secret: Zeroizing<Vec<u8>>,
        available: bool,
    }
    impl MemoryVault {
        fn new() -> Self {
            let mut secret = Zeroizing::new(vec![0; 32]);
            OsRng.fill_bytes(&mut secret);
            Self {
                secret,
                available: true,
            }
        }
    }
    impl Vault for MemoryVault {
        fn master(&self, _: bool) -> Result<Zeroizing<Vec<u8>>, Error> {
            if self.available {
                Ok(self.secret.clone())
            } else {
                Err(Error::SecureStorage)
            }
        }
    }
    fn account() -> Account {
        let mut bytes = [0; 24];
        OsRng.fill_bytes(&mut bytes);
        Account {
            key: bytes.iter().map(|b| format!("{b:02x}")).collect(),
            note: String::new(),
        }
    }
    #[test]
    fn encrypted_roundtrip_and_empty_configuration_override_environment() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keys.enc");
        let vault = MemoryVault::new();
        let a = account();
        save(&path, std::slice::from_ref(&a), &vault).unwrap();
        assert!(!String::from_utf8_lossy(&std::fs::read(&path).unwrap()).contains(&a.key));
        assert_eq!(
            load(&path, &vault, || panic!("local takes precedence")).unwrap()[0].key,
            a.key
        );
        save(&path, &[], &vault).unwrap();
        assert!(
            load(&path, &vault, || panic!("empty takes precedence"))
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn storage_failure_preserves_saved_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keys.enc");
        let mut vault = MemoryVault::new();
        save(&path, &[account()], &vault).unwrap();
        let before = std::fs::read(&path).unwrap();
        vault.available = false;
        assert_eq!(save(&path, &[], &vault).unwrap_err(), Error::SecureStorage);
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    #[test]
    fn rejects_duplicates_malformed_keys_and_oversized_notes() {
        let a = account();
        assert_eq!(
            normalize(&[a.clone(), a.clone()]).unwrap_err().1,
            Error::KeyDuplicate
        );
        assert_eq!(
            normalize(&[Account {
                key: String::new(),
                note: String::new()
            }])
            .unwrap_err()
            .1,
            Error::KeyFormat
        );
        assert_eq!(
            normalize(&vec![a.clone(); 51]).unwrap_err().1,
            Error::KeyLimit
        );
        assert_eq!(
            normalize(&[Account {
                key: a.key.clone(),
                note: "中".repeat(101)
            }])
            .unwrap_err()
            .1,
            Error::NoteLimit
        );
    }
    #[test]
    fn corrupted_ciphertext_never_uses_environment() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keys.enc");
        std::fs::write(&path, b"corrupt").unwrap();
        assert!(load(&path, &MemoryVault::new(), || panic!("no fallback")).is_err());
    }
}

#[cfg(test)]
mod environment_tests {
    use super::*;
    #[test]
    fn environment_trims_and_deduplicates_both_variables() {
        use chacha20poly1305::aead::{OsRng, rand_core::RngCore};
        let mut bytes = [0; 24];
        OsRng.fill_bytes(&mut bytes);
        let first = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        OsRng.fill_bytes(&mut bytes);
        let second = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let accounts = environment(&format!(" {first} , {second}; {first}\n"), &second).unwrap();
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].key, first);
        assert_eq!(accounts[1].key, second);
    }
}

#[cfg(test)]
mod native_test {
    use super::*;
    #[test]
    #[ignore = "explicit native keychain roundtrip using a temporary credential, no user API Keys"]
    fn native_vault_roundtrip() {
        let mut random = [0; 32];
        OsRng.fill_bytes(&mut random);
        let identifier = random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let entry = keyring::Entry::new(
            &format!("com.tinyeasy.test.{identifier}"),
            "temporary-master",
        )
        .unwrap();
        OsRng.fill_bytes(&mut random);
        entry.set_secret(&random).unwrap();
        let result = entry.get_secret();
        // Clean up only the entry created by this test, even if readback failed.
        let cleanup = entry.delete_credential();
        assert!(result.is_ok_and(|secret| secret.as_slice() == random));
        assert!(cleanup.is_ok());
    }
}
