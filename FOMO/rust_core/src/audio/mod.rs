// privmesh_core/src/audio/mod.rs
//
// Opus audio encoding/decoding for encrypted voice calls.
// SRTP framing is handled by flutter_webrtc on the Dart side;
// this module handles the raw codec work in Rust.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Encoder initialisation failed: {0}")]
    EncoderInit(String),
    #[error("Encoding failed: {0}")]
    Encode(String),
    #[error("Decoding failed: {0}")]
    Decode(String),
}

// Audio constants matching WebRTC defaults
pub const SAMPLE_RATE:   u32 = 48_000;  // Hz
pub const CHANNELS:      u8  = 1;       // mono for voice
pub const FRAME_MS:      u32 = 20;      // 20 ms frames (960 samples @ 48kHz)
pub const FRAME_SAMPLES: usize = (SAMPLE_RATE as usize * FRAME_MS as usize) / 1000;

/// Encode one 20 ms PCM frame into Opus bytes.
/// input: FRAME_SAMPLES i16 samples (960 @ 48kHz mono)
/// Returns compressed Opus packet, typically 20–80 bytes for voice.
pub fn encode_frame(pcm: &[i16]) -> Result<Vec<u8>, AudioError> {
    // Placeholder: real implementation uses the `opus` crate encoder.
    // opus::Encoder::new(SAMPLE_RATE, opus::Channels::Mono, opus::Application::Voip)
    //     .encode(pcm, &mut out_buf)
    let _ = pcm;
    Ok(vec![0u8; 40]) // stub — 40-byte silence packet
}

/// Decode one Opus packet back to PCM.
/// Returns FRAME_SAMPLES i16 samples.
pub fn decode_frame(packet: &[u8]) -> Result<Vec<i16>, AudioError> {
    // Placeholder: real implementation uses the `opus` crate decoder.
    let _ = packet;
    Ok(vec![0i16; FRAME_SAMPLES]) // stub — silence
}
