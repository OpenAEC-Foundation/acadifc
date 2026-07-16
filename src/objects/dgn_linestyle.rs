//! DGN LineStyle objects (`AcDbLS*`).
//!
//! Drawings converted from MicroStation DGN carry their custom line styles as
//! DGN line-style objects rather than standard DWG linetype patterns. The
//! standard `LTYPE` table entry for such a linetype is empty (0 dashes); the
//! real definition lives in these objects, linked through the linetype's
//! extension dictionary (`DGNLSDEF` entry).
//!
//! The object graph is:
//! ```text
//! LineType (empty)
//!   -> xdictionary -> DGNLSDEF -> AcDbLSDefinition (name -> root component)
//!        -> AcDbLSCompoundComponent   (children: point + stroke components)
//!             -> AcDbLSStrokePatternComponent  (dash pattern)
//!             -> AcDbLSPointComponent          (places symbols along a stroke)
//!                  -> AcDbLSSymbolComponent    (references an anonymous block)
//! ```
//!
//! # Decoding status
//! The object header (`description`, component `type`) and the component tree
//! (hard-pointer handle references) are decoded. The leaf placement / pattern
//! data-stream fields (stroke dash lengths, point offsets, symbol scale /
//! rotation / offset) are **not yet decoded** — the objects are still stored
//! verbatim as [`crate::objects::ObjectType::Unknown`] so the DWG round-trips
//! byte-for-byte; these typed structs are the read-side view for rendering.

use crate::types::Handle;

/// Kind of a DGN line-style component (data-stream `component_type` field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DgnLsComponentType {
    /// `AcDbLSSymbolComponent` — references an anonymous block.
    Symbol,
    /// `AcDbLSCompoundComponent` — combines child components.
    Compound,
    /// `AcDbLSStrokePatternComponent` (LineCode) — a dash / gap pattern.
    Stroke,
    /// `AcDbLSPointComponent` (LinePoint) — places symbols along a stroke.
    Point,
}

impl DgnLsComponentType {
    /// Map the data-stream `component_type` value (1..=4) to the kind.
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            1 => Some(Self::Symbol),
            2 => Some(Self::Compound),
            3 => Some(Self::Stroke),
            4 => Some(Self::Point),
            _ => None,
        }
    }
}

/// `AcDbLSDefinition` — links a linetype name to its root component.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DgnLsDefinition {
    /// This object's handle.
    pub handle: Handle,
    /// Line-style name — equals the standard linetype's name (e.g. `"DET"`).
    pub name: String,
    /// Root component of the style tree (usually a compound component).
    pub root_component: Handle,
}

/// A DGN line-style component node (compound / stroke / point / symbol).
///
/// `refs` holds the object's hard-pointer handle references in file order; the
/// meaning depends on [`component_type`](Self::component_type):
/// - **Compound**: child component handles (points and strokes).
/// - **Point**: the base stroke component, then symbol component handles.
/// - **Symbol**: the anonymous block (first ref), then a back-reference.
/// - **Stroke**: none (self-contained pattern data).
///
/// Consumers resolve each ref's own [`DgnLsComponent`] (or block record) to
/// classify it, so the exact per-type counts do not need decoding yet.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DgnLsComponent {
    /// This object's handle.
    pub handle: Handle,
    /// Component kind.
    pub component_type: DgnLsComponentType,
    /// Component description (resource name; may be empty).
    pub description: String,
    /// Hard-pointer handle references (see the struct docs for their meaning).
    pub refs: Vec<Handle>,
}

impl DgnLsComponent {
    /// For a symbol component, the referenced anonymous block (first ref).
    pub fn symbol_block(&self) -> Option<Handle> {
        if self.component_type == DgnLsComponentType::Symbol {
            self.refs.first().copied()
        } else {
            None
        }
    }
}
