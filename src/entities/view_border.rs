//! Drawing-view border entity (`AcDbViewBorder`, DXF class "DRAWINGVIEW").
//!
//! The border of a Model-Documentation drawing view. Its paper-space rectangle,
//! view scale and the reference to the view's *active* viewport (the entity
//! carrying the real camera) are decoded; the rest of the record is **not**
//! re-encoded natively — the original DWG record bytes are preserved in
//! [`raw_dwg_data`](ViewBorder::raw_dwg_data) and re-emitted verbatim on
//! write-back, exactly like [`Light`](super::light::Light) and
//! [`UnknownEntity`](super::unknown_entity::UnknownEntity).
//!
//! The border itself is a non-plotting aid — it is not drawn — but its
//! rectangle gives each view's true paper placement, and its viewport link is
//! the last hop of the section-mark viewing-direction chain.

use super::{Entity, EntityCommon};
use crate::types::{
    BoundingBox3D, Color, DxfVersion, Handle, LineWeight, Transform, Transparency, Vector3,
};

/// A Model-Documentation drawing-view border (`AcDbViewBorder`).
///
/// All coordinates are layout paper-space. Verified: `max − min` equals the
/// view's template-viewport size, `center` is exactly the rectangle midpoint
/// (stored redundantly in the record), and `scale` equals the template/active
/// viewport height ratio.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ViewBorder {
    /// Common entity data (handle, layer, color, …).
    pub common: EntityCommon,
    /// Border rectangle minimum corner.
    pub min: [f64; 2],
    /// Border rectangle maximum corner.
    pub max: [f64; 2],
    /// View centre point (the rectangle midpoint).
    pub center: [f64; 2],
    /// View scale denominator (e.g. `10` for a 1:10 view).
    pub scale: f64,
    /// The view's *active* viewport entity (the border's first object-specific
    /// handle reference) — carries the real camera (`view_direction`, twist).
    pub active_viewport: Handle,
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

impl ViewBorder {
    /// Create an empty view border.
    pub fn new() -> Self {
        ViewBorder {
            common: EntityCommon::new(),
            min: [0.0; 2],
            max: [0.0; 2],
            center: [0.0; 2],
            scale: 1.0,
            active_viewport: Handle::NULL,
            dwg_type_code: 0,
            dwg_handle_bits: 0,
            raw_dwg_data: None,
            dwg_source_version: None,
        }
    }
}

impl Default for ViewBorder {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity for ViewBorder {
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
        BoundingBox3D {
            min: Vector3::new(self.min[0], self.min[1], 0.0),
            max: Vector3::new(self.max[0], self.max[1], 0.0),
        }
    }
    fn translate(&mut self, _offset: Vector3) {
        // Anchored to its drawing view; the preserved raw record is re-emitted
        // verbatim, so a display-only move would silently revert on save.
    }
    fn entity_type(&self) -> &'static str {
        "DRAWINGVIEW"
    }
    fn apply_transform(&mut self, _transform: &Transform) {
        // See `translate`.
    }
}
