//! Plugin signature verification (Sigstore / cosign).
//!
//! Sigstore is the SOTA for OSS package signing as of 2024+. We sign
//! plugin .wasm + manifest using `cosign sign-blob` and verify against the
//! transparency log on install.
//!
//! V3.0 ships verification only; signing happens on plugin authors'
//! machines via the standard `cosign` CLI.

use sha2::{Digest, Sha256};
use std::path::Path;
use thiserror::Error;

/// Errors during signature verification.
#[derive(Debug, Error)]
pub enum SignatureError {
    /// SHA-256 didn't match the manifest's `wasm_sha256` field.
    #[error("wasm_sha256 mismatch: manifest says {expected}, file is {actual}")]
    HashMismatch {
        /// Manifest-declared hash.
        expected: String,
        /// Computed hash.
        actual: String,
    },
    /// Sigstore detached-signature file missing.
    #[error("sigstore signature file missing at {0}")]
    SignatureMissing(String),
    /// Cosign verification failed (delegated to the user's `cosign` binary).
    #[error("cosign verification failed: {0}")]
    Cosign(String),
    /// I/O.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Verify a plugin .wasm file against its manifest's declared hash. Then
/// invoke `cosign verify-blob` against the detached signature if present.
///
/// Returns `Ok(())` only if both checks pass. The caller controls whether
/// to require Sigstore-signed plugins or accept hash-only.
pub fn verify_plugin(
    wasm_path: &Path,
    expected_sha256_hex: &str,
    sigstore_bundle_path: Option<&Path>,
    require_sigstore: bool,
) -> Result<(), SignatureError> {
    let actual = sha256_of(wasm_path)?;
    if actual != expected_sha256_hex {
        return Err(SignatureError::HashMismatch {
            expected: expected_sha256_hex.into(),
            actual,
        });
    }
    match (sigstore_bundle_path, require_sigstore) {
        (Some(bundle), _) if bundle.is_file() => {
            // Delegate to the system cosign binary. Calling it via a
            // separate process keeps Slop AI's binary footprint small.
            let status = std::process::Command::new("cosign")
                .args(["verify-blob"])
                .arg("--bundle")
                .arg(bundle)
                .arg(wasm_path)
                .status();
            match status {
                Ok(s) if s.success() => Ok(()),
                Ok(s) => Err(SignatureError::Cosign(format!(
                    "cosign exit {}",
                    s.code().unwrap_or(-1)
                ))),
                Err(e) => Err(SignatureError::Cosign(e.to_string())),
            }
        }
        (Some(bundle), _) => Err(SignatureError::SignatureMissing(
            bundle.display().to_string(),
        )),
        (None, true) => Err(SignatureError::SignatureMissing("not provided".into())),
        (None, false) => Ok(()),
    }
}

fn sha256_of(p: &Path) -> std::io::Result<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(p)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_hash_mismatch() {
        let dir =
            std::env::temp_dir().join(format!("slop-sig-test-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let wasm = dir.join("plugin.wasm");
        std::fs::write(&wasm, b"hello").unwrap();
        // The known SHA-256 of "hello" is 2cf24dba...; provide the wrong one.
        let r = verify_plugin(
            &wasm,
            "0000000000000000000000000000000000000000000000000000000000000000",
            None,
            false,
        );
        match r {
            Err(SignatureError::HashMismatch { expected, actual }) => {
                assert_eq!(
                    expected,
                    "0000000000000000000000000000000000000000000000000000000000000000"
                );
                assert_eq!(
                    actual,
                    "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
                );
            }
            other => panic!("expected HashMismatch, got {other:?}"),
        }
    }

    #[test]
    fn matching_hash_passes_when_sigstore_not_required() {
        let dir =
            std::env::temp_dir().join(format!("slop-sig-pass-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let wasm = dir.join("plugin.wasm");
        std::fs::write(&wasm, b"hello").unwrap();
        verify_plugin(
            &wasm,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
            None,
            false,
        )
        .unwrap();
    }

    #[test]
    fn require_sigstore_without_bundle_fails() {
        let dir = std::env::temp_dir().join(format!(
            "slop-sig-needsig-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let wasm = dir.join("plugin.wasm");
        std::fs::write(&wasm, b"hello").unwrap();
        let r = verify_plugin(
            &wasm,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
            None,
            true,
        );
        assert!(matches!(r, Err(SignatureError::SignatureMissing(_))));
    }
}
