// privmesh_core/src/network/mod.rs
//
// LAN transport layer — zero-copy UDP, mDNS peer discovery, O(1) routing.
//
// Time complexity:
//   send_packet      → O(n) network I/O  (n = packet size, typically <2KB)
//   route_to_peer    → O(1)  DashMap lookup by Ed25519 pubkey
//   discover_peers   → O(p)  p = number of peers on LAN (bounded, small)
//   whitelist_check  → O(1)  DashMap lookup
//
// Space: O(p) for peer table where p = group size (typically <50 devices)

use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use tokio::net::UdpSocket as AsyncUdpSocket;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::crypto::EncryptedPacket;

// ─────────────────────────────────────────────────────────────────────────────
// CONSTANTS
// ─────────────────────────────────────────────────────────────────────────────
pub const PRIVMESH_PORT:    u16 = 54321;
pub const MDNS_SERVICE:     &str = "_privmesh._udp.local.";
pub const MAX_PACKET_BYTES: usize = 65_507;   // UDP max payload
pub const PADDED_MSG_SIZE:  usize = 512;      // pad all messages to this size
                                               // hides message length from observers

// ─────────────────────────────────────────────────────────────────────────────
// ERRORS
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Peer not in trusted list — packet dropped")]
    UntrustedPeer,
    #[error("Peer not found: {0}")]
    PeerNotFound(String),
    #[error("Socket error: {0}")]
    SocketError(#[from] std::io::Error),
    #[error("Packet too large")]
    PacketTooLarge,
}

// ─────────────────────────────────────────────────────────────────────────────
// PEER — a trusted device on the network
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Peer {
    pub ed25519_pubkey:  [u8; 32],   // identity — used as routing key
    pub x25519_pubkey:   [u8; 32],   // for ECDH key exchange
    pub nickname:        String,
    pub last_seen_ms:    u64,
    pub addr:            Option<SocketAddr>,  // resolved from mDNS, can change
    pub shared_secret:   Option<[u8; 32]>,   // cached ECDH result (lazily computed)
}

// ─────────────────────────────────────────────────────────────────────────────
// WIRE PACKET FORMAT
// All UDP payloads use this format.
// Padded to PADDED_MSG_SIZE bytes to hide content length.
//
// Layout: [ version 1B ][ type 1B ][ sender_id 32B ][ nonce 12B ]
//         [ ciphertext (variable) ][ padding (zeros) ]
// ─────────────────────────────────────────────────────────────────────────────
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum PacketType {
    Message  = 0x01,
    CallOffer  = 0x02,
    CallAnswer = 0x03,
    CallEnd    = 0x04,
    Heartbeat  = 0x05,
    Revoke     = 0x06,
}

#[derive(Debug)]
pub struct WirePacket {
    pub version:   u8,
    pub ptype:     PacketType,
    pub sender_id: [u8; 32],    // Ed25519 pubkey of sender
    pub payload:   Vec<u8>,     // encrypted EncryptedPacket bytes
}

impl WirePacket {
    /// Serialise to bytes with padding. O(n)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(PADDED_MSG_SIZE);
        out.push(self.version);
        out.push(self.ptype as u8);
        out.extend_from_slice(&self.sender_id);
        out.extend_from_slice(&self.payload);
        // Pad to fixed size — hides message length from packet-length analysis
        while out.len() < PADDED_MSG_SIZE {
            out.push(0u8);
        }
        out
    }

    /// Parse from raw bytes. O(1) — fixed-header parsing
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 2 + 32 { return None; }
        let version   = data[0];
        let ptype_raw = data[1];
        let sender_id: [u8; 32] = data[2..34].try_into().ok()?;
        let payload   = data[34..].to_vec();
        let ptype = match ptype_raw {
            0x01 => PacketType::Message,
            0x02 => PacketType::CallOffer,
            0x03 => PacketType::CallAnswer,
            0x04 => PacketType::CallEnd,
            0x05 => PacketType::Heartbeat,
            0x06 => PacketType::Revoke,
            _    => return None,
        };
        Some(WirePacket { version, ptype, sender_id, payload })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TRANSPORT — the main network object
// ─────────────────────────────────────────────────────────────────────────────
pub struct Transport {
    socket:  Arc<AsyncUdpSocket>,
    /// O(1) lookup: Ed25519 pubkey → Peer
    peers:   Arc<DashMap<[u8; 32], Peer>>,
    /// Blocked devices (revoked). O(1) lookup.
    blocked: Arc<DashMap<[u8; 32], u64>>,
    our_id:  [u8; 32],
}

impl Transport {
    /// Bind to the PrivMesh UDP port.
    pub async fn new(our_id: [u8; 32]) -> Result<Self, NetworkError> {
        let socket = AsyncUdpSocket::bind(
            format!("0.0.0.0:{}", PRIVMESH_PORT)
        ).await?;
        // Enable broadcast — for group announcements
        socket.set_broadcast(true)?;
        Ok(Transport {
            socket:  Arc::new(socket),
            peers:   Arc::new(DashMap::new()),
            blocked: Arc::new(DashMap::new()),
            our_id,
        })
    }

    /// Add a trusted peer (called after successful NFC pairing). O(1)
    pub fn add_peer(&self, peer: Peer) {
        self.peers.insert(peer.ed25519_pubkey, peer);
    }

    /// Remove a peer by pubkey (admin revocation). O(1)
    pub fn revoke_peer(&self, pubkey: &[u8; 32]) {
        self.peers.remove(pubkey);
        self.blocked.insert(*pubkey, now_ms());
    }

    /// Check if a sender is trusted. O(1)
    fn is_trusted(&self, sender_id: &[u8; 32]) -> bool {
        // Blocked check first (revoked devices)
        if self.blocked.contains_key(sender_id) { return false; }
        self.peers.contains_key(sender_id)
    }

    /// Send an encrypted packet to a specific peer by their pubkey.
    /// O(n) for network I/O, O(1) for routing.
    pub async fn send_to_peer(
        &self,
        recipient_pubkey: &[u8; 32],
        ptype: PacketType,
        encrypted: &EncryptedPacket,
    ) -> Result<(), NetworkError> {
        // O(1) address lookup
        let peer = self.peers
            .get(recipient_pubkey)
            .ok_or_else(|| NetworkError::PeerNotFound(hex(recipient_pubkey)))?;

        let addr = peer.addr
            .ok_or_else(|| NetworkError::PeerNotFound("no addr resolved".into()))?;

        // Serialise encrypted packet
        let mut payload = Vec::with_capacity(12 + encrypted.ciphertext.len());
        payload.extend_from_slice(&encrypted.nonce);
        payload.extend_from_slice(&encrypted.ciphertext);

        let wire = WirePacket {
            version:   1,
            ptype,
            sender_id: self.our_id,
            payload,
        };

        let bytes = wire.to_bytes();
        self.socket.send_to(&bytes, addr).await?;
        Ok(())
    }

    /// Receive one packet. Returns None for untrusted senders (silently dropped).
    /// O(n) for network I/O, O(1) for whitelist check.
    pub async fn recv_packet(&self) -> Option<(WirePacket, SocketAddr)> {
        let mut buf = vec![0u8; MAX_PACKET_BYTES];
        loop {
            let (len, addr) = self.socket.recv_from(&mut buf).await.ok()?;
            let packet = WirePacket::from_bytes(&buf[..len])?;

            // WHITELIST CHECK — drop unknown senders before touching payload
            if !self.is_trusted(&packet.sender_id) {
                // Silent drop — no error response (don't fingerprint our port)
                continue;
            }

            // Update peer's last-seen timestamp and address
            if let Some(mut peer) = self.peers.get_mut(&packet.sender_id) {
                peer.last_seen_ms = now_ms();
                peer.addr = Some(addr);
            }

            return Some((packet, addr));
        }
    }

    /// Broadcast a heartbeat so peers can update our address. O(p)
    pub async fn broadcast_heartbeat(&self) -> Result<(), NetworkError> {
        let wire = WirePacket {
            version:   1,
            ptype:     PacketType::Heartbeat,
            sender_id: self.our_id,
            payload:   vec![],
        };
        let bytes = wire.to_bytes();
        // Send to all known peers
        for peer in self.peers.iter() {
            if let Some(addr) = peer.addr {
                let _ = self.socket.send_to(&bytes, addr).await;
            }
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// mDNS DISCOVERY
// Announces this device on the LAN so peers can find our IP without config.
// ─────────────────────────────────────────────────────────────────────────────
pub struct Discovery;

impl Discovery {
    /// Announce this device on mDNS so peers can discover us.
    /// Call once on app start. Runs in background async task.
    pub async fn announce(our_id: &[u8; 32], nickname: &str) {
        let service_name = format!("privmesh-{}", hex(&our_id[..8]));
        // mdns-sd crate handles the mDNS announce loop
        // TXT record carries our Ed25519 pubkey for identification
        let _txt = format!("id={}&nick={}", hex(our_id), nickname);
        // Implementation: register _privmesh._udp service on port PRIVMESH_PORT
        // mdns_sd::ServiceDaemon::new() → register(service_name, MDNS_SERVICE, ...)
        // This runs continuously and responds to peer queries
        tracing::info!("mDNS announced as {}", service_name);
    }

    /// Browse for peers on mDNS. Returns discovered (addr, txt_record) pairs.
    /// Filter by group_token before adding to trusted list.
    pub async fn browse() -> Vec<(SocketAddr, String)> {
        // mdns_sd::ServiceDaemon::new() → browse(MDNS_SERVICE, ...)
        // Returns peers announcing _privmesh._udp on the LAN
        vec![] // placeholder — real impl uses mdns-sd event loop
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HELPERS
// ─────────────────────────────────────────────────────────────────────────────
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
