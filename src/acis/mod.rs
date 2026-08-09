//! Bringing an ACIS document into the geometry kernel, and putting it back.
//!
//! ```text
//! SatDocument ──lift──► kernel::brep::Body ──edit──► ──lower──► SatDocument
//! ```
//!
//! # Why the bridge is here and not in the kernel
//!
//! The kernel's whole claim is that it knows geometry and not file formats,
//! and a dependency on a codec would end that. There is a practical reason
//! too: this crate already depends on the codec, so a kernel that also did
//! would give two paths to the same crate. Pin them at different revisions —
//! which happens the first time one is bumped and the other is not — and
//! Cargo builds two copies whose types do not unify, and every re-export from
//! this crate's root stops matching what the bridge produces.
//!
//! This crate is the one that knows both. The bridge belongs to it.
//!
//! # Provenance is what makes a save non-destructive
//!
//! Every node lifted records the record index it came from. Lowering reads
//! that back: a node still [`Clean`](kernel::brep::Provenance::Clean) is
//! written as its original record, byte for byte, so attributes,
//! parameter-space curves and the analytic surface types the kernel has not
//! learned survive an edit that never touched them. Only nodes an edit
//! actually dirtied are rebuilt.
//!
//! Without that, opening a file and saving it rewrites every solid in it
//! through whatever subset of ACIS this kernel models — which is a quiet,
//! total loss for bodies the user never opened.

mod lift;
mod lower;

pub use lift::{lift, lift_body, Loss};
pub use lower::{lower, pending, Unwritable};
