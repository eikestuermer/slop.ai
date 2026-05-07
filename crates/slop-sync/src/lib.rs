//! # slop-sync
//!
//! CRDT-based real-time collaboration.
//!
//! ## Document model
//!
//! The Slop AI timeline is encoded as an [Automerge](https://automerge.org)
//! document. Every operation that the V1 reducer produces becomes an
//! Automerge transaction, so the existing op log remains the conceptual
//! source of truth — Automerge just extends it with concurrent editing
//! and merge guarantees.
//!
//! ## Wire protocol
//!
//! Two peers (typically a desktop client and a sync server) communicate
//! over a single WebSocket using the official Automerge sync protocol.
//! Each side maintains a `SyncState` and exchanges binary messages until
//! both heads agree.
//!
//! On top of the raw sync protocol we layer:
//!
//! - **identity**: each peer presents an ed25519 public key during the
//!   WebSocket handshake (signed nonce); identity travels with every op
//!   so audit logs are tamper-evident.
//! - **ACL**: per-project, the project's authoritative ACL document
//!   (itself an Automerge document) lists allowed keys with roles.
//! - **CAS asset references**: media files are referenced by SHA-256
//!   content addresses; sync exchanges metadata only, never the media
//!   bytes (those move through Iroh in V2.5+).
//!
//! ## Why Automerge
//!
//! - First-class Rust API; no FFI to JS.
//! - Mature sync protocol (the standard p2p model since 2024).
//! - Document-shaped, not tree-shaped; matches our nested timeline.
//! - Production users include Ink & Switch, Inkdrop, Pony.

#![deny(missing_docs)]

pub mod acl;
pub mod doc;
pub mod identity;
pub mod sync_protocol;

pub use acl::{Acl, AclError, Role};
pub use doc::{TimelineDoc, TimelineDocError};
pub use identity::{Identity, IdentityError};
pub use sync_protocol::{SyncMessage, SyncSession};
