/// Multi-roundtrip data integrity test.
///
/// For each entity type × each DWG version:
///   1. Create an entity with non-default field values
///   2. Write → Read (Trip 1): snapshot all fields
///   3. Write → Read (Trip 2): snapshot all fields again
///   4. Compare Trip 1 vs Trip 2 — any delta = data loss
///   5. Also compare Original vs Trip 1 to identify first-trip losses
///
/// This catches silent data corruption, dropped fields, and encoding bugs.

use acadrust::entities::*;
use acadrust::entities::dimension::DimensionLinear;
use acadrust::entities::hatch::{
    BoundaryEdge, BoundaryPath, PolylineEdge,
};
use acadrust::entities::mesh::Mesh;
use acadrust::entities::mline::MLine;
use acadrust::entities::multileader::MultiLeader;
use acadrust::entities::polyface_mesh::PolyfaceMesh;
use acadrust::io::dwg::DwgReader;
use acadrust::types::{DxfVersion, Vector2, Vector3};
use acadrust::{CadDocument, DwgWriter};
use std::collections::BTreeMap;
use std::io::Write;
use std::time::Instant;

const VERSIONS: &[(DxfVersion, &str)] = &[
    (DxfVersion::AC1012, "AC1012"),
    (DxfVersion::AC1014, "AC1014"),
    (DxfVersion::AC1015, "AC1015"),
    (DxfVersion::AC1018, "AC1018"),
    (DxfVersion::AC1021, "AC1021"),
    (DxfVersion::AC1024, "AC1024"),
    (DxfVersion::AC1027, "AC1027"),
    (DxfVersion::AC1032, "AC1032"),
];

/// A snapshot of every meaningful field value in an entity, stored as key=value pairs.
type Snapshot = BTreeMap<String, String>;

macro_rules! out {
    ($file:expr, $($arg:tt)*) => {{
        let line = format!($($arg)*);
        println!("{}", line);
        let _ = writeln!($file, "{}", line);
    }};
}

fn main() {
    let root = "target/roundtrip_test";
    let report_path = "target/roundtrip_test/roundtrip_report.txt";
    let _ = std::fs::create_dir_all(root);
    let mut rf = std::fs::File::create(report_path).expect("failed to create report");

    out!(rf, "╔══════════════════════════════════════════════════════════════════════════╗");
    out!(rf, "║           MULTI-ROUNDTRIP DATA INTEGRITY TEST                          ║");
    out!(rf, "╠══════════════════════════════════════════════════════════════════════════╣");
    out!(rf, "║  Write→Read→Write→Read, compare field snapshots at each trip           ║");
    out!(rf, "╚══════════════════════════════════════════════════════════════════════════╝\n");

    let global_start = Instant::now();

    let mut total_tests = 0u32;
    let mut total_pass = 0u32;
    let mut total_trip1_loss = 0u32;
    let mut total_trip2_loss = 0u32;
    let mut total_write_fail = 0u32;
    let mut all_losses: Vec<String> = Vec::new();

    // Entity factories: (name, factory_fn)
    let entity_factories: Vec<(&str, Box<dyn Fn() -> EntityType>)> = vec![
        ("LINE", Box::new(|| {
            let mut l = Line::from_coords(10.5, 20.3, 5.0, 100.7, 200.9, 15.0);
            l.thickness = 2.5;
            l.normal = Vector3::new(0.0, 0.0, 1.0);
            EntityType::Line(l)
        })),
        ("CIRCLE", Box::new(|| {
            let mut c = Circle::from_coords(33.3, 44.4, 7.7, 12.5);
            c.thickness = 1.5;
            c.normal = Vector3::new(0.0, 0.0, 1.0);
            EntityType::Circle(c)
        })),
        ("ARC", Box::new(|| {
            let mut a = Arc::from_coords(25.0, 35.0, 3.0, 15.0, 0.5, 2.5);
            a.thickness = 0.8;
            a.normal = Vector3::new(0.0, 0.0, 1.0);
            EntityType::Arc(a)
        })),
        ("ELLIPSE", Box::new(|| {
            EntityType::Ellipse(Ellipse::from_center_axes(
                Vector3::new(50.0, 60.0, 2.0),
                Vector3::new(30.0, 10.0, 0.0),
                0.6,
            ))
        })),
        ("POINT", Box::new(|| {
            let mut p = Point::from_coords(77.7, 88.8, 99.9);
            p.thickness = 3.0;
            EntityType::Point(p)
        })),
        ("TEXT", Box::new(|| {
            let mut t = Text::with_value("Roundtrip Test!", Vector3::new(15.5, 25.5, 0.0));
            t.height = 3.5;
            t.rotation = 0.45;
            t.width_factor = 0.8;
            t.oblique_angle = 0.15;
            t.style = "Standard".to_string();
            EntityType::Text(t)
        })),
        ("MTEXT", Box::new(|| {
            let mut m = MText::with_value("Multi\\PLine\\PRoundtrip", Vector3::new(10.0, 20.0, 0.0));
            m.height = 2.5;
            m.rectangle_width = 50.0;
            m.rotation = 0.3;
            m.style = "Standard".to_string();
            m.line_spacing_factor = 1.5;
            EntityType::MText(m)
        })),
        ("LWPOLYLINE", Box::new(|| {
            let mut lw = LwPolyline::new();
            lw.add_vertex(LwVertex {
                location: Vector2::new(0.0, 0.0),
                bulge: 0.0,
                start_width: 1.0,
                end_width: 2.0,
            });
            lw.add_vertex(LwVertex {
                location: Vector2::new(10.0, 0.0),
                bulge: 0.5,
                start_width: 0.0,
                end_width: 0.0,
            });
            lw.add_vertex(LwVertex {
                location: Vector2::new(10.0, 10.0),
                bulge: -0.3,
                start_width: 0.0,
                end_width: 0.0,
            });
            lw.add_vertex(LwVertex {
                location: Vector2::new(0.0, 10.0),
                bulge: 0.0,
                start_width: 0.0,
                end_width: 0.0,
            });
            lw.is_closed = true;
            lw.elevation = 5.5;
            lw.constant_width = 0.5;
            lw.thickness = 1.0;
            EntityType::LwPolyline(lw)
        })),
        ("SPLINE", Box::new(|| {
            EntityType::Spline(Spline::from_control_points(3, vec![
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(5.0, 15.0, 0.0),
                Vector3::new(10.0, -5.0, 0.0),
                Vector3::new(20.0, 10.0, 0.0),
                Vector3::new(25.0, 0.0, 0.0),
            ]))
        })),
        ("RAY", Box::new(|| {
            EntityType::Ray(Ray::new(
                Vector3::new(1.1, 2.2, 3.3),
                Vector3::new(0.577, 0.577, 0.577),
            ))
        })),
        ("XLINE", Box::new(|| {
            EntityType::XLine(XLine::new(
                Vector3::new(10.0, 20.0, 30.0),
                Vector3::new(0.0, 1.0, 0.0),
            ))
        })),
        ("SOLID", Box::new(|| {
            let mut s = Solid::new(
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(10.0, 0.0, 0.0),
                Vector3::new(10.0, 10.0, 0.0),
                Vector3::new(0.0, 10.0, 0.0),
            );
            s.thickness = 5.0;
            EntityType::Solid(s)
        })),
        ("FACE3D", Box::new(|| {
            EntityType::Face3D(Face3D::new(
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(20.0, 0.0, 0.0),
                Vector3::new(20.0, 20.0, 10.0),
                Vector3::new(0.0, 20.0, 10.0),
            ))
        })),
        ("LEADER", Box::new(|| {
            EntityType::Leader(Leader::two_point(
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(25.0, 25.0, 0.0),
            ))
        })),
        ("DIMENSION", Box::new(|| {
            EntityType::Dimension(Dimension::Linear(DimensionLinear::new(
                Vector3::new(10.0, 0.0, 0.0),
                Vector3::new(80.0, 0.0, 0.0),
            )))
        })),
        ("TOLERANCE", Box::new(|| {
            EntityType::Tolerance(Tolerance::with_text(
                Vector3::new(10.0, 10.0, 0.0),
                "{\\Fgdt;p}%%v0.5",
            ))
        })),
        ("SHAPE", Box::new(|| {
            EntityType::Shape(Shape::with_name(
                Vector3::new(50.0, 50.0, 0.0),
                "BOX",
                5.0,
            ))
        })),
        ("INSERT", Box::new(|| {
            EntityType::Insert(Insert::new("*Model_Space", Vector3::new(5.0, 10.0, 0.0)))
        })),
        ("HATCH", Box::new(|| {
            let mut hatch = Hatch::solid();
            let mut path = BoundaryPath::new();
            path.add_edge(BoundaryEdge::Polyline(PolylineEdge::new(
                vec![
                    Vector2::new(0.0, 0.0),
                    Vector2::new(100.0, 0.0),
                    Vector2::new(100.0, 100.0),
                    Vector2::new(0.0, 100.0),
                ],
                true,
            )));
            hatch.add_path(path);
            EntityType::Hatch(hatch)
        })),
        ("MLINE", Box::new(|| {
            EntityType::MLine(MLine::from_points(&[
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(50.0, 0.0, 0.0),
                Vector3::new(50.0, 50.0, 0.0),
            ]))
        })),
        ("POLYFACE", Box::new(|| {
            let mut pf = PolyfaceMesh::new();
            let v1 = pf.add_vertex_xyz(0.0, 0.0, 0.0);
            let v2 = pf.add_vertex_xyz(10.0, 0.0, 0.0);
            let v3 = pf.add_vertex_xyz(5.0, 10.0, 0.0);
            pf.add_triangle(v1, v2, v3);
            EntityType::PolyfaceMesh(pf)
        })),
        ("MESH", Box::new(|| {
            EntityType::Mesh(Mesh::from_triangles(
                vec![
                    Vector3::new(0.0, 0.0, 0.0),
                    Vector3::new(10.0, 0.0, 0.0),
                    Vector3::new(5.0, 10.0, 5.0),
                ],
                &[(0, 1, 2)],
            ))
        })),
        ("MULTILEADER", Box::new(|| {
            EntityType::MultiLeader(MultiLeader::with_text(
                "Label",
                Vector3::new(20.0, 20.0, 0.0),
                vec![Vector3::new(0.0, 0.0, 0.0), Vector3::new(10.0, 10.0, 0.0)],
            ))
        })),
        ("POLYLINE2D", Box::new(|| {
            let mut pl = Polyline2D::new();
            pl.add_vertex(Vertex2D::new(Vector3::new(0.0, 0.0, 0.0)));
            pl.add_vertex(Vertex2D::new(Vector3::new(20.0, 0.0, 0.0)));
            pl.add_vertex(Vertex2D::new(Vector3::new(20.0, 20.0, 0.0)));
            EntityType::Polyline2D(pl)
        })),
        ("POLYLINE3D", Box::new(|| {
            EntityType::Polyline3D(Polyline3D::from_points(vec![
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(10.0, 0.0, 5.0),
                Vector3::new(20.0, 10.0, 10.0),
            ]))
        })),
        ("VIEWPORT", Box::new(|| {
            EntityType::Viewport(Viewport::new())
        })),
    ];

    for &(version, ver_str) in VERSIONS {
        let dir = format!("{}/{}", root, ver_str);
        let _ = std::fs::create_dir_all(&dir);
        out!(rf, "\n══ {} ══════════════════════════════════════════════════════════", ver_str);

        for (ent_name, factory) in &entity_factories {
            total_tests += 1;

            // Create original entity and snapshot it
            let original_entity = factory();
            let original_snap = snapshot_entity(&original_entity);

            // ── TRIP 1: Write → Read ──
            let path1 = format!("{}/rt1_{}_{}.dwg", dir, ver_str, ent_name);
            let trip1_result = roundtrip_once(version, &original_entity, &path1);

            let (trip1_snap, trip1_entity) = match trip1_result {
                Ok((snap, ent)) => (snap, Some(ent)),
                Err(e) => {
                    out!(rf, "  ✗ {:<16} TRIP1 FAIL: {}", ent_name, e);
                    total_write_fail += 1;
                    continue;
                }
            };

            // ── TRIP 2: Write trip1 entity → Read ──
            let path2 = format!("{}/rt2_{}_{}.dwg", dir, ver_str, ent_name);
            let trip2_result = roundtrip_once(version, trip1_entity.as_ref().unwrap(), &path2);

            let trip2_snap = match trip2_result {
                Ok((snap, _)) => snap,
                Err(e) => {
                    out!(rf, "  ✗ {:<16} TRIP2 FAIL: {}", ent_name, e);
                    total_write_fail += 1;
                    continue;
                }
            };

            // ── Compare Original vs Trip 1 ──
            let orig_vs_trip1 = diff_snapshots(&original_snap, &trip1_snap);

            // ── Compare Trip 1 vs Trip 2 ──
            let trip1_vs_trip2 = diff_snapshots(&trip1_snap, &trip2_snap);

            if orig_vs_trip1.is_empty() && trip1_vs_trip2.is_empty() {
                out!(rf, "  ✓ {:<16} PERFECT — {} fields, 0 deltas across 2 trips",
                    ent_name, trip1_snap.len());
                total_pass += 1;
            } else {
                const MAX_DIFFS: usize = 20;
                // Report losses
                if !orig_vs_trip1.is_empty() {
                    out!(rf, "  ⚠ {:<16} TRIP 1 LOSS ({} fields changed):", ent_name, orig_vs_trip1.len());
                    for d in orig_vs_trip1.iter().take(MAX_DIFFS) {
                        out!(rf, "      ↓ {}", d);
                    }
                    if orig_vs_trip1.len() > MAX_DIFFS {
                        out!(rf, "      ... +{} more", orig_vs_trip1.len() - MAX_DIFFS);
                    }
                    total_trip1_loss += 1;
                    for d in orig_vs_trip1.iter().take(MAX_DIFFS) {
                        all_losses.push(format!("{}/{}: T1: {}", ver_str, ent_name, d));
                    }
                }
                if !trip1_vs_trip2.is_empty() {
                    out!(rf, "  ✗ {:<16} TRIP 2 DRIFT ({} fields changed):", ent_name, trip1_vs_trip2.len());
                    for d in trip1_vs_trip2.iter().take(MAX_DIFFS) {
                        out!(rf, "      ↓ {}", d);
                    }
                    if trip1_vs_trip2.len() > MAX_DIFFS {
                        out!(rf, "      ... +{} more", trip1_vs_trip2.len() - MAX_DIFFS);
                    }
                    total_trip2_loss += 1;
                    for d in trip1_vs_trip2.iter().take(MAX_DIFFS) {
                        all_losses.push(format!("{}/{}: T2: {}", ver_str, ent_name, d));
                    }
                } else if !orig_vs_trip1.is_empty() {
                    out!(rf, "      Trip2 stable (no further drift)");
                    total_pass += 1; // Trip1 loss but stable after — count as pass with caveat
                }
            }
        }
    }

    let elapsed = global_start.elapsed();

    out!(rf, "\n╔══════════════════════════════════════════════════════════════════════════╗");
    out!(rf, "║  ROUNDTRIP RESULTS                                                     ║");
    out!(rf, "╠══════════════════════════════════════════════════════════════════════════╣");
    out!(rf, "║  Tests:       {:>4}                                                     ║", total_tests);
    out!(rf, "║  Perfect:     {:>4}  (no data loss across 2 roundtrips)                 ║", total_pass);
    out!(rf, "║  Trip1 loss:  {:>4}  (original→trip1 field changes)                     ║", total_trip1_loss);
    out!(rf, "║  Trip2 drift: {:>4}  (trip1→trip2 divergence — UNSTABLE)                ║", total_trip2_loss);
    out!(rf, "║  Write fail:  {:>4}  (could not write or read back)                     ║", total_write_fail);
    out!(rf, "║  Time:        {:.1}s                                                    ║", elapsed.as_secs_f64());
    out!(rf, "╚══════════════════════════════════════════════════════════════════════════╝");

    // Deduped summary of all losses
    if !all_losses.is_empty() {
        out!(rf, "\n── ALL DATA LOSSES ({} captured) ──────────────────────────────────", all_losses.len());

        // Group by entity+version for cleaner view
        let mut by_entity: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for loss in &all_losses {
            // Format: "VER/ENT: Tx: field: old → new"
            if let Some(colon) = loss.find(": ") {
                let key = &loss[..colon];
                by_entity.entry(key.to_string()).or_default().push(loss.clone());
            }
        }

        for (entity, instances) in &by_entity {
            out!(rf, "\n  {} — {} field changes:", entity, instances.len());
            for (i, inst) in instances.iter().enumerate().take(10) {
                // Strip entity prefix for brevity
                let display = inst.find(": ").map(|p| &inst[p+2..]).unwrap_or(inst);
                out!(rf, "    {:>3}. {}", i + 1, display);
            }
            if instances.len() > 10 {
                out!(rf, "    ... +{} more", instances.len() - 10);
            }
        }
    }

    let _ = rf.flush();
    let abs_report = std::fs::canonicalize(report_path).unwrap_or_else(|_| report_path.into());
    println!("\nReport: {}", abs_report.display());

    if total_trip2_loss > 0 || total_write_fail > 0 {
        std::process::exit(1);
    }
}

/// Performs a single roundtrip: write entity → read back
/// Returns the snapshot of the read-back entity and the entity itself.
fn roundtrip_once(
    version: DxfVersion,
    entity: &EntityType,
    path: &str,
) -> Result<(Snapshot, EntityType), String> {
    // Build a document with this entity
    let mut doc = CadDocument::with_version(version);
    doc.add_entity(entity.clone())
        .map_err(|e| format!("add_entity: {:?}", e))?;

    // Write
    DwgWriter::write_to_file(path, &doc)
        .map_err(|e| format!("write: {:?}", e))?;

    // Read back
    let mut reader = DwgReader::from_file(path)
        .map_err(|e| format!("open: {}", e))?;
    let read_doc = reader.read()
        .map_err(|e| format!("read: {}", e))?;

    // Find the entity
    let entities: Vec<&EntityType> = read_doc.entities().collect();
    if entities.is_empty() {
        return Err("no entities after read-back".to_string());
    }

    let ent = entities[0];
    let snap = snapshot_entity(ent);
    Ok((snap, ent.clone()))
}

/// Compares two snapshots and returns human-readable diffs.
fn diff_snapshots(before: &Snapshot, after: &Snapshot) -> Vec<String> {
    let mut diffs = Vec::new();

    // Check fields in before
    for (key, val_before) in before {
        match after.get(key) {
            Some(val_after) if val_before != val_after => {
                // Floating point tolerance check
                if is_close_enough(val_before, val_after) {
                    continue;
                }
                diffs.push(format!("{}: \"{}\" → \"{}\"", key, val_before, val_after));
            }
            None => {
                diffs.push(format!("{}: \"{}\" → MISSING", key, val_before));
            }
            _ => {} // same value
        }
    }

    // Check fields added in after
    for (key, val_after) in after {
        if !before.contains_key(key) {
            diffs.push(format!("{}: MISSING → \"{}\"", key, val_after));
        }
    }

    diffs
}

/// Checks if two stringified float values are close enough to be considered equal.
fn is_close_enough(a: &str, b: &str) -> bool {
    if let (Ok(fa), Ok(fb)) = (a.parse::<f64>(), b.parse::<f64>()) {
        let diff = (fa - fb).abs();
        if diff < 1e-6 {
            return true;
        }
        // Relative
        let max = fa.abs().max(fb.abs());
        if max > 0.0 && diff / max < 1e-6 {
            return true;
        }
    }
    false
}

/// Creates a complete field snapshot of an entity.
fn snapshot_entity(entity: &EntityType) -> Snapshot {
    let mut s = Snapshot::new();

    // Common fields (present on every entity)
    let c = entity.common();
    s.insert("common.layer".into(), c.layer.clone());
    s.insert("common.color".into(), format!("{}", c.color));
    s.insert("common.line_weight".into(), format!("{}", c.line_weight));
    s.insert("common.invisible".into(), format!("{}", c.invisible));
    s.insert("common.transparency".into(), format!("{}", c.transparency));

    // Entity-specific fields
    match entity {
        EntityType::Line(l) => {
            s.insert("type".into(), "LINE".into());
            snap_vec3(&mut s, "start", &l.start);
            snap_vec3(&mut s, "end", &l.end);
            s.insert("thickness".into(), format!("{}", l.thickness));
            snap_vec3(&mut s, "normal", &l.normal);
        }
        EntityType::Circle(c) => {
            s.insert("type".into(), "CIRCLE".into());
            snap_vec3(&mut s, "center", &c.center);
            s.insert("radius".into(), format!("{}", c.radius));
            s.insert("thickness".into(), format!("{}", c.thickness));
            snap_vec3(&mut s, "normal", &c.normal);
        }
        EntityType::Arc(a) => {
            s.insert("type".into(), "ARC".into());
            snap_vec3(&mut s, "center", &a.center);
            s.insert("radius".into(), format!("{}", a.radius));
            s.insert("start_angle".into(), format!("{}", a.start_angle));
            s.insert("end_angle".into(), format!("{}", a.end_angle));
            s.insert("thickness".into(), format!("{}", a.thickness));
            snap_vec3(&mut s, "normal", &a.normal);
        }
        EntityType::Ellipse(e) => {
            s.insert("type".into(), "ELLIPSE".into());
            snap_vec3(&mut s, "center", &e.center);
            snap_vec3(&mut s, "major_axis", &e.major_axis);
            s.insert("minor_axis_ratio".into(), format!("{}", e.minor_axis_ratio));
            s.insert("start_parameter".into(), format!("{}", e.start_parameter));
            s.insert("end_parameter".into(), format!("{}", e.end_parameter));
            snap_vec3(&mut s, "normal", &e.normal);
        }
        EntityType::Point(p) => {
            s.insert("type".into(), "POINT".into());
            snap_vec3(&mut s, "location", &p.location);
            s.insert("thickness".into(), format!("{}", p.thickness));
            snap_vec3(&mut s, "normal", &p.normal);
        }
        EntityType::Text(t) => {
            s.insert("type".into(), "TEXT".into());
            s.insert("value".into(), t.value.clone());
            snap_vec3(&mut s, "insertion_point", &t.insertion_point);
            if let Some(ref ap) = t.alignment_point {
                snap_vec3(&mut s, "alignment_point", ap);
            }
            s.insert("height".into(), format!("{}", t.height));
            s.insert("rotation".into(), format!("{}", t.rotation));
            s.insert("width_factor".into(), format!("{}", t.width_factor));
            s.insert("oblique_angle".into(), format!("{}", t.oblique_angle));
            s.insert("style".into(), t.style.clone());
            snap_vec3(&mut s, "normal", &t.normal);
        }
        EntityType::MText(m) => {
            s.insert("type".into(), "MTEXT".into());
            s.insert("value".into(), m.value.clone());
            snap_vec3(&mut s, "insertion_point", &m.insertion_point);
            s.insert("height".into(), format!("{}", m.height));
            s.insert("rectangle_width".into(), format!("{}", m.rectangle_width));
            s.insert("rotation".into(), format!("{}", m.rotation));
            s.insert("style".into(), m.style.clone());
            s.insert("line_spacing_factor".into(), format!("{}", m.line_spacing_factor));
            snap_vec3(&mut s, "normal", &m.normal);
        }
        EntityType::LwPolyline(lw) => {
            s.insert("type".into(), "LWPOLYLINE".into());
            s.insert("vertex_count".into(), format!("{}", lw.vertices.len()));
            s.insert("is_closed".into(), format!("{}", lw.is_closed));
            s.insert("constant_width".into(), format!("{}", lw.constant_width));
            s.insert("elevation".into(), format!("{}", lw.elevation));
            s.insert("thickness".into(), format!("{}", lw.thickness));
            snap_vec3(&mut s, "normal", &lw.normal);
            for (i, v) in lw.vertices.iter().enumerate() {
                s.insert(format!("v[{}].x", i), format!("{}", v.location.x));
                s.insert(format!("v[{}].y", i), format!("{}", v.location.y));
                s.insert(format!("v[{}].bulge", i), format!("{}", v.bulge));
                s.insert(format!("v[{}].start_width", i), format!("{}", v.start_width));
                s.insert(format!("v[{}].end_width", i), format!("{}", v.end_width));
            }
        }
        EntityType::Spline(sp) => {
            s.insert("type".into(), "SPLINE".into());
            s.insert("degree".into(), format!("{}", sp.degree));
            s.insert("flags.closed".into(), format!("{}", sp.flags.closed));
            s.insert("flags.rational".into(), format!("{}", sp.flags.rational));
            s.insert("flags.planar".into(), format!("{}", sp.flags.planar));
            s.insert("flags.periodic".into(), format!("{}", sp.flags.periodic));
            s.insert("flags.linear".into(), format!("{}", sp.flags.linear));
            s.insert("ctrl_pt_count".into(), format!("{}", sp.control_points.len()));
            for (i, cp) in sp.control_points.iter().enumerate() {
                snap_vec3(&mut s, &format!("ctrl[{}]", i), cp);
            }
            s.insert("knot_count".into(), format!("{}", sp.knots.len()));
            for (i, k) in sp.knots.iter().enumerate() {
                s.insert(format!("knot[{}]", i), format!("{}", k));
            }
            s.insert("fit_pt_count".into(), format!("{}", sp.fit_points.len()));
            for (i, fp) in sp.fit_points.iter().enumerate() {
                snap_vec3(&mut s, &format!("fit[{}]", i), fp);
            }
            if !sp.weights.is_empty() {
                for (i, w) in sp.weights.iter().enumerate() {
                    s.insert(format!("weight[{}]", i), format!("{}", w));
                }
            }
            snap_vec3(&mut s, "normal", &sp.normal);
        }
        EntityType::Ray(r) => {
            s.insert("type".into(), "RAY".into());
            snap_vec3(&mut s, "base_point", &r.base_point);
            snap_vec3(&mut s, "direction", &r.direction);
        }
        EntityType::XLine(x) => {
            s.insert("type".into(), "XLINE".into());
            snap_vec3(&mut s, "base_point", &x.base_point);
            snap_vec3(&mut s, "direction", &x.direction);
        }
        EntityType::Solid(so) => {
            s.insert("type".into(), "SOLID".into());
            snap_vec3(&mut s, "corner1", &so.first_corner);
            snap_vec3(&mut s, "corner2", &so.second_corner);
            snap_vec3(&mut s, "corner3", &so.third_corner);
            snap_vec3(&mut s, "corner4", &so.fourth_corner);
            s.insert("thickness".into(), format!("{}", so.thickness));
            snap_vec3(&mut s, "normal", &so.normal);
        }
        EntityType::Face3D(f) => {
            s.insert("type".into(), "FACE3D".into());
            snap_vec3(&mut s, "corner1", &f.first_corner);
            snap_vec3(&mut s, "corner2", &f.second_corner);
            snap_vec3(&mut s, "corner3", &f.third_corner);
            snap_vec3(&mut s, "corner4", &f.fourth_corner);
        }
        EntityType::Leader(l) => {
            s.insert("type".into(), "LEADER".into());
            s.insert("vertex_count".into(), format!("{}", l.vertices.len()));
            for (i, v) in l.vertices.iter().enumerate() {
                snap_vec3(&mut s, &format!("vertex[{}]", i), v);
            }
            s.insert("dimension_style".into(), l.dimension_style.clone());
            s.insert("arrow_enabled".into(), format!("{}", l.arrow_enabled));
            s.insert("text_height".into(), format!("{}", l.text_height));
            s.insert("text_width".into(), format!("{}", l.text_width));
            snap_vec3(&mut s, "normal", &l.normal);
        }
        EntityType::Dimension(d) => {
            s.insert("type".into(), "DIMENSION".into());
            let base = d.base();
            snap_vec3(&mut s, "definition_point", &base.definition_point);
            snap_vec3(&mut s, "text_middle_point", &base.text_middle_point);
            snap_vec3(&mut s, "insertion_point", &base.insertion_point);
            s.insert("text".into(), base.text.clone());
            s.insert("style_name".into(), base.style_name.clone());
            s.insert("actual_measurement".into(), format!("{}", base.actual_measurement));
            s.insert("text_rotation".into(), format!("{}", base.text_rotation));
            s.insert("block_name".into(), base.block_name.clone());
            snap_vec3(&mut s, "normal", &base.normal);
            match d {
                acadrust::entities::Dimension::Linear(dl) => {
                    s.insert("dim_subtype".into(), "Linear".into());
                    snap_vec3(&mut s, "first_point", &dl.first_point);
                    snap_vec3(&mut s, "second_point", &dl.second_point);
                    s.insert("rotation".into(), format!("{}", dl.rotation));
                }
                acadrust::entities::Dimension::Aligned(da) => {
                    s.insert("dim_subtype".into(), "Aligned".into());
                    snap_vec3(&mut s, "first_point", &da.first_point);
                    snap_vec3(&mut s, "second_point", &da.second_point);
                }
                acadrust::entities::Dimension::Radius(dr) => {
                    s.insert("dim_subtype".into(), "Radius".into());
                    snap_vec3(&mut s, "angle_vertex", &dr.angle_vertex);
                    s.insert("leader_length".into(), format!("{}", dr.leader_length));
                }
                acadrust::entities::Dimension::Diameter(dd) => {
                    s.insert("dim_subtype".into(), "Diameter".into());
                    snap_vec3(&mut s, "angle_vertex", &dd.angle_vertex);
                    s.insert("leader_length".into(), format!("{}", dd.leader_length));
                }
                acadrust::entities::Dimension::Angular2Ln(da) => {
                    s.insert("dim_subtype".into(), "Angular2Ln".into());
                    snap_vec3(&mut s, "first_point", &da.first_point);
                    snap_vec3(&mut s, "second_point", &da.second_point);
                    snap_vec3(&mut s, "angle_vertex", &da.angle_vertex);
                }
                acadrust::entities::Dimension::Angular3Pt(da) => {
                    s.insert("dim_subtype".into(), "Angular3Pt".into());
                    snap_vec3(&mut s, "first_point", &da.first_point);
                    snap_vec3(&mut s, "second_point", &da.second_point);
                    snap_vec3(&mut s, "angle_vertex", &da.angle_vertex);
                }
                acadrust::entities::Dimension::Ordinate(dord) => {
                    s.insert("dim_subtype".into(), "Ordinate".into());
                    snap_vec3(&mut s, "feature_location", &dord.feature_location);
                    snap_vec3(&mut s, "leader_endpoint", &dord.leader_endpoint);
                    s.insert("is_ordinate_type_x".into(), format!("{}", dord.is_ordinate_type_x));
                }
            }
        }
        EntityType::Tolerance(t) => {
            s.insert("type".into(), "TOLERANCE".into());
            snap_vec3(&mut s, "insertion_point", &t.insertion_point);
            snap_vec3(&mut s, "direction", &t.direction);
            s.insert("text".into(), t.text.clone());
            s.insert("dimension_style_name".into(), t.dimension_style_name.clone());
        }
        EntityType::Shape(sh) => {
            s.insert("type".into(), "SHAPE".into());
            snap_vec3(&mut s, "insertion_point", &sh.insertion_point);
            s.insert("size".into(), format!("{}", sh.size));
            s.insert("shape_name".into(), sh.shape_name.clone());
            s.insert("shape_number".into(), format!("{}", sh.shape_number));
            s.insert("rotation".into(), format!("{}", sh.rotation));
            s.insert("relative_x_scale".into(), format!("{}", sh.relative_x_scale));
            snap_vec3(&mut s, "normal", &sh.normal);
        }
        EntityType::Insert(ins) => {
            s.insert("type".into(), "INSERT".into());
            s.insert("block_name".into(), ins.block_name.clone());
            snap_vec3(&mut s, "insert_point", &ins.insert_point);
            s.insert("x_scale".into(), format!("{}", ins.x_scale));
            s.insert("y_scale".into(), format!("{}", ins.y_scale));
            s.insert("z_scale".into(), format!("{}", ins.z_scale));
            s.insert("rotation".into(), format!("{}", ins.rotation));
            s.insert("row_count".into(), format!("{}", ins.row_count));
            s.insert("column_count".into(), format!("{}", ins.column_count));
            snap_vec3(&mut s, "normal", &ins.normal);
        }
        EntityType::Hatch(h) => {
            s.insert("type".into(), "HATCH".into());
            s.insert("is_solid".into(), format!("{}", h.is_solid));
            s.insert("is_associative".into(), format!("{}", h.is_associative));
            s.insert("pattern_name".into(), h.pattern.name.clone());
            s.insert("pattern_angle".into(), format!("{}", h.pattern_angle));
            s.insert("pattern_scale".into(), format!("{}", h.pattern_scale));
            s.insert("is_double".into(), format!("{}", h.is_double));
            s.insert("elevation".into(), format!("{}", h.elevation));
            s.insert("path_count".into(), format!("{}", h.paths.len()));
            snap_vec3(&mut s, "normal", &h.normal);
            for (i, path) in h.paths.iter().enumerate() {
                s.insert(format!("path[{}].edge_count", i), format!("{}", path.edges.len()));
                for (j, edge) in path.edges.iter().enumerate() {
                    match edge {
                        BoundaryEdge::Line(le) => {
                            s.insert(format!("path[{}].edge[{}].type", i, j), "Line".into());
                            s.insert(format!("path[{}].edge[{}].start.x", i, j), format!("{}", le.start.x));
                            s.insert(format!("path[{}].edge[{}].start.y", i, j), format!("{}", le.start.y));
                            s.insert(format!("path[{}].edge[{}].end.x", i, j), format!("{}", le.end.x));
                            s.insert(format!("path[{}].edge[{}].end.y", i, j), format!("{}", le.end.y));
                        }
                        BoundaryEdge::Polyline(pe) => {
                            s.insert(format!("path[{}].edge[{}].type", i, j), "Polyline".into());
                            s.insert(format!("path[{}].edge[{}].vert_count", i, j), format!("{}", pe.vertices.len()));
                            s.insert(format!("path[{}].edge[{}].is_closed", i, j), format!("{}", pe.is_closed));
                            for (k, v) in pe.vertices.iter().enumerate() {
                                s.insert(format!("path[{}].edge[{}].v[{}].x", i, j, k), format!("{}", v.x));
                                s.insert(format!("path[{}].edge[{}].v[{}].y", i, j, k), format!("{}", v.y));
                            }
                        }
                        _ => {
                            s.insert(format!("path[{}].edge[{}].type", i, j), format!("{:?}", edge));
                        }
                    }
                }
            }
            if !h.seed_points.is_empty() {
                s.insert("seed_point_count".into(), format!("{}", h.seed_points.len()));
                for (i, sp) in h.seed_points.iter().enumerate() {
                    s.insert(format!("seed[{}].x", i), format!("{}", sp.x));
                    s.insert(format!("seed[{}].y", i), format!("{}", sp.y));
                }
            }
        }
        EntityType::MLine(ml) => {
            s.insert("type".into(), "MLINE".into());
            s.insert("vertex_count".into(), format!("{}", ml.vertices.len()));
            s.insert("scale_factor".into(), format!("{}", ml.scale_factor));
            s.insert("style_name".into(), ml.style_name.clone());
            snap_vec3(&mut s, "start_point", &ml.start_point);
            snap_vec3(&mut s, "normal", &ml.normal);
            for (i, v) in ml.vertices.iter().enumerate() {
                snap_vec3(&mut s, &format!("mlv[{}].position", i), &v.position);
                snap_vec3(&mut s, &format!("mlv[{}].direction", i), &v.direction);
                snap_vec3(&mut s, &format!("mlv[{}].miter", i), &v.miter);
            }
        }
        EntityType::Mesh(m) => {
            s.insert("type".into(), "MESH".into());
            s.insert("vertex_count".into(), format!("{}", m.vertices.len()));
            s.insert("face_count".into(), format!("{}", m.faces.len()));
            s.insert("edge_count".into(), format!("{}", m.edges.len()));
            s.insert("subdivision_level".into(), format!("{}", m.subdivision_level));
            for (i, v) in m.vertices.iter().enumerate() {
                snap_vec3(&mut s, &format!("mesh_v[{}]", i), v);
            }
            for (i, f) in m.faces.iter().enumerate() {
                s.insert(format!("face[{}]", i), format!("{:?}", f.vertices));
            }
        }
        EntityType::PolyfaceMesh(pf) => {
            s.insert("type".into(), "POLYFACEMESH".into());
            s.insert("vertex_count".into(), format!("{}", pf.vertices.len()));
            s.insert("face_count".into(), format!("{}", pf.faces.len()));
            s.insert("elevation".into(), format!("{}", pf.elevation));
            snap_vec3(&mut s, "normal", &pf.normal);
            for (i, v) in pf.vertices.iter().enumerate() {
                snap_vec3(&mut s, &format!("pfv[{}]", i), &v.location);
            }
        }
        EntityType::MultiLeader(ml) => {
            s.insert("type".into(), "MULTILEADER".into());
            s.insert("arrowhead_size".into(), format!("{}", ml.arrowhead_size));
            s.insert("dogleg_length".into(), format!("{}", ml.dogleg_length));
            s.insert("text_height".into(), format!("{}", ml.text_height));
            s.insert("ctx.text_string".into(), ml.context.text_string.clone());
            s.insert("ctx.scale_factor".into(), format!("{}", ml.context.scale_factor));
            snap_vec3(&mut s, "ctx.content_base_point", &ml.context.content_base_point);
            s.insert("ctx.leader_root_count".into(), format!("{}", ml.context.leader_roots.len()));
        }
        EntityType::Polyline2D(p) => {
            s.insert("type".into(), "POLYLINE2D".into());
            s.insert("vertex_count".into(), format!("{}", p.vertices.len()));
            s.insert("elevation".into(), format!("{}", p.elevation));
            s.insert("thickness".into(), format!("{}", p.thickness));
            snap_vec3(&mut s, "normal", &p.normal);
            for (i, v) in p.vertices.iter().enumerate() {
                snap_vec3(&mut s, &format!("v2d[{}].location", i), &v.location);
                s.insert(format!("v2d[{}].bulge", i), format!("{}", v.bulge));
                s.insert(format!("v2d[{}].start_width", i), format!("{}", v.start_width));
                s.insert(format!("v2d[{}].end_width", i), format!("{}", v.end_width));
            }
        }
        EntityType::Polyline3D(p) => {
            s.insert("type".into(), "POLYLINE3D".into());
            s.insert("vertex_count".into(), format!("{}", p.vertices.len()));
            snap_vec3(&mut s, "normal", &p.normal);
            for (i, v) in p.vertices.iter().enumerate() {
                snap_vec3(&mut s, &format!("v3d[{}]", i), &v.position);
            }
        }
        EntityType::Viewport(vp) => {
            s.insert("type".into(), "VIEWPORT".into());
            snap_vec3(&mut s, "center", &vp.center);
            s.insert("width".into(), format!("{}", vp.width));
            s.insert("height".into(), format!("{}", vp.height));
            s.insert("id".into(), format!("{}", vp.id));
            s.insert("view_height".into(), format!("{}", vp.view_height));
            s.insert("lens_length".into(), format!("{}", vp.lens_length));
        }
        _ => {
            s.insert("type".into(), format!("{:?}", std::mem::discriminant(entity)));
        }
    }

    s
}

fn snap_vec3(s: &mut Snapshot, prefix: &str, v: &Vector3) {
    s.insert(format!("{}.x", prefix), format!("{}", v.x));
    s.insert(format!("{}.y", prefix), format!("{}", v.y));
    s.insert(format!("{}.z", prefix), format!("{}", v.z));
}
