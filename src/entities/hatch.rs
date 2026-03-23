//! Hatch entity and boundary path types

use crate::entities::{Entity, EntityCommon};
use crate::types::{BoundingBox3D, Color, Handle, LineWeight, Matrix3, Transform, Transparency, Vector2, Vector3};

/// Hatch pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HatchPatternType {
    /// User-defined pattern
    UserDefined = 0,
    /// Predefined pattern
    Predefined = 1,
    /// Custom pattern
    Custom = 2,
}

/// Hatch style type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HatchStyleType {
    /// Hatch "odd parity" area (normal)
    Normal = 0,
    /// Hatch outermost area only
    Outer = 1,
    /// Hatch through entire area
    Ignore = 2,
}

/// Boundary path flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BoundaryPathFlags {
    bits: u32,
}

impl BoundaryPathFlags {
    pub const DEFAULT: Self = Self { bits: 0 };
    pub const EXTERNAL: Self = Self { bits: 1 };
    pub const POLYLINE: Self = Self { bits: 2 };
    pub const DERIVED: Self = Self { bits: 4 };
    pub const TEXTBOX: Self = Self { bits: 8 };
    pub const OUTERMOST: Self = Self { bits: 16 };
    pub const NOT_CLOSED: Self = Self { bits: 32 };
    pub const SELF_INTERSECTING: Self = Self { bits: 64 };
    pub const TEXT_ISLAND: Self = Self { bits: 128 };
    pub const DUPLICATE: Self = Self { bits: 256 };

    pub fn new() -> Self {
        Self::DEFAULT
    }
    
    /// Create from raw bits
    pub fn from_bits(bits: u32) -> Self {
        Self { bits }
    }
    
    /// Get raw bits
    pub fn bits(&self) -> u32 {
        self.bits
    }

    pub fn is_external(&self) -> bool {
        self.bits & 1 != 0
    }

    pub fn is_polyline(&self) -> bool {
        self.bits & 2 != 0
    }

    pub fn is_derived(&self) -> bool {
        self.bits & 4 != 0
    }
    
    pub fn is_outermost(&self) -> bool {
        self.bits & 16 != 0
    }
    
    pub fn is_not_closed(&self) -> bool {
        self.bits & 32 != 0
    }

    pub fn set_external(&mut self, value: bool) {
        if value {
            self.bits |= 1;
        } else {
            self.bits &= !1;
        }
    }

    pub fn set_polyline(&mut self, value: bool) {
        if value {
            self.bits |= 2;
        } else {
            self.bits &= !2;
        }
    }
    
    pub fn set_derived(&mut self, value: bool) {
        if value {
            self.bits |= 4;
        } else {
            self.bits &= !4;
        }
    }
}

impl Default for BoundaryPathFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// Edge type for boundary paths
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EdgeType {
    Polyline = 0,
    Line = 1,
    CircularArc = 2,
    EllipticArc = 3,
    Spline = 4,
}

/// Line edge in a boundary path
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LineEdge {
    /// Start point (in OCS)
    pub start: Vector2,
    /// End point (in OCS)
    pub end: Vector2,
}

/// Circular arc edge in a boundary path
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CircularArcEdge {
    /// Center point (in OCS)
    pub center: Vector2,
    /// Radius
    pub radius: f64,
    /// Start angle in radians
    pub start_angle: f64,
    /// End angle in radians
    pub end_angle: f64,
    /// Counter-clockwise flag
    pub counter_clockwise: bool,
}

/// Elliptic arc edge in a boundary path
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EllipticArcEdge {
    /// Center point (in OCS)
    pub center: Vector2,
    /// Endpoint of major axis relative to center (in OCS)
    pub major_axis_endpoint: Vector2,
    /// Ratio of minor axis to major axis
    pub minor_axis_ratio: f64,
    /// Start angle in radians
    pub start_angle: f64,
    /// End angle in radians
    pub end_angle: f64,
    /// Counter-clockwise flag
    pub counter_clockwise: bool,
}

/// Spline edge in a boundary path
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplineEdge {
    /// Degree of the spline
    pub degree: i32,
    /// Rational flag
    pub rational: bool,
    /// Periodic flag
    pub periodic: bool,
    /// Knot values
    pub knots: Vec<f64>,
    /// Control points (X, Y, weight)
    pub control_points: Vec<Vector3>,
    /// Fit points
    pub fit_points: Vec<Vector2>,
    /// Start tangent
    pub start_tangent: Vector2,
    /// End tangent
    pub end_tangent: Vector2,
}

/// Polyline edge in a boundary path
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PolylineEdge {
    /// Vertices (X, Y, bulge)
    pub vertices: Vec<Vector3>,
    /// Is closed flag
    pub is_closed: bool,
}

impl PolylineEdge {
    /// Create a new polyline edge
    pub fn new(vertices: Vec<Vector2>, is_closed: bool) -> Self {
        Self {
            vertices: vertices.into_iter().map(|v| Vector3::new(v.x, v.y, 0.0)).collect(),
            is_closed,
        }
    }

    /// Add a vertex with bulge
    pub fn add_vertex(&mut self, point: Vector2, bulge: f64) {
        self.vertices.push(Vector3::new(point.x, point.y, bulge));
    }

    /// Check if the polyline has any bulges
    pub fn has_bulge(&self) -> bool {
        self.vertices.iter().any(|v| v.z.abs() > 1e-10)
    }
}

/// Boundary path edge
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BoundaryEdge {
    Line(LineEdge),
    CircularArc(CircularArcEdge),
    EllipticArc(EllipticArcEdge),
    Spline(SplineEdge),
    Polyline(PolylineEdge),
}

/// Boundary path for a hatch
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BoundaryPath {
    /// Boundary path flags
    pub flags: BoundaryPathFlags,
    /// Edges that form the boundary
    pub edges: Vec<BoundaryEdge>,
    /// Handles of associated boundary objects (for associative hatches)
    pub boundary_handles: Vec<Handle>,
}

impl BoundaryPath {
    /// Create a new boundary path
    pub fn new() -> Self {
        Self {
            flags: BoundaryPathFlags::new(),
            edges: Vec::new(),
            boundary_handles: Vec::new(),
        }
    }
    
    /// Create a boundary path with flags
    pub fn with_flags(flags: BoundaryPathFlags) -> Self {
        Self {
            flags,
            edges: Vec::new(),
            boundary_handles: Vec::new(),
        }
    }

    /// Create an external boundary path
    pub fn external() -> Self {
        let mut path = Self::new();
        path.flags.set_external(true);
        path
    }

    /// Add an edge to the boundary
    pub fn add_edge(&mut self, edge: BoundaryEdge) {
        // Update polyline flag if needed
        if matches!(edge, BoundaryEdge::Polyline(_)) {
            self.flags.set_polyline(true);
        }
        self.edges.push(edge);
    }
    
    /// Add a boundary object handle
    pub fn add_boundary_handle(&mut self, handle: Handle) {
        self.boundary_handles.push(handle);
    }

    /// Check if this is a polyline boundary
    pub fn is_polyline(&self) -> bool {
        self.edges.len() == 1 && matches!(self.edges[0], BoundaryEdge::Polyline(_))
    }
}

impl Default for BoundaryPath {
    fn default() -> Self {
        Self::new()
    }
}

/// Hatch pattern line
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HatchPatternLine {
    /// Pattern line angle in radians
    pub angle: f64,
    /// Pattern line base point
    pub base_point: Vector2,
    /// Pattern line offset
    pub offset: Vector2,
    /// Dash lengths (positive = dash, negative = space)
    pub dash_lengths: Vec<f64>,
}

/// Hatch pattern
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HatchPattern {
    /// Pattern name
    pub name: String,
    /// Pattern description
    pub description: String,
    /// Pattern lines
    pub lines: Vec<HatchPatternLine>,
}

impl HatchPattern {
    /// Create a new pattern
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            lines: Vec::new(),
        }
    }

    /// Create a solid fill pattern
    pub fn solid() -> Self {
        Self::new("SOLID")
    }

    /// Add a pattern line
    pub fn add_line(&mut self, line: HatchPatternLine) {
        self.lines.push(line);
    }

    /// Update pattern with scale and rotation
    pub fn update(&mut self, _base_point: Vector2, angle: f64, scale: f64) {
        for line in &mut self.lines {
            line.angle += angle;
            line.offset = line.offset * scale;
            for dash in &mut line.dash_lengths {
                *dash *= scale;
            }
        }
    }
}

/// Gradient color with value
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GradientColorEntry {
    /// Gradient value (position 0.0 - 1.0)
    pub value: f64,
    /// Color at this position
    pub color: Color,
}

/// Gradient color pattern
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HatchGradientPattern {
    /// Gradient is enabled
    pub enabled: bool,
    /// Reserved value (DXF 451)
    pub reserved: i32,
    /// Gradient angle in radians
    pub angle: f64,
    /// Gradient shift (0.0 - 1.0)
    pub shift: f64,
    /// Single color gradient flag
    pub is_single_color: bool,
    /// Color tint (for single color gradients)
    pub color_tint: f64,
    /// Gradient colors with their values
    pub colors: Vec<GradientColorEntry>,
    /// Gradient name (e.g., "LINEAR", "CYLINDER", etc.)
    pub name: String,
}

impl HatchGradientPattern {
    pub fn new() -> Self {
        Self {
            enabled: false,
            reserved: 0,
            angle: 0.0,
            shift: 0.0,
            is_single_color: false,
            color_tint: 0.0,
            colors: Vec::new(),
            name: String::new(),
        }
    }
    
    /// Check if gradient is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// Add a color to the gradient
    pub fn add_color(&mut self, value: f64, color: Color) {
        self.colors.push(GradientColorEntry { value, color });
    }
}

impl Default for HatchGradientPattern {
    fn default() -> Self {
        Self::new()
    }
}

/// Hatch entity
///
/// Represents a filled or patterned area defined by boundary paths.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Hatch {
    pub common: EntityCommon,
    /// Elevation of the hatch
    pub elevation: f64,
    /// Normal vector (extrusion direction)
    pub normal: Vector3,
    /// Hatch pattern
    pub pattern: HatchPattern,
    /// Is solid fill
    pub is_solid: bool,
    /// Is associative (linked to boundary objects)
    pub is_associative: bool,
    /// Hatch pattern type
    pub pattern_type: HatchPatternType,
    /// Hatch pattern angle in radians
    pub pattern_angle: f64,
    /// Hatch pattern scale
    pub pattern_scale: f64,
    /// Is pattern double (for pattern fill only)
    pub is_double: bool,
    /// Hatch style
    pub style: HatchStyleType,
    /// Boundary paths (loops)
    pub paths: Vec<BoundaryPath>,
    /// Seed points (in OCS)
    pub seed_points: Vec<Vector2>,
    /// Pixel size for intersection operations
    pub pixel_size: f64,
    /// Gradient color pattern
    pub gradient_color: HatchGradientPattern,
}

impl Hatch {
    /// Create a new hatch entity
    pub fn new() -> Self {
        Self {
            common: EntityCommon::default(),
            elevation: 0.0,
            normal: Vector3::new(0.0, 0.0, 1.0),
            pattern: HatchPattern::solid(),
            is_solid: true,
            is_associative: false,
            pattern_type: HatchPatternType::Predefined,
            pattern_angle: 0.0,
            pattern_scale: 1.0,
            is_double: false,
            style: HatchStyleType::Normal,
            paths: Vec::new(),
            seed_points: Vec::new(),
            pixel_size: 0.0,
            gradient_color: HatchGradientPattern::new(),
        }
    }

    /// Create a solid fill hatch
    pub fn solid() -> Self {
        let mut hatch = Self::new();
        hatch.is_solid = true;
        hatch.pattern = HatchPattern::solid();
        hatch
    }

    /// Create a pattern fill hatch
    pub fn with_pattern(pattern: HatchPattern) -> Self {
        let mut hatch = Self::new();
        hatch.is_solid = false;
        hatch.pattern = pattern;
        hatch
    }

    /// Builder: Set the pattern angle
    pub fn with_pattern_angle(mut self, angle: f64) -> Self {
        self.pattern_angle = angle;
        self.pattern.update(Vector2::new(0.0, 0.0), angle, self.pattern_scale);
        self
    }

    /// Builder: Set the pattern scale
    pub fn with_pattern_scale(mut self, scale: f64) -> Self {
        self.pattern_scale = scale;
        self.pattern.update(Vector2::new(0.0, 0.0), self.pattern_angle, scale);
        self
    }

    /// Builder: Set the normal vector
    pub fn with_normal(mut self, normal: Vector3) -> Self {
        self.normal = normal;
        self
    }

    /// Builder: Set the elevation
    pub fn with_elevation(mut self, elevation: f64) -> Self {
        self.elevation = elevation;
        self
    }

    /// Add a boundary path
    pub fn add_path(&mut self, path: BoundaryPath) {
        self.paths.push(path);
    }

    /// Add a seed point
    pub fn add_seed_point(&mut self, point: Vector2) {
        self.seed_points.push(point);
    }

    /// Set the pattern angle (updates pattern)
    pub fn set_pattern_angle(&mut self, angle: f64) {
        self.pattern_angle = angle;
        self.pattern.update(Vector2::new(0.0, 0.0), angle, self.pattern_scale);
    }

    /// Set the pattern scale (updates pattern)
    pub fn set_pattern_scale(&mut self, scale: f64) {
        self.pattern_scale = scale;
        self.pattern.update(Vector2::new(0.0, 0.0), self.pattern_angle, scale);
    }

    /// Get the number of boundary paths
    pub fn path_count(&self) -> usize {
        self.paths.len()
    }

    /// Check if the hatch has any paths
    pub fn has_paths(&self) -> bool {
        !self.paths.is_empty()
    }
}

impl Default for Hatch {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity for Hatch {
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
        // Calculate bounding box from all boundary paths
        let mut all_points = Vec::new();

        for path in &self.paths {
            for edge in &path.edges {
                match edge {
                    BoundaryEdge::Line(line) => {
                        all_points.push(Vector3::new(line.start.x, line.start.y, self.elevation));
                        all_points.push(Vector3::new(line.end.x, line.end.y, self.elevation));
                    }
                    BoundaryEdge::CircularArc(arc) => {
                        // Add center and radius-based bounds
                        all_points.push(Vector3::new(arc.center.x - arc.radius, arc.center.y - arc.radius, self.elevation));
                        all_points.push(Vector3::new(arc.center.x + arc.radius, arc.center.y + arc.radius, self.elevation));
                    }
                    BoundaryEdge::EllipticArc(ellipse) => {
                        // Add center and major axis-based bounds
                        let major_len = ellipse.major_axis_endpoint.length();
                        all_points.push(Vector3::new(ellipse.center.x - major_len, ellipse.center.y - major_len, self.elevation));
                        all_points.push(Vector3::new(ellipse.center.x + major_len, ellipse.center.y + major_len, self.elevation));
                    }
                    BoundaryEdge::Spline(spline) => {
                        for cp in &spline.control_points {
                            all_points.push(Vector3::new(cp.x, cp.y, self.elevation));
                        }
                    }
                    BoundaryEdge::Polyline(poly) => {
                        for v in &poly.vertices {
                            all_points.push(Vector3::new(v.x, v.y, self.elevation));
                        }
                    }
                }
            }
        }

        if all_points.is_empty() {
            BoundingBox3D::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 0.0))
        } else {
            BoundingBox3D::from_points(&all_points).unwrap_or_else(|| BoundingBox3D::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 0.0)))
        }
    }

    fn translate(&mut self, offset: Vector3) {
        let ocs_to_wcs = Matrix3::arbitrary_axis(self.normal);
        let wcs_to_ocs = ocs_to_wcs.transpose();
        let offset_ocs = wcs_to_ocs * offset;

        self.elevation += offset_ocs.z;

        for path in &mut self.paths {
            for edge in &mut path.edges {
                match edge {
                    BoundaryEdge::Line(line) => {
                        line.start.x += offset_ocs.x;
                        line.start.y += offset_ocs.y;
                        line.end.x += offset_ocs.x;
                        line.end.y += offset_ocs.y;
                    }
                    BoundaryEdge::CircularArc(arc) => {
                        arc.center.x += offset_ocs.x;
                        arc.center.y += offset_ocs.y;
                    }
                    BoundaryEdge::EllipticArc(ellipse) => {
                        ellipse.center.x += offset_ocs.x;
                        ellipse.center.y += offset_ocs.y;
                    }
                    BoundaryEdge::Spline(spline) => {
                        for cp in &mut spline.control_points {
                            cp.x += offset_ocs.x;
                            cp.y += offset_ocs.y;
                        }
                        for fp in &mut spline.fit_points {
                            fp.x += offset_ocs.x;
                            fp.y += offset_ocs.y;
                        }
                    }
                    BoundaryEdge::Polyline(poly) => {
                        for v in &mut poly.vertices {
                            v.x += offset_ocs.x;
                            v.y += offset_ocs.y;
                        }
                    }
                }
            }
        }

        for seed in &mut self.seed_points {
            seed.x += offset_ocs.x;
            seed.y += offset_ocs.y;
        }
    }

    fn entity_type(&self) -> &'static str {
        "HATCH"
    }
    
    fn apply_transform(&mut self, transform: &Transform) {
        let old_normal = self.normal;
        let old_elevation = self.elevation;

        let new_normal = transform.apply_rotation(old_normal).normalize();
        
        let old_ocs_to_wcs = Matrix3::arbitrary_axis(old_normal);
        let new_wcs_to_ocs = Matrix3::arbitrary_axis(new_normal).transpose();
        
        let origin_wcs = old_ocs_to_wcs * Vector3::new(0.0, 0.0, old_elevation);
        let new_origin_wcs = transform.apply(origin_wcs);
        let new_origin_ocs = new_wcs_to_ocs * new_origin_wcs;
        let new_elevation = new_origin_ocs.z;

        let ocs_x_wcs = old_ocs_to_wcs * Vector3::UNIT_X;
        let ocs_y_wcs = old_ocs_to_wcs * Vector3::UNIT_Y;
        
        let trans_ocs_x_wcs = transform.apply_rotation(ocs_x_wcs);
        let trans_ocs_y_wcs = transform.apply_rotation(ocs_y_wcs);
        
        let scale_x = trans_ocs_x_wcs.length();
        let scale_y = trans_ocs_y_wcs.length();
        
        let is_uniform = (scale_x - scale_y).abs() < 1e-10 && trans_ocs_x_wcs.dot(&trans_ocs_y_wcs).abs() < 1e-10;

        let x_in_new_ocs = new_wcs_to_ocs * trans_ocs_x_wcs;
        let y_in_new_ocs = new_wcs_to_ocs * trans_ocs_y_wcs;
        let is_flipped = (x_in_new_ocs.x * y_in_new_ocs.y - x_in_new_ocs.y * y_in_new_ocs.x) < 0.0;

        let transform_ocs_point = |p: Vector2| -> Vector2 {
            let p_wcs = old_ocs_to_wcs * Vector3::new(p.x, p.y, old_elevation);
            let p_new_wcs = transform.apply(p_wcs);
            let p_new_ocs = new_wcs_to_ocs * p_new_wcs;
            Vector2::new(p_new_ocs.x, p_new_ocs.y)
        };
        
        let transform_ocs_angle = |angle_rad: f64| -> f64 {
            let p_ocs = Vector2::new(angle_rad.cos(), angle_rad.sin());
            let p_wcs_dir = old_ocs_to_wcs * Vector3::new(p_ocs.x, p_ocs.y, 0.0);
            let transformed_wcs_dir = transform.apply_rotation(p_wcs_dir);
            let transformed_ocs_dir = new_wcs_to_ocs * transformed_wcs_dir;
            let mut new_angle_rad = transformed_ocs_dir.y.atan2(transformed_ocs_dir.x);
            if new_angle_rad < 0.0 { new_angle_rad += 2.0 * std::f64::consts::PI; }
            new_angle_rad
        };

        for path in &mut self.paths {
            for edge in &mut path.edges {
                match edge {
                    BoundaryEdge::Line(line) => {
                        line.start = transform_ocs_point(line.start);
                        line.end = transform_ocs_point(line.end);
                    }
                    BoundaryEdge::CircularArc(arc) => {
                        let new_start = transform_ocs_angle(arc.start_angle);
                        let new_end = transform_ocs_angle(arc.end_angle);
                        let center = transform_ocs_point(arc.center);
                        
                        if is_uniform {
                            arc.center = center;
                            arc.radius *= scale_x;
                            if is_flipped {
                                arc.counter_clockwise = !arc.counter_clockwise;
                            }
                            arc.start_angle = new_start;
                            arc.end_angle = new_end;
                        } else {
                            let major_axis_wcs = trans_ocs_x_wcs * arc.radius;
                            let major_axis_ocs_3d = new_wcs_to_ocs * major_axis_wcs;
                            let major_axis_endpoint = Vector2::new(major_axis_ocs_3d.x, major_axis_ocs_3d.y);
                            
                            let mut ellipse = EllipticArcEdge {
                                center,
                                major_axis_endpoint,
                                minor_axis_ratio: scale_y / scale_x,
                                start_angle: arc.start_angle,
                                end_angle: arc.end_angle,
                                counter_clockwise: arc.counter_clockwise,
                            };
                            if is_flipped {
                                ellipse.counter_clockwise = !ellipse.counter_clockwise;
                            }
                            
                            if ellipse.minor_axis_ratio > 1.0 {
                                let old_major = ellipse.major_axis_endpoint;
                                let old_major_len = old_major.length();
                                let new_major_len = old_major_len * ellipse.minor_axis_ratio;
                                let new_major_dir = Vector2::new(-old_major.y, old_major.x).normalize();
                                ellipse.major_axis_endpoint = new_major_dir * new_major_len;
                                ellipse.minor_axis_ratio = 1.0 / ellipse.minor_axis_ratio;
                                ellipse.start_angle -= std::f64::consts::FRAC_PI_2;
                                ellipse.end_angle -= std::f64::consts::FRAC_PI_2;
                            }
                            
                            *edge = BoundaryEdge::EllipticArc(ellipse);
                        }
                    }
                    BoundaryEdge::EllipticArc(ellipse) => {
                        let center = transform_ocs_point(ellipse.center);

                        let old_major_wcs = old_ocs_to_wcs * Vector3::new(ellipse.major_axis_endpoint.x, ellipse.major_axis_endpoint.y, 0.0);
                        let old_major_len = ellipse.major_axis_endpoint.length();
                        let old_minor_len = old_major_len * ellipse.minor_axis_ratio;
                        let old_minor_ocs_dir = Vector2::new(-ellipse.major_axis_endpoint.y, ellipse.major_axis_endpoint.x).normalize();
                        let old_minor_wcs = old_ocs_to_wcs * Vector3::new(old_minor_ocs_dir.x * old_minor_len, old_minor_ocs_dir.y * old_minor_len, 0.0);
                        
                        let new_major_wcs = transform.apply_rotation(old_major_wcs);
                        let new_minor_wcs = transform.apply_rotation(old_minor_wcs);
                        
                        let new_major_ocs_3d = new_wcs_to_ocs * new_major_wcs;
                        let new_minor_ocs_3d = new_wcs_to_ocs * new_minor_wcs;
                        
                        let new_major_ocs = Vector2::new(new_major_ocs_3d.x, new_major_ocs_3d.y);
                        let new_minor_ocs = Vector2::new(new_minor_ocs_3d.x, new_minor_ocs_3d.y);
                        
                        let new_major_len = new_major_ocs.length();
                        let new_minor_len = new_minor_ocs.length();
                        
                        ellipse.center = center;
                        if is_flipped {
                            ellipse.counter_clockwise = !ellipse.counter_clockwise;
                        }

                        if new_minor_len > new_major_len + 1e-12 {
                            ellipse.major_axis_endpoint = new_minor_ocs;
                            ellipse.minor_axis_ratio = new_major_len / new_minor_len;
                            ellipse.start_angle -= std::f64::consts::FRAC_PI_2;
                            ellipse.end_angle -= std::f64::consts::FRAC_PI_2;
                        } else {
                            ellipse.major_axis_endpoint = new_major_ocs;
                            ellipse.minor_axis_ratio = if new_major_len > 1e-12 { new_minor_len / new_major_len } else { 1.0 };
                        }
                    }
                    BoundaryEdge::Spline(spline) => {
                        for cp in &mut spline.control_points {
                            let p_wcs = old_ocs_to_wcs * Vector3::new(cp.x, cp.y, old_elevation);
                            let p_new_wcs = transform.apply(p_wcs);
                            let p_new_ocs = new_wcs_to_ocs * p_new_wcs;
                            cp.x = p_new_ocs.x;
                            cp.y = p_new_ocs.y;
                        }
                        for fp in &mut spline.fit_points {
                            *fp = transform_ocs_point(*fp);
                        }
                        let trans_dir = |d: Vector2| -> Vector2 {
                             let d_wcs = old_ocs_to_wcs * Vector3::new(d.x, d.y, 0.0);
                             let d_new_wcs = transform.apply_rotation(d_wcs);
                             let d_new_ocs = new_wcs_to_ocs * d_new_wcs;
                             Vector2::new(d_new_ocs.x, d_new_ocs.y)
                        };
                        spline.start_tangent = trans_dir(spline.start_tangent);
                        spline.end_tangent = trans_dir(spline.end_tangent);
                    }
                    BoundaryEdge::Polyline(poly) => {
                        for v in &mut poly.vertices {
                            let t = transform_ocs_point(Vector2::new(v.x, v.y));
                            v.x = t.x;
                            v.y = t.y;
                            if is_flipped {
                                v.z = -v.z;
                            }
                        }
                    }
                }
            }
        }

        for seed in &mut self.seed_points {
            *seed = transform_ocs_point(*seed);
        }
        
        let p_dir = Vector2::new(self.pattern_angle.cos(), self.pattern_angle.sin());
        let p_wcs_dir = old_ocs_to_wcs * Vector3::new(p_dir.x, p_dir.y, 0.0);
        let transformed_p_wcs_dir = transform.apply_rotation(p_wcs_dir);
        let transformed_p_ocs_dir = new_wcs_to_ocs * transformed_p_wcs_dir;
        self.pattern_angle = transformed_p_ocs_dir.y.atan2(transformed_p_ocs_dir.x);
        self.pattern_scale *= scale_x;

        for line in &mut self.pattern.lines {
            let l_dir = Vector2::new(line.angle.cos(), line.angle.sin());
            let l_wcs_dir = old_ocs_to_wcs * Vector3::new(l_dir.x, l_dir.y, 0.0);
            let transformed_l_wcs_dir = transform.apply_rotation(l_wcs_dir);
            let transformed_l_ocs_dir = new_wcs_to_ocs * transformed_l_wcs_dir;
            line.angle = transformed_l_ocs_dir.y.atan2(transformed_l_ocs_dir.x);
            
            line.base_point = transform_ocs_point(line.base_point);
            
            let off_wcs = old_ocs_to_wcs * Vector3::new(line.offset.x, line.offset.y, 0.0);
            let transformed_off_wcs = transform.apply_rotation(off_wcs);
            let transformed_off_ocs = new_wcs_to_ocs * transformed_off_wcs;
            line.offset = Vector2::new(transformed_off_ocs.x, transformed_off_ocs.y);
            
            for dash in &mut line.dash_lengths {
                *dash *= scale_x;
            }
        }

        if self.gradient_color.enabled {
            let g_dir = Vector2::new(self.gradient_color.angle.cos(), self.gradient_color.angle.sin());
            let g_wcs_dir = old_ocs_to_wcs * Vector3::new(g_dir.x, g_dir.y, 0.0);
            let transformed_g_wcs_dir = transform.apply_rotation(g_wcs_dir);
            let transformed_g_ocs_dir = new_wcs_to_ocs * transformed_g_wcs_dir;
            self.gradient_color.angle = transformed_g_ocs_dir.y.atan2(transformed_g_ocs_dir.x);
        }

        self.normal = new_normal;
        self.elevation = new_elevation;
    }
    
    fn apply_mirror(&mut self, transform: &crate::types::Transform) {
        use crate::types::normalize_angle;
        
        self.apply_transform(transform);
        
        // Mirror-specific corrections for boundary edges
        for path in &mut self.paths {
            for edge in &mut path.edges {
                match edge {
                    BoundaryEdge::CircularArc(arc) => {
                        // Recalculate mirrored angles and flip direction
                        let old_start = arc.start_angle;
                        let old_end = arc.end_angle;
                        
                        // Mirror the angle endpoints using the transform
                        let center_3d = Vector3::new(arc.center.x, arc.center.y, 0.0);
                        let start_pt = Vector3::new(
                            arc.center.x + arc.radius * old_start.cos(),
                            arc.center.y + arc.radius * old_start.sin(),
                            0.0,
                        );
                        let end_pt = Vector3::new(
                            arc.center.x + arc.radius * old_end.cos(),
                            arc.center.y + arc.radius * old_end.sin(),
                            0.0,
                        );
                        let ms = transform.apply(start_pt);
                        let me = transform.apply(end_pt);
                        
                        // Swap start/end and recalculate (mirror reverses direction)
                        arc.start_angle = normalize_angle((me.y - center_3d.y).atan2(me.x - center_3d.x));
                        arc.end_angle = normalize_angle((ms.y - center_3d.y).atan2(ms.x - center_3d.x));
                        arc.counter_clockwise = !arc.counter_clockwise;
                    }
                    BoundaryEdge::EllipticArc(ellipse) => {
                        // Mirror reverses parametric direction
                        let new_start = -ellipse.end_angle;
                        let new_end = -ellipse.start_angle;
                        ellipse.start_angle = new_start;
                        ellipse.end_angle = new_end;
                        ellipse.counter_clockwise = !ellipse.counter_clockwise;
                    }
                    BoundaryEdge::Polyline(poly) => {
                        // Negate bulge values (stored in z component)
                        for v in &mut poly.vertices {
                            v.z = -v.z;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
<<<<<<< HEAD

=======
>>>>>>> 7d754f76d3718e5503ef7ab69060b3b868b665fb
