// privmesh_core/src/storage/mod.rs
//
// Encrypted message storage layer.
// On the Dart side, sqflite_sqlcipher opens the AES-256 encrypted database.
// This module provides key derivation and serialisation helpers that Dart
// calls via the FFI bridge before opening the database.

use crate::crypto::derive_db_key;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),
    #[error("Serialisation failed: {0}")]
    Serialisation(String),
}

/// A stored message record (mirrors the SQLite schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub id:           String,     // UUID v4
    pub peer_pubkey:  [u8; 32],   // Ed25519 pubkey of the other party
    pub ciphertext:   Vec<u8>,    // encrypted payload (nonce + AES-GCM bytes)
    pub timestamp_ms: u64,
    pub sent:         bool,
}

/// Derive a 32-byte database key from a PIN and a stored salt.
/// This is the only storage function exposed over FFI — the actual
/// database file is opened by Flutter's sqflite_sqlcipher plugin
/// using the key returned here.
///
/// pin: user-entered PIN string
/// salt: 16 random bytes generated on first launch, stored in flutter_secure_storage
pub fn derive_storage_key(pin: &str, salt: &[u8; 16]) -> [u8; 32] {
    derive_db_key(pin, salt, 600_000)
}

/// Serialise a MessageRecord to bytes for storage.
pub fn serialise_record(record: &MessageRecord) -> Result<Vec<u8>, StorageError> {
    serde_json::to_vec(record).map_err(|e| StorageError::Serialisation(e.to_string()))
}

/// Deserialise a MessageRecord from bytes.
pub fn deserialise_record(bytes: &[u8]) -> Result<MessageRecord, StorageError> {
    serde_json::from_slice(bytes).map_err(|e| StorageError::Serialisation(e.to_string()))
}
