//! # slop-otio
//!
//! Write the V1 OTIO subset directly as JSON, matching the official
//! OpenTimelineIO schema:
//!
//! - `Timeline`
//! - `Stack`
//! - `Track`
//! - `Clip`
//! - `Gap`
//! - `ExternalReference`
//! - `Marker`
//! - `TimeRange` / `RationalTime`
//!
//! For V1 we deliberately do not link the C++ `OpenTimelineIO` library:
//!
//! - the FFI surface is large,
//! - none of its rich adapters (FCP7 XML, FCPX, AAF) are available without
//!   the full Python ecosystem,
//! - our V1 promise is the cuts/markers/captions subset, which fits
//!   comfortably in pure Rust.
//!
//! Adapters for Premiere, Resolve, and Kdenlive live in [`adapters`].

#![deny(missing_docs)]

pub mod adapters;
pub mod schema;
pub mod writer;

pub use adapters::{write_fcp7_xml, write_kdenlive_xml, write_resolve_fcpxml};
pub use writer::{timeline_to_otio_json, write_otio};
