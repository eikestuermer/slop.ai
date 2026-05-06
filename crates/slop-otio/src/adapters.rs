//! Pro-NLE export adapters.
//!
//! Each adapter takes a `slop_core::Timeline` and writes the appropriate XML
//! file to disk. The rough-cut subset (cuts, trims, markers, captions,
//! linear speed) survives all targets; effect stacks do not. See
//! `docs/export-fidelity.md` for the full promise.
//!
//! These are pragmatic V1 implementations: hand-written XML, no external
//! dependencies. They produce documents that:
//!
//! - the target NLE imports without errors,
//! - the V1 feature subset round-trips cleanly,
//! - more advanced features are explicitly omitted rather than approximated.

pub mod fcp7;
pub mod fcpxml;
pub mod kdenlive;

pub use fcp7::write_fcp7_xml;
pub use fcpxml::write_resolve_fcpxml;
pub use kdenlive::write_kdenlive_xml;
