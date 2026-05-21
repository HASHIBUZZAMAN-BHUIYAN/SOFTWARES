// privmesh_core/src/lib.rs
//
// FFI boundary — these are the ONLY functions Flutter/Dart can call.
// All types crossing the boundary must be C-safe (primitives, raw pointers).
// flutter_rust_bridge generates the Dart wrappers automatically from this file.
//
// Run to regenerate bindings after changing this file:
//   flutter_rust_bridge_codegen generate

pub mod crypto;
pub mod network;
pub mod audio;
pub mod storage;

use flutter_rust_bridge::frb;
use crate::crypto::{
    Identity, PlainMessage, EncryptedPacket,
    encrypt_message, decrypt_message, verify_signature, derive_db_key,
};

// ─────────────────────────────────────────────────────────────────────────────
// IDENTITY FUNCTIONS (called from Dart)
// ─────────────────────────────────────────────────────────────────────────────

/// Generate a new Ed25519 identity. Returns the 32-byte seed for secure storage.
/// Call once on first app launch. Store result in flutter_secure_storage.
#[frb]
pub fn api_generate_identity() -> Vec<u8> {
    let identity = Identity::generate();
    identity.export_seed().to_vec()
}

/// Export the public identity (nickname + pubkeys) for NFC sharing.
/// seed: the stored 32-byte seed from api_generate_identity().
#[frb]
pub fn api_get_public_identity(
    seed: Vec<u8>,
    nickname: String,
    group_token: Vec<u8>,
) -> Vec<u8> {
    let seed_arr: [u8; 32] = seed.try_into().expect("seed must be 32 bytes");
    let token_arr: [u8; 32] = group_token.try_into().expect("token must be 32 bytes");
    let identity = Identity::from_seed(&seed_arr);
    let public = identity.public_identity(nickname, token_arr);
    serde_json::to_vec(&public).expect("serialization ok")
}

// ─────────────────────────────────────────────────────────────────────────────
// CRYPTO FUNCTIONS
// ─────────────────────────────────────────────────────────────────────────────

/// Encrypt a text message. Returns raw bytes of the encrypted packet.
/// shared_secret: 32-byte result from api_derive_shared_secret().
#[frb]
pub fn api_encrypt_message(
    text: String,
    shared_secret: Vec<u8>,
    sender_id: Vec<u8>,
    timestamp_ms: u64,
) -> Result<Vec<u8>, String> {
    let secret: [u8; 32] = shared_secret.try_into().map_err(|_| "bad key")?;
    let sid: [u8; 32]    = sender_id.try_into().map_err(|_| "bad sender_id")?;
    let msg = PlainMessage { text, timestamp_ms, sender_id: sid };
    let packet = encrypt_message(&msg, &secret).map_err(|e| e.to_string())?;
    // Pack nonce + ciphertext into a single Vec
    let mut out = Vec::with_capacity(12 + packet.ciphertext.len());
    out.extend_from_slice(&packet.nonce);
    out.extend_from_slice(&packet.ciphertext);
    Ok(out)
}

/// Derive the shared ECDH secret between us and a peer. O(1)
/// our_seed: our 32-byte identity seed.
/// their_x25519_pubkey: from their PublicIdentity (received via NFC).
#[frb]
pub fn api_derive_shared_secret(
    our_seed: Vec<u8>,
    their_x25519_pubkey: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let seed: [u8; 32]   = our_seed.try_into().map_err(|_| "bad seed")?;
    let their: [u8; 32]  = their_x25519_pubkey.try_into().map_err(|_| "bad pubkey")?;
    let secret = crypto::derive_shared_secret(&seed, &their);
    Ok(secret.to_vec())
}

/// Derive database encryption key from PIN. Slow by design (600k iterations).
/// salt: 16 random bytes, stored alongside the encrypted database.
#[frb]
pub fn api_derive_db_key(pin: String, salt: Vec<u8>) -> Result<Vec<u8>, String> {
    let salt_arr: [u8; 16] = salt.try_into().map_err(|_| "salt must be 16 bytes")?;
    let key = derive_db_key(&pin, &salt_arr, 600_000);
    Ok(key.to_vec())
}

/// Sign data with our identity key (for admin revocation broadcasts).
#[frb]
pub fn api_sign(seed: Vec<u8>, data: Vec<u8>) -> Result<Vec<u8>, String> {
    let seed_arr: [u8; 32] = seed.try_into().map_err(|_| "bad seed")?;
    let identity = Identity::from_seed(&seed_arr);
    let sig = identity.sign(&data);
    Ok(sig.to_vec())
}

/// Verify a signature. Returns true if valid.
#[frb]
pub fn api_verify_signature(
    pubkey: Vec<u8>,
    message: Vec<u8>,
    signature: Vec<u8>,
) -> bool {
    let pk: [u8; 32]  = match pubkey.try_into()    { Ok(v) => v, Err(_) => return false };
    let sig: [u8; 64] = match signature.try_into() { Ok(v) => v, Err(_) => return false };
    verify_signature(&pk, &message, &sig).is_ok()
}
