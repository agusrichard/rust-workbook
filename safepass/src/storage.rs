use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng, generic_array::GenericArray},
    Aes256Gcm, Key,
};
use base64::{engine::general_purpose, Engine as _};
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

pub fn get_store_path() -> PathBuf {
    let mut path = dirs::home_dir().expect("Could not find home directory");
    path.push(".safepass");
    if !path.exists() {
        fs::create_dir(&path).ok();
    }
    path.push("store.json");
    path
}

pub fn load_entries() -> io::Result<Vec<PasswordEntry>> {
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
            // If we can't deserialize, return empty (or handle migration in future)
            // For now, let's assume if it fails it might be old format or corrupt.
            // Returning empty might be dangerous (overwriting), so let's return error
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Could not parse storage file",
            ))
        }
    }
}

pub fn save_entry(
    service: &str,
    username: &str,
    password: &str,
    master_password: &str,
) -> io::Result<()> {
    let mut entries = match load_entries() {
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

pub fn update_entry(
    service: &str,
    username: &str,
    new_password: &str,
    master_password: &str,
) -> io::Result<()> {
    let mut entries = load_entries()?;
    
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
            format!("Service '{}' with username '{}' not found", service, username),
        ))
    }
}

pub fn delete_entry(service: &str, username: &str) -> io::Result<()> {
    let mut entries = load_entries()?;
    let original_len = entries.len();
    
    entries.retain(|e| e.service != service || e.username != username);
    
    if entries.len() == original_len {
        return Err(io::Error::new(
            io::ErrorKind::NotFound, 
            format!("Service '{}' with username '{}' not found", service, username)
        ));
    }

    let path = get_store_path();
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &entries)?;

    Ok(())
}
