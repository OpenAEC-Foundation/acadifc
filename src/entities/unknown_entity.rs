//! Unknown entity type for round-trip preservation.
//!
//! When the reader encounters an entity type that is not directly supported,
//! it captures the common entity properties (handle, layer, color, …) and
//! the raw record data so the entity can be written back losslessly.
//!
//! For DWG files, the entire merged-stream record is stored in
//! [`raw_dwg_data`](UnknownEntity::raw_dwg_data) together with the
//! original DWG type code.  The writer emits these bytes verbatim,
//! preserving the entity exactly as it was in the source file.
//!
//! For DXF files, the entity-specific group-code pairs are stored in
//! [`raw_dxf_codes`](UnknownEntity::raw_dxf_codes) so they can be
//! written back alongside the common entity data.

use crate::entities::{Entity, EntityCommon};
use crate::types::{BoundingBox3D, Color, Handle, LineWeight, Transform, Transparency, Vector3};

/// Decoded geometry of an `AcDbSectionSymbol` (DWG class 825, "SECTIONLINE").
///
/// The section "A-A" cut mark drawn on a Model-Documentation base view. The
/// full object is still preserved verbatim in
/// [`raw_dwg_data`](UnknownEntity::raw_dwg_data) for lossless write-back; this
/// is the minimal geometry the editor needs to *display* the mark (matching how
/// [`Light`](crate::entities::Light) keeps decoded position + raw bytes).
///
/// Both endpoints are 2-D points in the layout's paper space. `tick_*` is the
/// signed extension length past each end (sign = extension direction along the
/// cut line). `label` is the section identifier (e.g. `"A"`).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SectionSymbol {
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
}

/// Display-relevant fields of an `AcDbSectionViewStyle` (DWG class 825's style,
/// "ACDBSECTIONVIEWSTYLE"), the named style that controls how a section mark is
/// drawn. Only the fields the editor needs to render the mark faithfully are
/// kept; the full object is preserved verbatim for write-back.
///
/// Cross-validated against LibreDWG `dwg2.spec` and a real AutoCAD sample.
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

/// An entity whose type is not directly supported by the library.
///
/// Preserves the DXF/DWG type name and common entity properties.
/// When raw data is available (DWG `raw_dwg_data` or DXF
/// `raw_dxf_codes`), the entity is written back losslessly.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UnknownEntity {
    /// Common entity data (handle, layer, color, reactors, …).
    pub common: EntityCommon,
    /// The DXF type name as it appeared in the file (e.g. `"ACAD_PROXY_ENTITY"`).
    pub dxf_name: String,
    /// DWG object type code (from the binary record header).
    /// `0` if the entity did not come from a DWG file.
    pub dwg_type_code: i16,
    /// Raw DWG merged-stream record bytes.
    ///
    /// This is the exact payload between the ModularShort length prefix
    /// and the CRC-16 trailer.  When present the writer emits these
    /// bytes verbatim (with fresh framing) so the entity survives a
    /// roundtrip without any data loss.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub raw_dwg_data: Option<Vec<u8>>,
    /// Handle-stream bit count for R2010+ DWG records.
    ///
    /// Stored alongside `raw_dwg_data` because R2010+ records require
    /// a ModularChar(handle_bits) field in the framing header.
    pub dwg_handle_bits: i64,
    /// Raw DXF entity-specific group-code pairs.
    ///
    /// Each entry is `(group_code, value_string)`.  When present the
    /// DXF writer emits the common entity header followed by these
    /// pairs, reproducing the original entity content.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub raw_dxf_codes: Option<Vec<(i32, String)>>,
    /// DWG version `raw_dwg_data` was read from (drop on incompatible cross-version save).
    #[cfg_attr(feature = "serde", serde(skip))]
    pub dwg_source_version: Option<crate::types::DxfVersion>,
    /// Decoded `AcDbSectionSymbol` geometry (DWG class 825), when this unknown
    /// entity is a Model-Documentation section mark. `None` for every other
    /// unknown type. The raw bytes above still drive write-back; this only
    /// enables display.
    pub section_symbol: Option<SectionSymbol>,
}

impl UnknownEntity {
    /// Create a new unknown entity with the given DXF type name.
    pub fn new(dxf_name: impl Into<String>) -> Self {
        Self {
            common: EntityCommon::new(),
            dxf_name: dxf_name.into(),
            dwg_type_code: 0,
            raw_dwg_data: None,
            dwg_handle_bits: 0,
            raw_dxf_codes: None,
            dwg_source_version: None,
            section_symbol: None,
        }
    }
}

impl Entity for UnknownEntity {
    fn handle(&self) -> Handle { self.common.handle }
    fn set_handle(&mut self, handle: Handle) { self.common.handle = handle; }
    fn layer(&self) -> &str { &self.common.layer }
    fn set_layer(&mut self, layer: String) { self.common.layer = layer; }
    fn color(&self) -> Color { self.common.color }
    fn set_color(&mut self, color: Color) { self.common.color = color; }
    fn line_weight(&self) -> LineWeight { self.common.line_weight }
    fn set_line_weight(&mut self, weight: LineWeight) { self.common.line_weight = weight; }
    fn transparency(&self) -> Transparency { self.common.transparency }
    fn set_transparency(&mut self, transparency: Transparency) { self.common.transparency = transparency; }
    fn is_invisible(&self) -> bool { self.common.invisible }
    fn set_invisible(&mut self, invisible: bool) { self.common.invisible = invisible; }
    fn bounding_box(&self) -> BoundingBox3D { BoundingBox3D::from_point(Vector3::ZERO) }
    fn translate(&mut self, _offset: Vector3) { super::translate::translate_unknown(self, _offset); }
    fn entity_type(&self) -> &'static str { "UNKNOWN" }
    fn apply_transform(&mut self, _transform: &Transform) { super::transform::transform_unknown(self, _transform); }
}
