//! Annotative per-object context data (`AcDb*ObjectContextData`).
//!
//! An annotative entity stores one *representation per annotation scale* in a
//! leaf object hung off its extension dictionary, via the chain
//! `entity xdict → "AcDbContextDataManager" → "ACDB_ANNOTATIONSCALES" → "*An"`.
//! Each leaf is a concrete `ACDB_<TYPE>OBJECTCONTEXTDATA_CLASS` object sharing a
//! common base:
//!
//! - `AcDbObjectContextData`: `70` class_version (BS), `290` is_default (B).
//! - `AcDbAnnotScaleObjectContextData`: `340` handle → the `AcDbScale` this
//!   representation applies to (rides the object handle stream).
//!
//! followed by a type-specific placement payload ([`ObjectContextKind`]).
//!
//! Objects read from a file keep their verbatim record bytes in
//! [`ObjectContextData::source_raw`] so they re-emit byte-for-byte on a
//! same-version save (the existing round-trip guarantee is preserved exactly);
//! only objects synthesized in memory (`source_raw == None`) are encoded from
//! their fields.

use crate::types::{DxfVersion, Handle, Vector2, Vector3};

/// A per-scale annotative representation (`AcDb*ObjectContextData` leaf).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjectContextData {
    /// Object handle.
    pub handle: Handle,
    /// Owner handle — the `ACDB_ANNOTATIONSCALES` dictionary that lists this leaf.
    pub owner_handle: Handle,
    /// Persistent reactors (real leaves carry one back to the owning dictionary).
    pub reactors: Vec<Handle>,
    /// Extension dictionary handle, if any.
    pub xdictionary_handle: Option<Handle>,

    /// `AcDbObjectContextData` class_version (DXF 70). Real R2018 files write 4;
    /// LibreDWG's default is 3. Preserved on read.
    pub class_version: i16,
    /// `AcDbObjectContextData` is_default flag (DXF 290) — set on the native rep.
    pub is_default: bool,
    /// `AcDbAnnotScaleObjectContextData` scale handle (DXF 340) → an `AcDbScale`
    /// in `ACAD_SCALELIST`.
    pub scale: Handle,

    /// Type-specific placement payload.
    pub kind: ObjectContextKind,

    /// Verbatim source record bytes captured on read (the full merged
    /// data+handle stream between the length prefix and the CRC). Re-emitted
    /// byte-for-byte on same-version save so reading these objects never
    /// perturbs an existing file. `None` for in-memory-synthesized objects,
    /// which are encoded from the fields above.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub source_raw: Option<Vec<u8>>,
    /// Handle-stream bit count that accompanies [`source_raw`](Self::source_raw)
    /// (needed to reproduce the data/handle split on verbatim re-emit).
    pub source_handle_bits: i64,
    /// DWG version [`source_raw`](Self::source_raw) was read from. Verbatim
    /// re-emit is only valid within the same encoding family; on an incompatible
    /// cross-version save the writer drops the object (parity with `Unknown`).
    /// `None` for synthesized objects, which are always encoded from fields.
    pub source_version: Option<DxfVersion>,
}

/// The type-specific placement payload of an annotative context leaf. Field
/// layouts follow LibreDWG `dwg2.spec` (the binary-canonical order).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ObjectContextKind {
    /// `AcDbBlkRefObjectContextData` (`ACDB_BLKREFOBJECTCONTEXTDATA_CLASS`,
    /// class 533): per-scale placement of an annotative block INSERT.
    ///
    /// DWG order after the shared base: `BD rotation(50)`, `3BD insertion(10)`,
    /// `BD x_scale(41)`, `BD y_scale(42)`, `BD z_scale(43)`.
    BlkRef {
        /// Rotation, radians (DXF 50).
        rotation: f64,
        /// Insertion point (DXF 10/20/30).
        insertion: Vector3,
        /// Per-axis scale (DXF 41/42/43).
        scale_factor: Vector3,
    },

    /// `AcDbTextObjectContextData` (`ACDB_TEXTOBJECTCONTEXTDATA_CLASS`):
    /// per-scale placement of annotative single-line TEXT/ATTRIB/ATTDEF.
    ///
    /// DWG order after the shared base: `BS horizontal_mode(70)`,
    /// `BD rotation(50)`, `2RD ins_pt(10)`, `2RD alignment_pt(11)`.
    Text {
        /// Horizontal justification mode (DXF 70).
        horizontal_mode: i16,
        /// Rotation, radians (DXF 50).
        rotation: f64,
        /// Insertion point, 2D only (DXF 10/20).
        insertion: Vector2,
        /// Alignment point, 2D only (DXF 11/21).
        alignment: Vector2,
    },

    /// `AcDbMTextObjectContextData` (`ACDB_MTEXTOBJECTCONTEXTDATA_CLASS`):
    /// per-scale placement of annotative MTEXT.
    ///
    /// DWG order after the shared base: `BL attachment(70)`, then — note the
    /// binary stream stores `x_axis_dir` **before** `ins_pt` (LibreDWG flags the
    /// reversed order an "ODA bug"; DXF emits them the other way) — `3BD
    /// x_axis_dir`, `3BD ins_pt`, `BD rect_width(40)`, `BD rect_height(41)`,
    /// `BD extents_width(42)`, `BD extents_height(43)`, `BL column_type(71)`,
    /// and — only when `column_type != 0` — the column sub-fields.
    MText(MTextContext),

    /// A dimension per-scale context (`ACDB_<AL|ANG|DM|RA|RADIMLG|ORD>DIMOBJECTCONTEXTDATA_CLASS`).
    ///
    /// ⚠ Layout confidence: the shared `AcDbDimensionObjectContextData` base +
    /// the *aligned* subtype are LibreDWG "UNSTABLE" (partially verified); the
    /// other five subtypes are LibreDWG "DEBUGGING" (best-guess, not
    /// round-trip-verified) and ACadSharp models none of them. Encoded from the
    /// verbatim `dwg2.spec` layout; read objects always re-emit verbatim.
    Dim(DimContext),

    /// A context leaf whose type is recognised as annotation-context (so its
    /// scale handle is captured) but whose placement payload is not yet modeled.
    /// Always carries [`source_raw`](ObjectContextData::source_raw) and is
    /// re-emitted verbatim; it is never synthesized.
    Opaque,
}

/// MTEXT per-scale placement payload (`AcDbMTextObjectContextData`).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MTextContext {
    /// Attachment point (DXF 70) — `BL` (bit-long), not `BS`.
    pub attachment: i32,
    /// Text X-axis direction (DXF 11/21/31 in DXF; stored *first* in binary).
    pub x_axis_dir: Vector3,
    /// Insertion point (DXF 10/20/30 in DXF; stored *second* in binary).
    pub insertion: Vector3,
    /// Reference rectangle width (DXF 40).
    pub rect_width: f64,
    /// Reference rectangle height (DXF 41).
    pub rect_height: f64,
    /// Extents width (DXF 42).
    pub extents_width: f64,
    /// Extents height (DXF 43).
    pub extents_height: f64,
    /// Column type (DXF 71): 0 none, 1 static, 2 dynamic.
    pub column_type: i32,
    /// Column sub-fields, present only when `column_type != 0`.
    pub columns: Option<MTextColumns>,
}

/// Dimension per-scale context (`AcDbDimensionObjectContextData` base + a
/// subtype). DWG data-stream order (after the shared class_version/is_default):
/// `2RD def_pt(10)`, `B is_def_textloc(294)`, `BD text_rotation(140)`,
/// `B b293`, `B dimtofl(298)`, `B dimosxd(291)`, `B dimatfit(70)`,
/// `B dimtix(292)`, `B dimtmove(71)`, `RC override_code(280)`,
/// `B has_arrow2(295)`, `B flip_arrow2(296)`, `B flip_arrow1(297)`, then the
/// subtype point(s). Handle stream: `scale`(soft-owner) then `block`(hard-ptr).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DimContext {
    /// Text location (DXF 10/20; the Z=30 is a constant 0.0 in DXF).
    pub def_pt: Vector2,
    /// Text location is user-defined (DXF 294).
    pub is_def_textloc: bool,
    /// Dimension text rotation (DXF 140).
    pub text_rotation: f64,
    /// The dimension's block reference (hard pointer; DXF 2 in the DXF path).
    pub block: Handle,
    /// Reserved bit (DXF 293 in the DXF path).
    pub b293: bool,
    /// DIMTOFL override (DXF 298).
    pub dimtofl: bool,
    /// DIMOSXD override (DXF 291).
    pub dimosxd: bool,
    /// DIMATFIT override (DXF 70 second occurrence).
    pub dimatfit: bool,
    /// DIMTIX override (DXF 292).
    pub dimtix: bool,
    /// DIMTMOVE override (DXF 71).
    pub dimtmove: bool,
    /// Override code (DXF 280) — a raw byte.
    pub override_code: u8,
    /// Second-arrow present (DXF 295).
    pub has_arrow2: bool,
    /// Flip second arrow (DXF 296).
    pub flip_arrow2: bool,
    /// Flip first arrow (DXF 297).
    pub flip_arrow1: bool,
    /// Subtype-specific point(s).
    pub subtype: DimSubtype,
}

/// The dimension subtype, selecting the concrete context class and its extra
/// `3BD` point field(s) (all after the shared dimension base).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DimSubtype {
    /// `ACDB_ALDIMOBJECTCONTEXTDATA_CLASS`: `3BD dimline_pt(11)`.
    Aligned { dimline_pt: Vector3 },
    /// `ACDB_ANGDIMOBJECTCONTEXTDATA_CLASS`: `3BD arc_pt(11)`.
    Angular { arc_pt: Vector3 },
    /// `ACDB_DMDIMOBJECTCONTEXTDATA_CLASS`: `3BD first_arc_pt(11)`, `3BD def_pt(12)`.
    Diametric { first_arc_pt: Vector3, def_pt: Vector3 },
    /// `ACDB_RADIMOBJECTCONTEXTDATA_CLASS`: `3BD first_arc_pt(11)`.
    Radial { first_arc_pt: Vector3 },
    /// `ACDB_RADIMLGOBJECTCONTEXTDATA_CLASS`: `3BD ovr_center(12)`, `3BD jog_point(13)`.
    RadialLarge { ovr_center: Vector3, jog_point: Vector3 },
    /// `ACDB_ORDDIMOBJECTCONTEXTDATA_CLASS`: `3BD feature_location_pt(11)`, `3BD leader_endpt(12)`.
    Ordinate { feature_location_pt: Vector3, leader_endpt: Vector3 },
}

impl DimSubtype {
    /// The concrete DXF class name for this subtype.
    pub fn class_name(&self) -> &'static str {
        match self {
            DimSubtype::Aligned { .. } => "ACDB_ALDIMOBJECTCONTEXTDATA_CLASS",
            DimSubtype::Angular { .. } => "ACDB_ANGDIMOBJECTCONTEXTDATA_CLASS",
            DimSubtype::Diametric { .. } => "ACDB_DMDIMOBJECTCONTEXTDATA_CLASS",
            DimSubtype::Radial { .. } => "ACDB_RADIMOBJECTCONTEXTDATA_CLASS",
            DimSubtype::RadialLarge { .. } => "ACDB_RADIMLGOBJECTCONTEXTDATA_CLASS",
            DimSubtype::Ordinate { .. } => "ACDB_ORDDIMOBJECTCONTEXTDATA_CLASS",
        }
    }

    /// The dimension subclass marker for this subtype.
    pub fn subclass_marker(&self) -> &'static str {
        match self {
            DimSubtype::Aligned { .. } => "AcDbAlignedDimensionObjectContextData",
            DimSubtype::Angular { .. } => "AcDbAngularDimensionObjectContextData",
            DimSubtype::Diametric { .. } => "AcDbDiametricDimensionObjectContextData",
            DimSubtype::Radial { .. } => "AcDbRadialDimensionObjectContextData",
            DimSubtype::RadialLarge { .. } => "AcDbRadialDimensionLargeObjectContextData",
            DimSubtype::Ordinate { .. } => "AcDbOrdinateDimensionObjectContextData",
        }
    }
}

/// MTEXT column layout, present only when `column_type != 0`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MTextColumns {
    /// Number of column heights (DXF 72).
    pub num_heights: i32,
    /// Column width (DXF 44).
    pub width: f64,
    /// Gutter width (DXF 45).
    pub gutter: f64,
    /// Auto-height flag (DXF 73).
    pub auto_height: bool,
    /// Flow-reversed flag (DXF 74).
    pub flow_reversed: bool,
    /// Explicit per-column heights (DXF 46) — present when
    /// `!auto_height && column_type == 2`.
    pub heights: Vec<f64>,
}

impl ObjectContextData {
    /// The DXF class / record name for this leaf's kind.
    pub fn class_name(&self) -> &'static str {
        match &self.kind {
            ObjectContextKind::BlkRef { .. } => "ACDB_BLKREFOBJECTCONTEXTDATA_CLASS",
            ObjectContextKind::Text { .. } => "ACDB_TEXTOBJECTCONTEXTDATA_CLASS",
            ObjectContextKind::MText(_) => "ACDB_MTEXTOBJECTCONTEXTDATA_CLASS",
            ObjectContextKind::Dim(d) => d.subtype.class_name(),
            ObjectContextKind::Opaque => "ACDB_OBJECTCONTEXTDATA_CLASS",
        }
    }
}
