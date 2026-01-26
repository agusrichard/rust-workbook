use aes_gcm::{
    Aes256Gcm, Key,
    aead::{Aead, AeadCore, KeyInit, OsRng, generic_array::GenericArray},
};
use base64::{Engine as _, engine::general_purpose};
use hmac::Hmac;
use pbkdf2::pbkdf2;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PasswordEntry {
    pub service: String,
    pub username: String,
    pub encrypted_password: String,
    pub salt: String,
    pub nonce: String,
}

impl PasswordEntry {
    pub fn decrypt(&self, master_password: &str) -> io::Result<String> {
        let salt = general_purpose::STANDARD
            .decode(&self.salt)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        let nonce_bytes = general_purpose::STANDARD
            .decode(&self.nonce)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        let ciphertext = general_purpose::STANDARD
            .decode(&self.encrypted_password)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        let mut key = [0u8; 32];
        pbkdf2::<Hmac<Sha256>>(master_password.as_bytes(), &salt, 100_000, &mut key)
            .expect("PBKDF2 failed");

        let key = Key::<Aes256Gcm>::from_slice(&key);
        let cipher = Aes256Gcm::new(key);
        let nonce = GenericArray::from_slice(&nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        String::from_utf8(plaintext)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    }
}

pub trait Storage {
    fn load_entries(&self) -> io::Result<Vec<PasswordEntry>>;
    fn save_entry(
        &self,
        service: &str,
        username: &str,
        password: &str,
        master_password: &str,
    ) -> io::Result<()>;
    fn update_entry(
        &self,
        service: &str,
        username: &str,
        new_password: &str,
        master_password: &str,
    ) -> io::Result<()>;
    fn delete_entry(&self, service: &str, username: &str) -> io::Result<()>;
}

pub struct FileStorage;

impl FileStorage {
    pub fn new() -> Self {
        Self
    }
}

fn get_store_path() -> PathBuf {
    let mut path = match std::env::var("SAFEPASS_STORE_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => dirs::home_dir().expect("Could not find home directory"),
    };

    // If we are using the default home dir, append .safepass
    if std::env::var("SAFEPASS_STORE_DIR").is_err() {
        path.push(".safepass");
    }

    if !path.exists() {
        fs::create_dir_all(&path).ok();
    }
    path.push("store.json");
    path
}

impl Storage for FileStorage {
    fn load_entries(&self) -> io::Result<Vec<PasswordEntry>> {
        let path = get_store_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(path)?;
        // Handle empty file
        if file.metadata()?.len() == 0 {
            return Ok(Vec::new());
        }

        let reader = BufReader::new(file);
        match serde_json::from_reader(reader) {
            Ok(entries) => Ok(entries),
            Err(_) => {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Could not parse storage file",
                ))
            }
        }
    }

    fn save_entry(
        &self,
        service: &str,
        username: &str,
        password: &str,
        master_password: &str,
    ) -> io::Result<()> {
        let mut entries = match self.load_entries() {
            Ok(e) => e,
            Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                return Err(e);
            }
            Err(e) => return Err(e),
        };

        if entries
            .iter()
            .any(|e| e.service == service && e.username == username)
        {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "Service '{}' with username '{}' already exists",
                    service, username
                ),
            ));
        }

        // Generate Salt
        let mut salt = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut salt);

        // Derive Key
        let mut key = [0u8; 32]; // AES-256
        pbkdf2::<Hmac<Sha256>>(master_password.as_bytes(), &salt, 100_000, &mut key)
            .expect("PBKDF2 failed");
        let key = Key::<Aes256Gcm>::from_slice(&key);
        let cipher = Aes256Gcm::new(key);

        // Generate Nonce
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message

        // Encrypt
        let ciphertext = cipher
            .encrypt(&nonce, password.as_bytes())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let entry = PasswordEntry {
            service: service.to_string(),
            username: username.to_string(),
            encrypted_password: general_purpose::STANDARD.encode(ciphertext),
            salt: general_purpose::STANDARD.encode(salt),
            nonce: general_purpose::STANDARD.encode(nonce),
        };

        entries.push(entry);

        let path = get_store_path();
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &entries)?;

        Ok(())
    }

    fn update_entry(
        &self,
        service: &str,
        username: &str,
        new_password: &str,
        master_password: &str,
    ) -> io::Result<()> {
        let mut entries = self.load_entries()?;

        let index = entries
            .iter()
            .position(|e| e.service == service && e.username == username);

        if let Some(idx) = index {
            // Generate Salt
            let mut salt = [0u8; 16];
            rand::thread_rng().fill_bytes(&mut salt);

            // Derive Key
            let mut key = [0u8; 32]; // AES-256
            pbkdf2::<Hmac<Sha256>>(master_password.as_bytes(), &salt, 100_000, &mut key)
                .expect("PBKDF2 failed");
            let key = Key::<Aes256Gcm>::from_slice(&key);
            let cipher = Aes256Gcm::new(key);

            // Generate Nonce
            let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message

            // Encrypt
            let ciphertext = cipher
                .encrypt(&nonce, new_password.as_bytes())
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

            let new_entry = PasswordEntry {
                service: service.to_string(),
                username: username.to_string(),
                encrypted_password: general_purpose::STANDARD.encode(ciphertext),
                salt: general_purpose::STANDARD.encode(salt),
                nonce: general_purpose::STANDARD.encode(nonce),
            };

            entries[idx] = new_entry;

            let path = get_store_path();
            let file = File::create(path)?;
            let writer = BufWriter::new(file);
            serde_json::to_writer_pretty(writer, &entries)?;

            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "Service '{}' with username '{}' not found",
                    service, username
                ),
            ))
        }
    }

    fn delete_entry(&self, service: &str, username: &str) -> io::Result<()> {
        let mut entries = self.load_entries()?;
        let original_len = entries.len();

        entries.retain(|e| e.service != service || e.username != username);

        if entries.len() == original_len {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "Service '{}' with username '{}' not found",
                    service, username
                ),
            ));
        }

        let path = get_store_path();
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &entries)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    fn setup_test_env() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            env::set_var("SAFEPASS_STORE_DIR", temp_dir.path());
        }
        temp_dir
    }

    #[test]
    fn test_storage_lifecycle() {
        let _temp_dir = setup_test_env();
        let master_password = "master_secret";
        let storage = FileStorage::new();

        // 1. Test Save and Load
        storage.save_entry("google", "user@gmail.com", "password123", master_password).unwrap();

        let entries = storage.load_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].service, "google");
        assert_eq!(entries[0].username, "user@gmail.com");

        let decrypted = entries[0].decrypt(master_password).unwrap();
        assert_eq!(decrypted, "password123");

        // 2. Test Duplicate
        storage.save_entry("google", "user2", "pass2", master_password).unwrap();
        
        let result = storage.save_entry("google", "user@gmail.com", "pass3", master_password);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::AlreadyExists);

        // 3. Test Update
        storage.save_entry("github", "dev", "old_pass", master_password).unwrap();
        storage.update_entry("github", "dev", "new_pass", master_password).unwrap();

        let entries = storage.load_entries().unwrap();
        let github_entry = entries.iter().find(|e| e.service == "github").unwrap();
        let decrypted = github_entry.decrypt(master_password).unwrap();
        assert_eq!(decrypted, "new_pass");

        // 4. Test Delete
        storage.save_entry("netflix", "watcher", "12345", master_password).unwrap();
        storage.delete_entry("netflix", "watcher").unwrap();

        let entries = storage.load_entries().unwrap();
        assert!(entries.iter().all(|e| e.service != "netflix"));
    }
}