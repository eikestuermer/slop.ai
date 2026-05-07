//! Sync protocol wrapper around Automerge's binary sync messages.

use crate::doc::{TimelineDoc, TimelineDocError};
use automerge::sync::{Message, State as SyncState, SyncDoc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One sync wire message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncMessage {
    /// First message both sides send. Carries the peer's public key.
    Hello {
        /// Hex-encoded ed25519 public key.
        pubkey_hex: String,
        /// 32-byte challenge nonce hex.
        nonce_hex: String,
    },
    /// Reply to Hello: signature over the challenge nonce.
    HelloAck {
        /// Hex-encoded signature.
        signature_hex: String,
    },
    /// Automerge sync message bytes (base64-free; we use binary
    /// frames in WebSocket).
    Sync {
        /// Raw automerge sync message.
        body_b64: String,
    },
    /// Final close.
    Bye,
}

/// Errors during a sync session.
#[derive(Debug, Error)]
pub enum SyncError {
    /// Automerge error.
    #[error("automerge: {0}")]
    Automerge(String),
    /// Wrapping doc error.
    #[error(transparent)]
    Doc(#[from] TimelineDocError),
    /// Base64 decode failure.
    #[error("base64: {0}")]
    Base64(String),
    /// Identity / signature mismatch.
    #[error("identity: {0}")]
    Identity(String),
}

/// One peer's view of a sync session.
pub struct SyncSession {
    state: SyncState,
}

impl SyncSession {
    /// Construct a fresh sync state.
    pub fn new() -> Self {
        Self {
            state: SyncState::new(),
        }
    }

    /// Generate a sync message to send to the peer (or `None` if there is
    /// nothing to send right now).
    pub fn generate_message(
        &mut self,
        doc: &mut TimelineDoc,
    ) -> Result<Option<Vec<u8>>, SyncError> {
        let inner = doc.inner_mut();
        let msg = inner
            .sync()
            .generate_sync_message(&mut self.state)
            .map(|m| m.encode());
        Ok(msg)
    }

    /// Receive a sync message from the peer.
    pub fn receive_message(
        &mut self,
        doc: &mut TimelineDoc,
        bytes: &[u8],
    ) -> Result<(), SyncError> {
        let msg = Message::decode(bytes).map_err(|e| SyncError::Automerge(e.to_string()))?;
        let inner = doc.inner_mut();
        inner
            .sync()
            .receive_sync_message(&mut self.state, msg)
            .map_err(|e| SyncError::Automerge(e.to_string()))?;
        Ok(())
    }
}

impl Default for SyncSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slop_core::{Op, OpKind, TrackKind};

    #[test]
    fn two_peers_converge_after_one_exchange() {
        let mut alice_doc = TimelineDoc::empty().unwrap();
        let mut bob_doc = TimelineDoc::empty().unwrap();
        alice_doc
            .apply_op(&Op::new(OpKind::AddTrack {
                track_id: "v1".into(),
                kind: TrackKind::Video,
            }))
            .unwrap();
        let mut alice_sync = SyncSession::new();
        let mut bob_sync = SyncSession::new();

        for _ in 0..4 {
            if let Some(msg) = alice_sync.generate_message(&mut alice_doc).unwrap() {
                bob_sync.receive_message(&mut bob_doc, &msg).unwrap();
            }
            if let Some(msg) = bob_sync.generate_message(&mut bob_doc).unwrap() {
                alice_sync.receive_message(&mut alice_doc, &msg).unwrap();
            }
        }
        let bob_tl = bob_doc.reconstruct().unwrap();
        assert_eq!(bob_tl.tracks.len(), 1);
        assert_eq!(bob_tl.tracks[0].track_id, "v1");
    }
}
