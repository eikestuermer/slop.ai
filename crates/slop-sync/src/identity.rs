//! ed25519-based identity. No central account server.
//!
//! Each Slop AI desktop instance generates an ed25519 keypair on first
//! launch and stores it in the user's local config directory. The
//! corresponding public key is the device's stable identifier across all
//! sync sessions, similar to Tailscale's WireGuard-key model.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors.
#[derive(Debug, Error)]
pub enum IdentityError {
    /// Hex decode failure.
    #[error("hex: {0}")]
    Hex(#[from] hex::FromHexError),
    /// Key load failure.
    #[error("key: {0}")]
    Key(String),
    /// Signature verification failure.
    #[error("signature mismatch")]
    SignatureMismatch,
    /// I/O.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Local identity (private + public key).
#[derive(Debug)]
pub struct Identity {
    signing: SigningKey,
}

impl Identity {
    /// Generate a fresh identity.
    pub fn generate() -> Self {
        Self {
            signing: SigningKey::generate(&mut OsRng),
        }
    }

    /// Load from raw 32-byte secret.
    pub fn from_secret_bytes(bytes: [u8; 32]) -> Self {
        Self {
            signing: SigningKey::from_bytes(&bytes),
        }
    }

    /// Public key bytes.
    pub fn pubkey_bytes(&self) -> [u8; 32] {
        self.signing.verifying_key().to_bytes()
    }

    /// Public key hex.
    pub fn pubkey_hex(&self) -> String {
        hex::encode(self.pubkey_bytes())
    }

    /// Sign a message.
    pub fn sign(&self, msg: &[u8]) -> [u8; 64] {
        self.signing.sign(msg).to_bytes()
    }
}

/// Verify a signature against a hex-encoded public key.
pub fn verify(pubkey_hex: &str, msg: &[u8], signature: &[u8; 64]) -> Result<(), IdentityError> {
    let bytes = hex::decode(pubkey_hex)?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| IdentityError::Key("32 bytes required".into()))?;
    let vk = VerifyingKey::from_bytes(&arr).map_err(|e| IdentityError::Key(e.to_string()))?;
    let sig = Signature::from_bytes(signature);
    vk.verify(msg, &sig)
        .map_err(|_| IdentityError::SignatureMismatch)
}

/// Persistable identity blob. Plain bincode would also work; we use a
/// stable JSON shape for portability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityFile {
    /// Hex secret.
    pub secret_hex: String,
    /// Hex public key (cached for inspection).
    pub pubkey_hex: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_verify_round_trip() {
        let id = Identity::generate();
        let msg = b"hello slop";
        let sig = id.sign(msg);
        verify(&id.pubkey_hex(), msg, &sig).unwrap();
    }

    #[test]
    fn verify_rejects_bad_signature() {
        let id = Identity::generate();
        let mut sig = id.sign(b"a");
        sig[0] ^= 0x01;
        assert!(matches!(
            verify(&id.pubkey_hex(), b"a", &sig),
            Err(IdentityError::SignatureMismatch)
        ));
    }

    #[test]
    fn from_secret_bytes_is_deterministic() {
        let secret = [42u8; 32];
        let a = Identity::from_secret_bytes(secret);
        let b = Identity::from_secret_bytes(secret);
        assert_eq!(a.pubkey_hex(), b.pubkey_hex());
        let sig_a = a.sign(b"test");
        let sig_b = b.sign(b"test");
        assert_eq!(sig_a.to_vec(), sig_b.to_vec());
    }

    #[test]
    fn pubkey_hex_is_64_chars() {
        let id = Identity::generate();
        let h = id.pubkey_hex();
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn verify_rejects_wrong_message() {
        let id = Identity::generate();
        let sig = id.sign(b"original");
        assert!(matches!(
            verify(&id.pubkey_hex(), b"different", &sig),
            Err(IdentityError::SignatureMismatch)
        ));
    }
}
