//! IFC support for CAD drawings, and the single entry point consumers use to
//! reach the rest of the stack.
//!
//! # What lives where
//!
//! ```text
//! OpenCADStudio
//!       │
//!    acadifc          IFC packages, canonicalisation, conformance,
//!       │             and IFC ↔ DWG/DXF conversion
//!       ├──► cadcodec     DWG/DXF/ACIS read and write
//!       └──► cadkernel    2D curve algebra and B-rep solids
//! ```
//!
//! The codec is re-exported from this crate's root, so `acadifc::entities`,
//! `acadifc::io`, `acadifc::types` and everything else resolve exactly as
//! they did when the codec lived here. Callers do not need to know which
//! crate a type came from.
//!
//! The geometry kernel is re-exported as [`kernel`]. Conversion in either
//! direction is geometry work — `IfcAdvancedBrep` carries NURBS surfaces,
//! `IfcFacetedBrep` is polygonal, `IfcExtrudedAreaSolid` is a sweep, and
//! profiles are 2D curves — so the conversion code that lands here reaches
//! into the kernel the same way an editor does.

#![allow(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

/// The DWG/DXF/ACIS codec, re-exported so consumers reach the CAD side
/// through this crate rather than depending on it separately.
pub use cadcodec::*;

/// The geometry kernel, for conversion work that has to build or traverse
/// solids rather than just move records around.
pub use cadkernel as kernel;

pub mod ifccad;
