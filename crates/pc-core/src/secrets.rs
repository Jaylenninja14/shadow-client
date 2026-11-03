//! Secret storage protected with Windows DPAPI (bound to the current Windows user).
//!
//! Holds Microsoft refresh tokens, cached Minecraft access tokens and the CurseForge API key.
//! Secrets are never logged and never written in plain text. On non-Windows builds (only used
//! for CI type-checking) storage is refused rather than silently stored in the clear.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::{util, Error, Result};

pub const CURSEFORGE_KEY: &str = "curseforge_api_key";
pub const ACCOUNTS_KEY: &str = "accounts";

pub struct SecretStore {
    path: PathBuf,
    lock: Mutex<()>,
}

impl SecretStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path, lock: Mutex::new(()) }
    }

    fn load(&self) -> Result<BTreeMap<String, String>> {
        let bytes = match std::fs::read(&self.path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
            Err(e) => return Err(e.into()),
        };
        let plain = unprotect(&bytes)?;
        Ok(serde_json::from_slice(&plain)?)
    }

    fn store(&self, map: &BTreeMap<String, String>) -> Result<()> {
        let plain = serde_json::to_vec(map)?;
        let enc = protect(&plain)?;
        util::atomic_write(&self.path, &enc)
    }

    pub fn get(&self, key: &str) -> Result<Option<String>> {
        let _g = self.lock.lock().unwrap();
        Ok(self.load()?.get(key).cloned())
    }

    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        let _g = self.lock.lock().unwrap();
        let mut map = self.load().unwrap_or_default();
        map.insert(key.to_string(), value.to_string());
        self.store(&map)
    }

    pub fn remove(&self, key: &str) -> Result<()> {
        let _g = self.lock.lock().unwrap();
        let mut map = self.load().unwrap_or_default();
        map.remove(key);
        self.store(&map)
    }

    pub fn has(&self, key: &str) -> bool {
        self.get(key).ok().flatten().is_some_and(|v| !v.is_empty())
    }
}

#[cfg(windows)]
fn protect(data: &[u8]) -> Result<Vec<u8>> {
    use windows_sys::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN};
    unsafe {
        let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
        let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
        let ok = CryptProtectData(&input, std::ptr::null(), std::ptr::null(), std::ptr::null(), std::ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut out);
        if ok == 0 {
            return Err(Error::msg("Windows refused to encrypt the secret store (DPAPI)"));
        }
        let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        windows_sys::Win32::Foundation::LocalFree(out.pbData as _);
        Ok(v)
    }
}

#[cfg(windows)]
fn unprotect(data: &[u8]) -> Result<Vec<u8>> {
    use windows_sys::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN};
    unsafe {
        let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
        let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
        let ok = CryptUnprotectData(&input, std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut out);
        if ok == 0 {
            return Err(Error::msg("The secret store could not be decrypted (was it created by another Windows user?)"));
        }
        let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        windows_sys::Win32::Foundation::LocalFree(out.pbData as _);
        Ok(v)
    }
}

#[cfg(not(windows))]
fn protect(_: &[u8]) -> Result<Vec<u8>> {
    Err(Error::msg("Secure secret storage is only implemented on Windows"))
}

#[cfg(not(windows))]
fn unprotect(_: &[u8]) -> Result<Vec<u8>> {
    Err(Error::msg("Secure secret storage is only implemented on Windows"))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_is_encrypted_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let store = SecretStore::new(dir.path().join("secrets.dat"));
        store.set(CURSEFORGE_KEY, "super-secret-value-123").unwrap();
        assert_eq!(store.get(CURSEFORGE_KEY).unwrap().as_deref(), Some("super-secret-value-123"));
        let raw = std::fs::read(dir.path().join("secrets.dat")).unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("super-secret"), "plain text must not hit the disk");
        store.remove(CURSEFORGE_KEY).unwrap();
        assert!(!store.has(CURSEFORGE_KEY));
    }
}
