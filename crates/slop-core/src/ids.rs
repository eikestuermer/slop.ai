//! Stable, prefixed identifier helpers.
//!
//! Every entity in a timeline has a string id with a short prefix to make
//! human inspection of `ops.jsonl` easier:
//!
//! - assets: `a_<short>`
//! - tracks: `t_<short>`
//! - clips:  `c_<short>`
//! - ops:    `op_<short>`

use uuid::Uuid;

/// Generate an asset id.
pub fn asset() -> String {
    format!("a_{}", short())
}

/// Generate a track id.
pub fn track() -> String {
    format!("t_{}", short())
}

/// Generate a clip id.
pub fn clip() -> String {
    format!("c_{}", short())
}

/// Generate an op id.
pub fn op() -> String {
    format!("op_{}", short())
}

fn short() -> String {
    Uuid::new_v4().simple().to_string()[..12].to_string()
}
