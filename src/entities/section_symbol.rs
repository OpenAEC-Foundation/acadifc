//! Section symbol entity (`AcDbSectionSymbol`, DXF class "SECTIONLINE").
//!
//! The section "A-A" cut mark drawn on a Model-Documentation base view. The
//! cut-line endpoints, end ticks, identifier and the style / parent-view handle
//! references are decoded for display; the rest of the record (undocumented
//! header flags and per-end fields) is **not** re-encoded natively — the
//! original DWG record bytes are preserved in
//! [`raw_dwg_data`](SectionSymbol::raw_dwg_data) and re-emitted verbatim on
//! write-back, exactly like [`Light`](super::light::Light) and
//! [`UnknownEntity`](super::unknown_entity::UnknownEntity).

use super::{Entity, EntityCommon};
use crate::types::{
    BoundingBox3D, Color, DxfVersion, Handle, LineWeight, Transform, Transparency, Vector3,
};

/// A Model-Documentation section mark (`AcDbSectionSymbol`).
///
/// Both endpoints are 2-D points in the layout's paper space. `tick_*` is the
/// signed extension length past each end (along the B→A axis). `label` is the
/// section identifier (e.g. `"A"`).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SectionSymbol {
    /// Common entity data (handle, layer, color, …).
    pub common: EntityCommon,
    /// First cut-line endpoint (paper-space X, Y).
    pub end_a: [f64; 2],
    /// Second cut-line endpoint (paper-space X, Y).
    pub end_b: [f64; 2],
    /// Signed extension length past `end_a` along the cut line.
    pub tick_a: f64,
    /// Signed extension length past `end_b` along the cut line.
    pub tick_b: f64,
    /// Section identifier text (drawn at each end).
    pub label: String,
    /// The symbol's `AcDbSectionViewStyle` handle (first object-specific
    /// handle reference). `0` when unavailable.
    pub style_handle: u64,
    /// The parent view's `AcDbViewRep` handle (second object-specific handle
    /// reference) — the drawing view the cut line is sketched on. `0` when
    /// unavailable.
    pub view_rep_handle: u64,
    /// DWG object type code (round-trip).
    pub dwg_type_code: i16,
    /// Handle-stream bit count for R2010+ records (round-trip framing).
    pub dwg_handle_bits: i64,
    /// Raw DWG record bytes, re-emitted verbatim on write-back.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub raw_dwg_data: Option<Vec<u8>>,
    /// Source DWG version — dropped on an incompatible cross-version save.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub dwg_source_version: Option<DxfVersion>,
}

impl SectionSymbol {
    /// Create an empty section symbol.
    pub fn new() -> Self {
        SectionSymbol {
            common: EntityCommon::new(),
            end_a: [0.0; 2],
            end_b: [0.0; 2],
            tick_a: 0.0,
            tick_b: 0.0,
            label: String::new(),
            style_handle: 0,
            view_rep_handle: 0,
            dwg_type_code: 0,
            dwg_handle_bits: 0,
            raw_dwg_data: None,
            dwg_source_version: None,
        }
    }
}

impl Default for SectionSymbol {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity for SectionSymbol {
    fn handle(&self) -> Handle {
        self.common.handle
    }
    fn set_handle(&mut self, handle: Handle) {
        self.common.handle = handle;
    }
    fn layer(&self) -> &str {
        &self.common.layer
    }
    fn set_layer(&mut self, layer: String) {
        self.common.layer = layer;
    }
    fn color(&self) -> Color {
        self.common.color
    }
    fn set_color(&mut self, color: Color) {
        self.common.color = color;
    }
    fn line_weight(&self) -> LineWeight {
        self.common.line_weight
    }
    fn set_line_weight(&mut self, weight: LineWeight) {
        self.common.line_weight = weight;
    }
    fn transparency(&self) -> Transparency {
        self.common.transparency
    }
    fn set_transparency(&mut self, transparency: Transparency) {
        self.common.transparency = transparency;
    }
    fn is_invisible(&self) -> bool {
        self.common.invisible
    }
    fn set_invisible(&mut self, invisible: bool) {
        self.common.invisible = invisible;
    }
    fn bounding_box(&self) -> BoundingBox3D {
        // The default (all-zero) box: arrows, ticks and labels extend an
        // amount only the section-view style knows, so no tight box is
        // reported — renderers treat the degenerate box as unbounded.
        BoundingBox3D::from_point(Vector3::ZERO)
    }
    fn translate(&mut self, _offset: Vector3) {
        // Associative to its drawing view; the preserved raw record is
        // re-emitted verbatim, so a display-only move would silently revert
        // on save. Keep it anchored.
    }
    fn entity_type(&self) -> &'static str {
        "SECTIONLINE"
    }
    fn apply_transform(&mut self, _transform: &Transform) {
        // See `translate`.
    }
}

/// Display-relevant fields of an `AcDbSectionViewStyle` (DXF class
/// "ACDBSECTIONVIEWSTYLE"), the named style that controls how a section mark
/// is drawn. Only the fields the editor needs to render the mark faithfully
/// are kept; the full object is preserved verbatim for write-back.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SectionViewStyle {
    /// Whether direction arrowheads are drawn (style `flags` bit 0x02).
    pub show_arrows: bool,
    /// Whether the full cutting-plane line is drawn through the view (`flags`
    /// bit 0x08). Off = the familiar "broken" section line: only the end
    /// segments are drawn.
    pub show_plane_line: bool,
    /// Whether the end (and bend) line segments are drawn (`flags` bit 0x20).
    pub show_end_lines: bool,
    /// Arrowhead size (`arrow_symbol_size`).
    pub arrow_size: f64,
    /// How far the arrow extends past the cut line (`arrow_symbol_extension_length`).
    pub arrow_extension: f64,
    /// Section identifier ("A") text height (`identifier_height`).
    pub label_height: f64,
    /// Gap between the cut line and the identifier text (`identifier_offset`).
    pub label_offset: f64,
    /// Identifier placement enum (`identifier_position`), raw value.
    pub label_position: i32,
    /// Arrow placement enum (`arrow_position`), raw value.
    pub arrow_position: i32,
    /// End-segment length (`end_line_length`) — with the overshoot this equals
    /// the symbol's per-end tick.
    pub end_line_length: f64,
    /// Extension of the end segment beyond the arrow anchor (`end_line_overshoot`).
    pub end_line_overshoot: f64,
    /// Arrowhead block-record handles for the start / end of the section line
    /// (`arrow_start_symbol` / `arrow_end_symbol`). `0` (null) selects the
    /// built-in default arrow — the same ClosedFilled block dimensions and
    /// leaders default to.
    pub arrow_start_handle: u64,
    /// See [`arrow_start_handle`](Self::arrow_start_handle).
    pub arrow_end_handle: u64,
    /// True when both arrow symbol handles are null, i.e. the built-in default
    /// (solid/filled) arrowhead is used rather than a custom arrow block.
    pub arrow_is_default: bool,
}
