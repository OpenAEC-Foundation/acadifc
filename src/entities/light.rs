//! Light entity (point / spot / distant light source).
//!
//! The light is parsed for its display glyph — the source position and, for
//! spot/distant lights, the aim point. The full photometric body (attenuation,
//! shadows, web/IES data) is **not** re-encoded natively: the original DWG
//! record bytes are preserved in [`raw_dwg_data`](Light::raw_dwg_data) and
//! re-emitted verbatim on write-back, exactly like
//! [`Surface`](super::solid3d::Surface) and
//! [`UnknownEntity`](super::unknown_entity::UnknownEntity). This keeps the file
//! lossless while still surfacing the geometry a renderer needs to draw the
//! nonprint light glyph.

use super::{Entity, EntityCommon};
use crate::types::{
    BoundingBox3D, Color, DxfVersion, Handle, LineWeight, Transform, Transparency, Vector3,
};

/// A light source entity (`AcDbLight`).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Light {
    /// Common entity data (handle, layer, color, …).
    pub common: EntityCommon,
    /// Light name (DXF 1), e.g. `"Spotlight1"`.
    pub name: String,
    /// Light type (DXF 70): 1 = distant, 2 = point, 3 = spot.
    pub light_type: i32,
    /// Light source position (DXF 10).
    pub position: Vector3,
    /// Aim / target point (DXF 11) — meaningful for spot and distant lights.
    pub target: Vector3,
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

impl Light {
    /// Create a new point light at the origin.
    pub fn new() -> Self {
        Light {
            common: EntityCommon::new(),
            name: String::new(),
            light_type: 2,
            position: Vector3::ZERO,
            target: Vector3::ZERO,
            dwg_type_code: 0,
            dwg_handle_bits: 0,
            raw_dwg_data: None,
            dwg_source_version: None,
        }
    }

    /// True for a spot (cone) light.
    pub fn is_spot(&self) -> bool {
        self.light_type == 3
    }

    /// True for a distant (parallel-ray) light.
    pub fn is_distant(&self) -> bool {
        self.light_type == 1
    }
}

impl Default for Light {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity for Light {
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
        BoundingBox3D::from_point(self.position)
    }
    fn translate(&mut self, offset: Vector3) {
        self.position = Vector3::new(
            self.position.x + offset.x,
            self.position.y + offset.y,
            self.position.z + offset.z,
        );
        self.target = Vector3::new(
            self.target.x + offset.x,
            self.target.y + offset.y,
            self.target.z + offset.z,
        );
    }
    fn entity_type(&self) -> &'static str {
        "LIGHT"
    }
    fn apply_transform(&mut self, _transform: &Transform) {
        // Lights are glyph-only in this library; a full transform of the
        // photometric frame is not modelled. Position/target stay put so the
        // preserved raw record continues to round-trip unchanged.
    }
}
