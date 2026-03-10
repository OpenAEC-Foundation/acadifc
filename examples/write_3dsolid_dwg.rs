//! Example: write DWG files containing various 3DSOLID shapes.
//!
//! Demonstrates the SAT builder API with seven different solid shapes:
//!
//! - **Box** (10×10×10) — 6 planar faces, 12 edges, 8 vertices
//! - **Wedge** (right triangular prism) — 5 planar faces, 9 edges, 6 vertices
//! - **Pyramid** (square base, apex) — 5 planar faces, 8 edges, 5 vertices
//! - **Cylinder** (radius 5, height 10) — 2 planar caps + 1 cylindrical surface
//! - **Cone** (base radius 5, height 10) — 1 conical surface + 1 base cap
//! - **Sphere** (radius 5) — 1 spherical surface, no edges/vertices
//! - **Torus** (major 5, minor 2) — 1 toroidal surface, no edges/vertices
//!
//! Each shape is written as R2013 (AC1027) DWG. The box is also written
//! at R2000 and R2004 for multi-version testing.
//!
//! ```
//! cargo run --example write_3dsolid_dwg
//! ```

use acadrust::{CadDocument, DwgWriter, DxfVersion, EntityType};
use acadrust::entities::Solid3D;
use acadrust::entities::acis::{SatDocument, SatPointer, SatToken, Sense, Sidedness};

fn main() -> acadrust::Result<()> {
    // ── 1. Build all shapes ─────────────────────────────────────────
    let box_sat = build_box_sat();
    let wedge_sat = build_wedge_sat();
    let pyramid_sat = build_pyramid_sat();
    let cylinder_sat = build_cylinder_sat();
    let cone_sat = build_cone_sat();
    let sphere_sat = build_sphere_sat();
    let torus_sat = build_torus_sat();

    // Print and validate all shapes
    print_sat_info("Box", &box_sat);
    print_sat_info("Wedge", &wedge_sat);
    print_sat_info("Pyramid", &pyramid_sat);
    print_sat_info("Cylinder", &cylinder_sat);
    print_sat_info("Cone", &cone_sat);
    print_sat_info("Sphere", &sphere_sat);
    print_sat_info("Torus", &torus_sat);

    // ── 2. Write box at multiple DWG versions ───────────────────────
    println!("\n=== Writing DWG files ===");
    write_solid("solid3d_r2000.dwg", DxfVersion::AC1015, &box_sat)?;
    write_solid("solid3d_r2004.dwg", DxfVersion::AC1018, &box_sat)?;
    write_solid("box_r2013.dwg", DxfVersion::AC1027, &box_sat)?;

    // ── 3. Write each shape at R2013 ────────────────────────────────
    write_solid("wedge_r2013.dwg", DxfVersion::AC1027, &wedge_sat)?;
    write_solid("pyramid_r2013.dwg", DxfVersion::AC1027, &pyramid_sat)?;
    write_solid("cylinder_r2013.dwg", DxfVersion::AC1027, &cylinder_sat)?;
    write_solid("cone_r2013.dwg", DxfVersion::AC1027, &cone_sat)?;
    write_solid("sphere_r2013.dwg", DxfVersion::AC1027, &sphere_sat)?;
    write_solid("torus_r2013.dwg", DxfVersion::AC1027, &torus_sat)?;

    // ── 4. Write cylinder at R2000 for SAT-text testing ─────────────
    write_solid("cylinder_r2000.dwg", DxfVersion::AC1015, &cylinder_sat)?;

    // ── 5. Read-back verification ───────────────────────────────────
    {
        use acadrust::DwgReader;
        println!("\n=== Read-back verification ===");

        for path in &[
            "solid3d_r2000.dwg",
            "solid3d_r2004.dwg",
            "box_r2013.dwg",
            "wedge_r2013.dwg",
            "pyramid_r2013.dwg",
            "cylinder_r2013.dwg",
            "cone_r2013.dwg",
            "sphere_r2013.dwg",
            "torus_r2013.dwg",
            "cylinder_r2000.dwg",
        ] {
            let mut reader = DwgReader::from_file(path)?;
            let doc = reader.read()?;
            let solids: Vec<&Solid3D> = doc.entities().filter_map(|e| {
                if let EntityType::Solid3D(s) = e { Some(s) } else { None }
            }).collect();

            let label = path.trim_end_matches(".dwg");
            if let Some(s) = solids.first() {
                let parsed = s.parse_sat();
                let (b, f, e, v) = parsed.as_ref().map_or(
                    (0, 0, 0, 0),
                    |p| (p.bodies().len(), p.faces().len(), p.edges().len(), p.vertices().len()),
                );
                println!("  {}: {} solid(s), has_data={}, bodies={}, faces={}, edges={}, vertices={}",
                    label, solids.len(), s.acis_data.has_data(), b, f, e, v);
            } else {
                println!("  {}: no Solid3D entities found", label);
            }
        }
    }

    println!("\nDone! Open any .dwg file in AutoCAD/IntelliCAD.");
    Ok(())
}

// ════════════════════════════════════════════════════════════════════════════
//  Helpers
// ════════════════════════════════════════════════════════════════════════════

fn write_solid(path: &str, version: DxfVersion, sat: &SatDocument) -> acadrust::Result<()> {
    let mut solid = Solid3D::new();
    solid.set_sat_document(sat);
    solid.common.layer = "0".to_string();

    let mut doc = CadDocument::with_version(version);
    doc.add_entity(EntityType::Solid3D(solid))?;

    DwgWriter::write_to_file(path, &doc)?;
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    println!("  {} ({} bytes, {:?})", path, size, version);
    Ok(())
}

fn print_sat_info(label: &str, sat: &SatDocument) {
    let text = sat.to_sat_string();
    let errors = sat.validate();
    let bodies = sat.bodies().len();
    let faces = sat.faces().len();
    let edges = sat.edges().len();
    let vertices = sat.vertices().len();
    println!("[{}] SAT: {} bytes, {} bodies, {} faces, {} edges, {} vertices, {} warnings",
        label, text.len(), bodies, faces, edges, vertices, errors.len());
    if !errors.is_empty() {
        for e in &errors {
            println!("  WARNING: {:?}", e);
        }
    }
}

/// Build a complete ACIS SAT document for a 10×10×10 axis-aligned box
/// centered at the origin (corners from (-5,-5,-5) to (5,5,5)).
fn build_box_sat() -> SatDocument {
    let mut sat = SatDocument::new_body();

    // The body record sits at index 0.
    let body_idx = SatPointer::new(0);

    // ════════════════════════════════════════════════════════════════
    //  Geometry (surfaces, curves, points)
    // ════════════════════════════════════════════════════════════════

    // 8 corner points
    //   p0(-5,-5,-5)  p1(5,-5,-5)  p2(5,5,-5)  p3(-5,5,-5)
    //   p4(-5,-5, 5)  p5(5,-5, 5)  p6(5,5, 5)  p7(-5,5, 5)
    let p0 = sat.add_point(-5.0, -5.0, -5.0);
    let p1 = sat.add_point( 5.0, -5.0, -5.0);
    let p2 = sat.add_point( 5.0,  5.0, -5.0);
    let p3 = sat.add_point(-5.0,  5.0, -5.0);
    let p4 = sat.add_point(-5.0, -5.0,  5.0);
    let p5 = sat.add_point( 5.0, -5.0,  5.0);
    let p6 = sat.add_point( 5.0,  5.0,  5.0);
    let p7 = sat.add_point(-5.0,  5.0,  5.0);

    // 6 plane surfaces  (origin, normal, u-vector)
    let surf_top    = sat.add_plane_surface([0.0, 0.0,  5.0], [0.0, 0.0,  1.0], [1.0, 0.0, 0.0]);
    let surf_bottom = sat.add_plane_surface([0.0, 0.0, -5.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]);
    let surf_front  = sat.add_plane_surface([0.0, -5.0, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]);
    let surf_back   = sat.add_plane_surface([0.0,  5.0, 0.0], [0.0,  1.0, 0.0], [1.0, 0.0, 0.0]);
    let surf_right  = sat.add_plane_surface([ 5.0, 0.0, 0.0], [ 1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    let surf_left   = sat.add_plane_surface([-5.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);

    // 12 straight curves  (start-point, direction)
    // Bottom face edges (Z = -5)
    let crv_b0 = sat.add_straight_curve([-5.0, -5.0, -5.0], [ 1.0, 0.0, 0.0]); // p0→p1
    let crv_b1 = sat.add_straight_curve([ 5.0, -5.0, -5.0], [ 0.0, 1.0, 0.0]); // p1→p2
    let crv_b2 = sat.add_straight_curve([ 5.0,  5.0, -5.0], [-1.0, 0.0, 0.0]); // p2→p3
    let crv_b3 = sat.add_straight_curve([-5.0,  5.0, -5.0], [ 0.0,-1.0, 0.0]); // p3→p0
    // Top face edges (Z = 5)
    let crv_t0 = sat.add_straight_curve([-5.0, -5.0,  5.0], [ 1.0, 0.0, 0.0]); // p4→p5
    let crv_t1 = sat.add_straight_curve([ 5.0, -5.0,  5.0], [ 0.0, 1.0, 0.0]); // p5→p6
    let crv_t2 = sat.add_straight_curve([ 5.0,  5.0,  5.0], [-1.0, 0.0, 0.0]); // p6→p7
    let crv_t3 = sat.add_straight_curve([-5.0,  5.0,  5.0], [ 0.0,-1.0, 0.0]); // p7→p4
    // Vertical edges
    let crv_v0 = sat.add_straight_curve([-5.0, -5.0, -5.0], [ 0.0, 0.0, 1.0]); // p0→p4
    let crv_v1 = sat.add_straight_curve([ 5.0, -5.0, -5.0], [ 0.0, 0.0, 1.0]); // p1→p5
    let crv_v2 = sat.add_straight_curve([ 5.0,  5.0, -5.0], [ 0.0, 0.0, 1.0]); // p2→p6
    let crv_v3 = sat.add_straight_curve([-5.0,  5.0, -5.0], [ 0.0, 0.0, 1.0]); // p3→p7

    // ════════════════════════════════════════════════════════════════
    //  Topology (vertices, edges, coedges, loops, faces, shell, lump)
    // ════════════════════════════════════════════════════════════════

    // 8 vertices
    let v0 = sat.add_vertex(SatPointer::NULL, SatPointer::new(p0));
    let v1 = sat.add_vertex(SatPointer::NULL, SatPointer::new(p1));
    let v2 = sat.add_vertex(SatPointer::NULL, SatPointer::new(p2));
    let v3 = sat.add_vertex(SatPointer::NULL, SatPointer::new(p3));
    let v4 = sat.add_vertex(SatPointer::NULL, SatPointer::new(p4));
    let v5 = sat.add_vertex(SatPointer::NULL, SatPointer::new(p5));
    let v6 = sat.add_vertex(SatPointer::NULL, SatPointer::new(p6));
    let v7 = sat.add_vertex(SatPointer::NULL, SatPointer::new(p7));

    // 12 edges  (start_vertex, start_t, end_vertex, end_t, coedge=NULL, curve, sense)
    let e0  = sat.add_edge(SatPointer::new(v0), 0.0, SatPointer::new(v1), 10.0, SatPointer::NULL, SatPointer::new(crv_b0), Sense::Forward);
    let e1  = sat.add_edge(SatPointer::new(v1), 0.0, SatPointer::new(v2), 10.0, SatPointer::NULL, SatPointer::new(crv_b1), Sense::Forward);
    let e2  = sat.add_edge(SatPointer::new(v2), 0.0, SatPointer::new(v3), 10.0, SatPointer::NULL, SatPointer::new(crv_b2), Sense::Forward);
    let e3  = sat.add_edge(SatPointer::new(v3), 0.0, SatPointer::new(v0), 10.0, SatPointer::NULL, SatPointer::new(crv_b3), Sense::Forward);
    let e4  = sat.add_edge(SatPointer::new(v4), 0.0, SatPointer::new(v5), 10.0, SatPointer::NULL, SatPointer::new(crv_t0), Sense::Forward);
    let e5  = sat.add_edge(SatPointer::new(v5), 0.0, SatPointer::new(v6), 10.0, SatPointer::NULL, SatPointer::new(crv_t1), Sense::Forward);
    let e6  = sat.add_edge(SatPointer::new(v6), 0.0, SatPointer::new(v7), 10.0, SatPointer::NULL, SatPointer::new(crv_t2), Sense::Forward);
    let e7  = sat.add_edge(SatPointer::new(v7), 0.0, SatPointer::new(v4), 10.0, SatPointer::NULL, SatPointer::new(crv_t3), Sense::Forward);
    let e8  = sat.add_edge(SatPointer::new(v0), 0.0, SatPointer::new(v4), 10.0, SatPointer::NULL, SatPointer::new(crv_v0), Sense::Forward);
    let e9  = sat.add_edge(SatPointer::new(v1), 0.0, SatPointer::new(v5), 10.0, SatPointer::NULL, SatPointer::new(crv_v1), Sense::Forward);
    let e10 = sat.add_edge(SatPointer::new(v2), 0.0, SatPointer::new(v6), 10.0, SatPointer::NULL, SatPointer::new(crv_v2), Sense::Forward);
    let e11 = sat.add_edge(SatPointer::new(v3), 0.0, SatPointer::new(v7), 10.0, SatPointer::NULL, SatPointer::new(crv_v3), Sense::Forward);

    // ── Pre-compute indices for 24 coedges + 6 loops + 6 faces + shell + lump
    let base = sat.records.len() as i32;
    let co_base   = base;         // 24 coedges: base+0..23
    let loop_base = base + 24;    // 6 loops:   base+24..29
    let face_base = base + 30;    // 6 faces:   base+30..35
    let shell_idx = base + 36;
    let lump_idx  = base + 37;

    let ptr = |i: i32| SatPointer::new(i);

    // Coedge aliases for partner references
    let co = |i: i32| co_base + i;

    // ── Bottom face (Z = -5, normal outward = -Z) ───────────────────
    //    Loop: e0(rev) → e3(rev) → e2(rev) → e1(rev)
    sat.add_coedge(ptr(co(1)),  ptr(co(3)),  ptr(co(8)),  ptr(e0), Sense::Reversed, ptr(loop_base));     // co0, partner=front co8
    sat.add_coedge(ptr(co(2)),  ptr(co(0)),  ptr(co(20)), ptr(e3), Sense::Reversed, ptr(loop_base));     // co1, partner=left co20
    sat.add_coedge(ptr(co(3)),  ptr(co(1)),  ptr(co(12)), ptr(e2), Sense::Reversed, ptr(loop_base));     // co2, partner=back co12
    sat.add_coedge(ptr(co(0)),  ptr(co(2)),  ptr(co(16)), ptr(e1), Sense::Reversed, ptr(loop_base));     // co3, partner=right co16

    // ── Top face (Z = +5, normal outward = +Z) ─────────────────────
    //    Loop: e4(fwd) → e5(fwd) → e6(fwd) → e7(fwd)
    sat.add_coedge(ptr(co(5)),  ptr(co(7)),  ptr(co(10)), ptr(e4), Sense::Forward, ptr(loop_base + 1));  // co4, partner=front co10
    sat.add_coedge(ptr(co(6)),  ptr(co(4)),  ptr(co(18)), ptr(e5), Sense::Forward, ptr(loop_base + 1));  // co5, partner=right co18
    sat.add_coedge(ptr(co(7)),  ptr(co(5)),  ptr(co(14)), ptr(e6), Sense::Forward, ptr(loop_base + 1));  // co6, partner=back co14
    sat.add_coedge(ptr(co(4)),  ptr(co(6)),  ptr(co(22)), ptr(e7), Sense::Forward, ptr(loop_base + 1));  // co7, partner=left co22

    // ── Front face (Y = -5, normal outward = -Y) ───────────────────
    //    Loop: e0(fwd) → e9(fwd) → e4(rev) → e8(rev)
    sat.add_coedge(ptr(co(9)),  ptr(co(11)), ptr(co(0)),  ptr(e0), Sense::Forward,  ptr(loop_base + 2)); // co8, partner=bottom co0
    sat.add_coedge(ptr(co(10)), ptr(co(8)),  ptr(co(19)), ptr(e9), Sense::Forward,  ptr(loop_base + 2)); // co9, partner=right co19
    sat.add_coedge(ptr(co(11)), ptr(co(9)),  ptr(co(4)),  ptr(e4), Sense::Reversed, ptr(loop_base + 2)); // co10, partner=top co4
    sat.add_coedge(ptr(co(8)),  ptr(co(10)), ptr(co(21)), ptr(e8), Sense::Reversed, ptr(loop_base + 2)); // co11, partner=left co21

    // ── Back face (Y = +5, normal outward = +Y) ────────────────────
    //    Loop: e2(fwd) → e11(fwd) → e6(rev) → e10(rev)
    sat.add_coedge(ptr(co(13)), ptr(co(15)), ptr(co(2)),  ptr(e2),  Sense::Forward,  ptr(loop_base + 3)); // co12, partner=bottom co2
    sat.add_coedge(ptr(co(14)), ptr(co(12)), ptr(co(23)), ptr(e11), Sense::Forward,  ptr(loop_base + 3)); // co13, partner=left co23
    sat.add_coedge(ptr(co(15)), ptr(co(13)), ptr(co(6)),  ptr(e6),  Sense::Reversed, ptr(loop_base + 3)); // co14, partner=top co6
    sat.add_coedge(ptr(co(12)), ptr(co(14)), ptr(co(17)), ptr(e10), Sense::Reversed, ptr(loop_base + 3)); // co15, partner=right co17

    // ── Right face (X = +5, normal outward = +X) ───────────────────
    //    Loop: e1(fwd) → e10(fwd) → e5(rev) → e9(rev)
    sat.add_coedge(ptr(co(17)), ptr(co(19)), ptr(co(3)),  ptr(e1),  Sense::Forward,  ptr(loop_base + 4)); // co16, partner=bottom co3
    sat.add_coedge(ptr(co(18)), ptr(co(16)), ptr(co(15)), ptr(e10), Sense::Forward,  ptr(loop_base + 4)); // co17, partner=back co15
    sat.add_coedge(ptr(co(19)), ptr(co(17)), ptr(co(5)),  ptr(e5),  Sense::Reversed, ptr(loop_base + 4)); // co18, partner=top co5
    sat.add_coedge(ptr(co(16)), ptr(co(18)), ptr(co(9)),  ptr(e9),  Sense::Reversed, ptr(loop_base + 4)); // co19, partner=front co9

    // ── Left face (X = -5, normal outward = -X) ────────────────────
    //    Loop: e3(fwd) → e8(fwd) → e7(rev) → e11(rev)
    sat.add_coedge(ptr(co(21)), ptr(co(23)), ptr(co(1)),  ptr(e3),  Sense::Forward,  ptr(loop_base + 5)); // co20, partner=bottom co1
    sat.add_coedge(ptr(co(22)), ptr(co(20)), ptr(co(11)), ptr(e8),  Sense::Forward,  ptr(loop_base + 5)); // co21, partner=front co11
    sat.add_coedge(ptr(co(23)), ptr(co(21)), ptr(co(7)),  ptr(e7),  Sense::Reversed, ptr(loop_base + 5)); // co22, partner=top co7
    sat.add_coedge(ptr(co(20)), ptr(co(22)), ptr(co(13)), ptr(e11), Sense::Reversed, ptr(loop_base + 5)); // co23, partner=back co13

    // ── 6 Loops ─────────────────────────────────────────────────────
    sat.add_loop(SatPointer::NULL, ptr(co(0)),  ptr(face_base));
    sat.add_loop(SatPointer::NULL, ptr(co(4)),  ptr(face_base + 1));
    sat.add_loop(SatPointer::NULL, ptr(co(8)),  ptr(face_base + 2));
    sat.add_loop(SatPointer::NULL, ptr(co(12)), ptr(face_base + 3));
    sat.add_loop(SatPointer::NULL, ptr(co(16)), ptr(face_base + 4));
    sat.add_loop(SatPointer::NULL, ptr(co(20)), ptr(face_base + 5));

    // ── 6 Faces (linked list via next_face) ─────────────────────────
    sat.add_face(ptr(face_base + 1), ptr(loop_base),     ptr(shell_idx), ptr(surf_bottom), Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 2), ptr(loop_base + 1), ptr(shell_idx), ptr(surf_top),    Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 3), ptr(loop_base + 2), ptr(shell_idx), ptr(surf_front),  Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 4), ptr(loop_base + 3), ptr(shell_idx), ptr(surf_back),   Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 5), ptr(loop_base + 4), ptr(shell_idx), ptr(surf_right),  Sense::Forward, Sidedness::Single);
    sat.add_face(SatPointer::NULL,   ptr(loop_base + 5), ptr(shell_idx), ptr(surf_left),   Sense::Forward, Sidedness::Single);

    // ── Shell → Lump → Body ─────────────────────────────────────────
    sat.add_shell(ptr(face_base), ptr(lump_idx));
    sat.add_lump(ptr(shell_idx), body_idx);

    // Patch the body record to point to the lump
    if let Some(body_rec) = sat.record_mut(0) {
        body_rec.tokens[1] = SatToken::Pointer(ptr(lump_idx));
    }

    sat
}

// ════════════════════════════════════════════════════════════════════════════
//  Wedge — Right Triangular Prism
// ════════════════════════════════════════════════════════════════════════════

/// Build a right triangular prism (wedge).
///
/// Bottom triangle: A(0,0,0), B(10,0,0), C(0,10,0)
/// Top triangle:    D(0,0,10), E(10,0,10), F(0,10,10)
///
/// 5 faces, 9 edges, 6 vertices.
fn build_wedge_sat() -> SatDocument {
    let mut sat = SatDocument::new_body();
    let body_idx = SatPointer::new(0);
    let ptr = |i: i32| SatPointer::new(i);

    let s = std::f64::consts::FRAC_1_SQRT_2; // 1/√2

    // ── Points (6) ──────────────────────────────────────────────────
    let p_a = sat.add_point(0.0,  0.0,  0.0);  // A
    let p_b = sat.add_point(10.0, 0.0,  0.0);  // B
    let p_c = sat.add_point(0.0,  10.0, 0.0);  // C
    let p_d = sat.add_point(0.0,  0.0,  10.0); // D
    let p_e = sat.add_point(10.0, 0.0,  10.0); // E
    let p_f = sat.add_point(0.0,  10.0, 10.0); // F

    // ── Surfaces (5) ────────────────────────────────────────────────
    let surf_bot  = sat.add_plane_surface([0.0, 0.0, 0.0],  [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]);
    let surf_top  = sat.add_plane_surface([0.0, 0.0, 10.0], [0.0, 0.0,  1.0], [1.0, 0.0, 0.0]);
    let surf_frt  = sat.add_plane_surface([0.0, 0.0, 0.0],  [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]); // y=0
    let surf_lft  = sat.add_plane_surface([0.0, 0.0, 0.0],  [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]); // x=0
    let surf_hyp  = sat.add_plane_surface([5.0, 5.0, 0.0],  [s,    s,   0.0], [0.0, 0.0, 1.0]); // x+y=10

    // ── Curves (9 straight) ─────────────────────────────────────────
    let crv0 = sat.add_straight_curve([0.0, 0.0, 0.0],   [1.0, 0.0, 0.0]);   // A→B
    let crv1 = sat.add_straight_curve([10.0, 0.0, 0.0],  [-s,   s,  0.0]);    // B→C
    let crv2 = sat.add_straight_curve([0.0, 10.0, 0.0],  [0.0, -1.0, 0.0]);   // C→A
    let crv3 = sat.add_straight_curve([0.0, 0.0, 10.0],  [1.0, 0.0, 0.0]);    // D→E
    let crv4 = sat.add_straight_curve([10.0, 0.0, 10.0], [-s,   s,  0.0]);    // E→F
    let crv5 = sat.add_straight_curve([0.0, 10.0, 10.0], [0.0, -1.0, 0.0]);   // F→D
    let crv6 = sat.add_straight_curve([0.0, 0.0, 0.0],   [0.0, 0.0, 1.0]);    // A→D
    let crv7 = sat.add_straight_curve([10.0, 0.0, 0.0],  [0.0, 0.0, 1.0]);    // B→E
    let crv8 = sat.add_straight_curve([0.0, 10.0, 0.0],  [0.0, 0.0, 1.0]);    // C→F

    let hyp_len = 10.0 * 2.0_f64.sqrt(); // length of hypotenuse edge

    // ── Vertices (6) ────────────────────────────────────────────────
    let v_a = sat.add_vertex(SatPointer::NULL, ptr(p_a));
    let v_b = sat.add_vertex(SatPointer::NULL, ptr(p_b));
    let v_c = sat.add_vertex(SatPointer::NULL, ptr(p_c));
    let v_d = sat.add_vertex(SatPointer::NULL, ptr(p_d));
    let v_e = sat.add_vertex(SatPointer::NULL, ptr(p_e));
    let v_f = sat.add_vertex(SatPointer::NULL, ptr(p_f));

    // ── Edges (9) ───────────────────────────────────────────────────
    let e0 = sat.add_edge(ptr(v_a), 0.0, ptr(v_b), 10.0,    SatPointer::NULL, ptr(crv0), Sense::Forward);
    let e1 = sat.add_edge(ptr(v_b), 0.0, ptr(v_c), hyp_len, SatPointer::NULL, ptr(crv1), Sense::Forward);
    let e2 = sat.add_edge(ptr(v_c), 0.0, ptr(v_a), 10.0,    SatPointer::NULL, ptr(crv2), Sense::Forward);
    let e3 = sat.add_edge(ptr(v_d), 0.0, ptr(v_e), 10.0,    SatPointer::NULL, ptr(crv3), Sense::Forward);
    let e4 = sat.add_edge(ptr(v_e), 0.0, ptr(v_f), hyp_len, SatPointer::NULL, ptr(crv4), Sense::Forward);
    let e5 = sat.add_edge(ptr(v_f), 0.0, ptr(v_d), 10.0,    SatPointer::NULL, ptr(crv5), Sense::Forward);
    let e6 = sat.add_edge(ptr(v_a), 0.0, ptr(v_d), 10.0,    SatPointer::NULL, ptr(crv6), Sense::Forward);
    let e7 = sat.add_edge(ptr(v_b), 0.0, ptr(v_e), 10.0,    SatPointer::NULL, ptr(crv7), Sense::Forward);
    let e8 = sat.add_edge(ptr(v_c), 0.0, ptr(v_f), 10.0,    SatPointer::NULL, ptr(crv8), Sense::Forward);

    // ── Coedge pre-computed indices ─────────────────────────────────
    let base = sat.records.len() as i32;
    let co = |i: i32| base + i;        // 18 coedges: base+0..17
    let loop_base = base + 18;          // 5 loops:   base+18..22
    let face_base = base + 23;          // 5 faces:   base+23..27
    let shell_idx = base + 28;
    let lump_idx  = base + 29;

    // ── Bottom face (z=0, normal -Z): A→C→B  (CW from above) ───────
    sat.add_coedge(ptr(co(1)),  ptr(co(2)),  ptr(co(13)), ptr(e2), Sense::Reversed, ptr(loop_base));     // co0  e2 rev = A→C
    sat.add_coedge(ptr(co(2)),  ptr(co(0)),  ptr(co(14)), ptr(e1), Sense::Reversed, ptr(loop_base));     // co1  e1 rev = C→B
    sat.add_coedge(ptr(co(0)),  ptr(co(1)),  ptr(co(6)),  ptr(e0), Sense::Reversed, ptr(loop_base));     // co2  e0 rev = B→A

    // ── Top face (z=10, normal +Z): D→E→F  (CCW from above) ────────
    sat.add_coedge(ptr(co(4)),  ptr(co(5)),  ptr(co(8)),  ptr(e3), Sense::Forward, ptr(loop_base + 1));  // co3  e3 fwd = D→E
    sat.add_coedge(ptr(co(5)),  ptr(co(3)),  ptr(co(16)), ptr(e4), Sense::Forward, ptr(loop_base + 1));  // co4  e4 fwd = E→F
    sat.add_coedge(ptr(co(3)),  ptr(co(4)),  ptr(co(11)), ptr(e5), Sense::Forward, ptr(loop_base + 1));  // co5  e5 fwd = F→D

    // ── Front face (y=0, normal -Y): A→B→E→D ───────────────────────
    sat.add_coedge(ptr(co(7)),  ptr(co(9)),  ptr(co(2)),  ptr(e0), Sense::Forward,  ptr(loop_base + 2)); // co6
    sat.add_coedge(ptr(co(8)),  ptr(co(6)),  ptr(co(17)), ptr(e7), Sense::Forward,  ptr(loop_base + 2)); // co7
    sat.add_coedge(ptr(co(9)),  ptr(co(7)),  ptr(co(3)),  ptr(e3), Sense::Reversed, ptr(loop_base + 2)); // co8
    sat.add_coedge(ptr(co(6)),  ptr(co(8)),  ptr(co(10)), ptr(e6), Sense::Reversed, ptr(loop_base + 2)); // co9

    // ── Left face (x=0, normal -X): A→D→F→C ────────────────────────
    sat.add_coedge(ptr(co(11)), ptr(co(13)), ptr(co(9)),  ptr(e6), Sense::Forward,  ptr(loop_base + 3)); // co10
    sat.add_coedge(ptr(co(12)), ptr(co(10)), ptr(co(5)),  ptr(e5), Sense::Reversed, ptr(loop_base + 3)); // co11
    sat.add_coedge(ptr(co(13)), ptr(co(11)), ptr(co(15)), ptr(e8), Sense::Reversed, ptr(loop_base + 3)); // co12
    sat.add_coedge(ptr(co(10)), ptr(co(12)), ptr(co(0)),  ptr(e2), Sense::Forward,  ptr(loop_base + 3)); // co13

    // ── Hypotenuse face (x+y=10, normal (s,s,0)): B→C→F→E ─────────
    sat.add_coedge(ptr(co(15)), ptr(co(17)), ptr(co(1)),  ptr(e1), Sense::Forward,  ptr(loop_base + 4)); // co14
    sat.add_coedge(ptr(co(16)), ptr(co(14)), ptr(co(12)), ptr(e8), Sense::Forward,  ptr(loop_base + 4)); // co15
    sat.add_coedge(ptr(co(17)), ptr(co(15)), ptr(co(4)),  ptr(e4), Sense::Reversed, ptr(loop_base + 4)); // co16
    sat.add_coedge(ptr(co(14)), ptr(co(16)), ptr(co(7)),  ptr(e7), Sense::Reversed, ptr(loop_base + 4)); // co17

    // ── Loops (5) ───────────────────────────────────────────────────
    sat.add_loop(SatPointer::NULL, ptr(co(0)),  ptr(face_base));
    sat.add_loop(SatPointer::NULL, ptr(co(3)),  ptr(face_base + 1));
    sat.add_loop(SatPointer::NULL, ptr(co(6)),  ptr(face_base + 2));
    sat.add_loop(SatPointer::NULL, ptr(co(10)), ptr(face_base + 3));
    sat.add_loop(SatPointer::NULL, ptr(co(14)), ptr(face_base + 4));

    // ── Faces (5, linked list) ──────────────────────────────────────
    sat.add_face(ptr(face_base + 1), ptr(loop_base),     ptr(shell_idx), ptr(surf_bot), Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 2), ptr(loop_base + 1), ptr(shell_idx), ptr(surf_top), Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 3), ptr(loop_base + 2), ptr(shell_idx), ptr(surf_frt), Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 4), ptr(loop_base + 3), ptr(shell_idx), ptr(surf_lft), Sense::Forward, Sidedness::Single);
    sat.add_face(SatPointer::NULL,   ptr(loop_base + 4), ptr(shell_idx), ptr(surf_hyp), Sense::Forward, Sidedness::Single);

    // ── Shell → Lump → Body ─────────────────────────────────────────
    sat.add_shell(ptr(face_base), ptr(lump_idx));
    sat.add_lump(ptr(shell_idx), body_idx);

    if let Some(body_rec) = sat.record_mut(0) {
        body_rec.tokens[1] = SatToken::Pointer(ptr(lump_idx));
    }

    sat
}

// ════════════════════════════════════════════════════════════════════════════
//  Pyramid — Square Base with Apex
// ════════════════════════════════════════════════════════════════════════════

/// Build a pyramid with a square base and a single apex.
///
/// Base: A(-5,-5,0), B(5,-5,0), C(5,5,0), D(-5,5,0)
/// Apex: E(0,0,10)
///
/// 5 faces, 8 edges, 5 vertices.
fn build_pyramid_sat() -> SatDocument {
    let mut sat = SatDocument::new_body();
    let body_idx = SatPointer::new(0);
    let ptr = |i: i32| SatPointer::new(i);

    let s5 = 5.0_f64.sqrt();
    let n1 = 2.0 / s5; // 2/√5 for lateral face normals
    let n2 = 1.0 / s5; // 1/√5

    let s6 = 6.0_f64.sqrt();
    let lat_len = 5.0 * s6; // lateral edge length = √(25+25+100) = 5√6

    // ── Points (5) ──────────────────────────────────────────────────
    let p_a = sat.add_point(-5.0, -5.0, 0.0);
    let p_b = sat.add_point( 5.0, -5.0, 0.0);
    let p_c = sat.add_point( 5.0,  5.0, 0.0);
    let p_d = sat.add_point(-5.0,  5.0, 0.0);
    let p_e = sat.add_point( 0.0,  0.0, 10.0);

    // ── Surfaces (5) ────────────────────────────────────────────────
    let surf_base  = sat.add_plane_surface([0.0, 0.0, 0.0], [0.0,  0.0, -1.0], [1.0, 0.0, 0.0]);
    let surf_front = sat.add_plane_surface([0.0, -5.0, 0.0], [0.0, -n1,  n2], [1.0, 0.0, 0.0]);
    let surf_right = sat.add_plane_surface([5.0,  0.0, 0.0], [n1,   0.0, n2], [0.0, 1.0, 0.0]);
    let surf_back  = sat.add_plane_surface([0.0,  5.0, 0.0], [0.0,  n1,  n2], [1.0, 0.0, 0.0]);
    let surf_left  = sat.add_plane_surface([-5.0, 0.0, 0.0], [-n1,  0.0, n2], [0.0, 1.0, 0.0]);

    // ── Curves (8 straight) ─────────────────────────────────────────
    // Base edges
    let crv0 = sat.add_straight_curve([-5.0, -5.0, 0.0], [ 1.0, 0.0, 0.0]);   // A→B
    let crv1 = sat.add_straight_curve([ 5.0, -5.0, 0.0], [ 0.0, 1.0, 0.0]);   // B→C
    let crv2 = sat.add_straight_curve([ 5.0,  5.0, 0.0], [-1.0, 0.0, 0.0]);   // C→D
    let crv3 = sat.add_straight_curve([-5.0,  5.0, 0.0], [ 0.0,-1.0, 0.0]);   // D→A
    // Lateral edges (directions normalized)
    let crv4 = sat.add_straight_curve([-5.0, -5.0, 0.0], [ 1.0/s6,  1.0/s6, 2.0/s6]); // A→E
    let crv5 = sat.add_straight_curve([ 5.0, -5.0, 0.0], [-1.0/s6,  1.0/s6, 2.0/s6]); // B→E
    let crv6 = sat.add_straight_curve([ 5.0,  5.0, 0.0], [-1.0/s6, -1.0/s6, 2.0/s6]); // C→E
    let crv7 = sat.add_straight_curve([-5.0,  5.0, 0.0], [ 1.0/s6, -1.0/s6, 2.0/s6]); // D→E

    // ── Vertices (5) ────────────────────────────────────────────────
    let v_a = sat.add_vertex(SatPointer::NULL, ptr(p_a));
    let v_b = sat.add_vertex(SatPointer::NULL, ptr(p_b));
    let v_c = sat.add_vertex(SatPointer::NULL, ptr(p_c));
    let v_d = sat.add_vertex(SatPointer::NULL, ptr(p_d));
    let v_e = sat.add_vertex(SatPointer::NULL, ptr(p_e));

    // ── Edges (8) ───────────────────────────────────────────────────
    let e0 = sat.add_edge(ptr(v_a), 0.0, ptr(v_b), 10.0,    SatPointer::NULL, ptr(crv0), Sense::Forward);
    let e1 = sat.add_edge(ptr(v_b), 0.0, ptr(v_c), 10.0,    SatPointer::NULL, ptr(crv1), Sense::Forward);
    let e2 = sat.add_edge(ptr(v_c), 0.0, ptr(v_d), 10.0,    SatPointer::NULL, ptr(crv2), Sense::Forward);
    let e3 = sat.add_edge(ptr(v_d), 0.0, ptr(v_a), 10.0,    SatPointer::NULL, ptr(crv3), Sense::Forward);
    let e4 = sat.add_edge(ptr(v_a), 0.0, ptr(v_e), lat_len, SatPointer::NULL, ptr(crv4), Sense::Forward);
    let e5 = sat.add_edge(ptr(v_b), 0.0, ptr(v_e), lat_len, SatPointer::NULL, ptr(crv5), Sense::Forward);
    let e6 = sat.add_edge(ptr(v_c), 0.0, ptr(v_e), lat_len, SatPointer::NULL, ptr(crv6), Sense::Forward);
    let e7 = sat.add_edge(ptr(v_d), 0.0, ptr(v_e), lat_len, SatPointer::NULL, ptr(crv7), Sense::Forward);

    // ── Coedge pre-computed indices ─────────────────────────────────
    let base = sat.records.len() as i32;
    let co = |i: i32| base + i;        // 16 coedges: base+0..15
    let loop_base = base + 16;          // 5 loops: base+16..20
    let face_base = base + 21;          // 5 faces: base+21..25
    let shell_idx = base + 26;
    let lump_idx  = base + 27;

    // ── Base face (z=0, normal -Z): A→D→C→B  (CW from above) ──────
    sat.add_coedge(ptr(co(1)),  ptr(co(3)),  ptr(co(15)), ptr(e3), Sense::Reversed, ptr(loop_base));     // co0
    sat.add_coedge(ptr(co(2)),  ptr(co(0)),  ptr(co(10)), ptr(e2), Sense::Reversed, ptr(loop_base));     // co1
    sat.add_coedge(ptr(co(3)),  ptr(co(1)),  ptr(co(7)),  ptr(e1), Sense::Reversed, ptr(loop_base));     // co2
    sat.add_coedge(ptr(co(0)),  ptr(co(2)),  ptr(co(4)),  ptr(e0), Sense::Reversed, ptr(loop_base));     // co3

    // ── Front face (A→B→E): normal (0, -2/√5, 1/√5) ────────────────
    sat.add_coedge(ptr(co(5)),  ptr(co(6)),  ptr(co(3)),  ptr(e0), Sense::Forward,  ptr(loop_base + 1)); // co4
    sat.add_coedge(ptr(co(6)),  ptr(co(4)),  ptr(co(9)),  ptr(e5), Sense::Forward,  ptr(loop_base + 1)); // co5
    sat.add_coedge(ptr(co(4)),  ptr(co(5)),  ptr(co(14)), ptr(e4), Sense::Reversed, ptr(loop_base + 1)); // co6

    // ── Right face (B→C→E): normal (2/√5, 0, 1/√5) ─────────────────
    sat.add_coedge(ptr(co(8)),  ptr(co(9)),  ptr(co(2)),  ptr(e1), Sense::Forward,  ptr(loop_base + 2)); // co7
    sat.add_coedge(ptr(co(9)),  ptr(co(7)),  ptr(co(12)), ptr(e6), Sense::Forward,  ptr(loop_base + 2)); // co8
    sat.add_coedge(ptr(co(7)),  ptr(co(8)),  ptr(co(5)),  ptr(e5), Sense::Reversed, ptr(loop_base + 2)); // co9

    // ── Back face (C→D→E): normal (0, 2/√5, 1/√5) ──────────────────
    sat.add_coedge(ptr(co(11)), ptr(co(12)), ptr(co(1)),  ptr(e2), Sense::Forward,  ptr(loop_base + 3)); // co10
    sat.add_coedge(ptr(co(12)), ptr(co(10)), ptr(co(15)), ptr(e7), Sense::Forward,  ptr(loop_base + 3)); // co11
    sat.add_coedge(ptr(co(10)), ptr(co(11)), ptr(co(8)),  ptr(e6), Sense::Reversed, ptr(loop_base + 3)); // co12

    // ── Left face (D→A→E): normal (-2/√5, 0, 1/√5) ─────────────────
    sat.add_coedge(ptr(co(14)), ptr(co(15)), ptr(co(0)),  ptr(e3), Sense::Forward,  ptr(loop_base + 4)); // co13
    sat.add_coedge(ptr(co(15)), ptr(co(13)), ptr(co(6)),  ptr(e4), Sense::Forward,  ptr(loop_base + 4)); // co14
    sat.add_coedge(ptr(co(13)), ptr(co(14)), ptr(co(11)), ptr(e7), Sense::Reversed, ptr(loop_base + 4)); // co15

    // ── Loops (5) ───────────────────────────────────────────────────
    sat.add_loop(SatPointer::NULL, ptr(co(0)),  ptr(face_base));      // base
    sat.add_loop(SatPointer::NULL, ptr(co(4)),  ptr(face_base + 1));  // front
    sat.add_loop(SatPointer::NULL, ptr(co(7)),  ptr(face_base + 2));  // right
    sat.add_loop(SatPointer::NULL, ptr(co(10)), ptr(face_base + 3));  // back
    sat.add_loop(SatPointer::NULL, ptr(co(13)), ptr(face_base + 4));  // left

    // ── Faces (5, linked list) ──────────────────────────────────────
    sat.add_face(ptr(face_base + 1), ptr(loop_base),     ptr(shell_idx), ptr(surf_base),  Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 2), ptr(loop_base + 1), ptr(shell_idx), ptr(surf_front), Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 3), ptr(loop_base + 2), ptr(shell_idx), ptr(surf_right), Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 4), ptr(loop_base + 3), ptr(shell_idx), ptr(surf_back),  Sense::Forward, Sidedness::Single);
    sat.add_face(SatPointer::NULL,   ptr(loop_base + 4), ptr(shell_idx), ptr(surf_left),  Sense::Forward, Sidedness::Single);

    // ── Shell → Lump → Body ─────────────────────────────────────────
    sat.add_shell(ptr(face_base), ptr(lump_idx));
    sat.add_lump(ptr(shell_idx), body_idx);

    if let Some(body_rec) = sat.record_mut(0) {
        body_rec.tokens[1] = SatToken::Pointer(ptr(lump_idx));
    }

    sat
}

// ════════════════════════════════════════════════════════════════════════════
//  Cylinder
// ════════════════════════════════════════════════════════════════════════════

/// Build a cylinder with radius 5 and height 10 along the Z-axis.
///
/// Bottom circle center at (0,0,0), top at (0,0,10).
/// Uses cone-surface (degenerate cone = cylinder) and ellipse-curve (circle).
///
/// 3 faces, 3 edges, 2 vertices, seam edge along x=5, y=0.
fn build_cylinder_sat() -> SatDocument {
    let mut sat = SatDocument::new_body();
    let body_idx = SatPointer::new(0);
    let ptr = |i: i32| SatPointer::new(i);

    let tau = std::f64::consts::TAU; // 2π

    // ── Points (2) — seam points where circles meet the seam edge ──
    let p0 = sat.add_point(5.0, 0.0, 0.0);   // bottom seam
    let p1 = sat.add_point(5.0, 0.0, 10.0);  // top seam

    // ── Surfaces (3) ────────────────────────────────────────────────
    let surf_bot = sat.add_plane_surface([0.0, 0.0, 0.0],  [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]);
    let surf_top = sat.add_plane_surface([0.0, 0.0, 10.0], [0.0, 0.0,  1.0], [1.0, 0.0, 0.0]);
    let surf_cyl = sat.add_cone_surface(
        [0.0, 0.0, 0.0],  // center (on axis)
        [0.0, 0.0, 1.0],  // axis direction
        [5.0, 0.0, 0.0],  // major-axis (direction + radius = 5)
        1.0,               // ratio (circular)
        1.0,               // cos(half-angle) = 1 → cylinder
        0.0,               // sin(half-angle) = 0 → cylinder
    );

    // ── Curves (3) ──────────────────────────────────────────────────
    let crv_bot  = sat.add_ellipse_curve([0.0, 0.0, 0.0],  [0.0, 0.0, 1.0], [5.0, 0.0, 0.0], 1.0);
    let crv_top  = sat.add_ellipse_curve([0.0, 0.0, 10.0], [0.0, 0.0, 1.0], [5.0, 0.0, 0.0], 1.0);
    let crv_seam = sat.add_straight_curve([5.0, 0.0, 0.0],  [0.0, 0.0, 1.0]);

    // ── Vertices (2) ────────────────────────────────────────────────
    let v0 = sat.add_vertex(SatPointer::NULL, ptr(p0)); // bottom seam
    let v1 = sat.add_vertex(SatPointer::NULL, ptr(p1)); // top seam

    // ── Edges (3) ───────────────────────────────────────────────────
    // Circles are closed: start/end vertex are the same, params 0→2π
    let e_bot  = sat.add_edge(ptr(v0), 0.0, ptr(v0), tau, SatPointer::NULL, ptr(crv_bot),  Sense::Forward);
    let e_top  = sat.add_edge(ptr(v1), 0.0, ptr(v1), tau, SatPointer::NULL, ptr(crv_top),  Sense::Forward);
    let e_seam = sat.add_edge(ptr(v0), 0.0, ptr(v1), 10.0, SatPointer::NULL, ptr(crv_seam), Sense::Forward);

    // ── Coedge pre-computed indices ─────────────────────────────────
    let base = sat.records.len() as i32;
    let co = |i: i32| base + i;   // 6 coedges: base+0..5
    let loop_base = base + 6;      // 3 loops: base+6..8
    let face_base = base + 9;      // 3 faces: base+9..11
    let shell_idx = base + 12;
    let lump_idx  = base + 13;

    // ── Bottom cap: single coedge (circle reversed = CW from above) ─
    sat.add_coedge(ptr(co(0)), ptr(co(0)), ptr(co(4)), ptr(e_bot), Sense::Reversed, ptr(loop_base));     // co0

    // ── Top cap: single coedge (circle forward = CCW from above) ────
    sat.add_coedge(ptr(co(1)), ptr(co(1)), ptr(co(2)), ptr(e_top), Sense::Forward, ptr(loop_base + 1));  // co1

    // ── Lateral face: 4 coedges around the cylinder surface ─────────
    //    Cycle via next: co2→co5→co4→co3→co2
    //    Vertex flow: v1(co2)→v1→v0(co5)→v0(co4)→v0→v1(co3)→v1(co2) ✓
    //    Winding at seam: -Y,-Z,+Y,+Z = CCW from radially outward ✓
    sat.add_coedge(ptr(co(5)), ptr(co(3)), ptr(co(1)),  ptr(e_top),  Sense::Reversed, ptr(loop_base + 2)); // co2 top rev
    sat.add_coedge(ptr(co(2)), ptr(co(4)), ptr(co(5)),  ptr(e_seam), Sense::Forward,  ptr(loop_base + 2)); // co3 seam up (v0→v1)
    sat.add_coedge(ptr(co(3)), ptr(co(5)), ptr(co(0)),  ptr(e_bot),  Sense::Forward,  ptr(loop_base + 2)); // co4 bot fwd
    sat.add_coedge(ptr(co(4)), ptr(co(2)), ptr(co(3)),  ptr(e_seam), Sense::Reversed, ptr(loop_base + 2)); // co5 seam down (v1→v0)

    // ── Loops (3) ───────────────────────────────────────────────────
    sat.add_loop(SatPointer::NULL, ptr(co(0)), ptr(face_base));       // bottom cap
    sat.add_loop(SatPointer::NULL, ptr(co(1)), ptr(face_base + 1));   // top cap
    sat.add_loop(SatPointer::NULL, ptr(co(2)), ptr(face_base + 2));   // lateral

    // ── Faces (3, linked list) ──────────────────────────────────────
    sat.add_face(ptr(face_base + 1), ptr(loop_base),     ptr(shell_idx), ptr(surf_bot), Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 2), ptr(loop_base + 1), ptr(shell_idx), ptr(surf_top), Sense::Forward, Sidedness::Single);
    sat.add_face(SatPointer::NULL,   ptr(loop_base + 2), ptr(shell_idx), ptr(surf_cyl), Sense::Forward, Sidedness::Single);

    // ── Shell → Lump → Body ─────────────────────────────────────────
    sat.add_shell(ptr(face_base), ptr(lump_idx));
    sat.add_lump(ptr(shell_idx), body_idx);

    if let Some(body_rec) = sat.record_mut(0) {
        body_rec.tokens[1] = SatToken::Pointer(ptr(lump_idx));
    }

    sat
}

// ════════════════════════════════════════════════════════════════════════════
//  Cone — base radius 5, apex at height 10
// ════════════════════════════════════════════════════════════════════════════

/// Build a cone with circular base (radius 5 at z=0) and apex at (0,0,10).
///
/// 2 faces, 1 edge (base circle), 1 vertex (seam on base circle).
fn build_cone_sat() -> SatDocument {
    let mut sat = SatDocument::new_body();
    let body_idx = SatPointer::new(0);
    let ptr = |i: i32| SatPointer::new(i);

    let tau = std::f64::consts::TAU;

    // Half-angle: tan = R/H = 5/10 = 0.5
    let hyp = (5.0_f64 * 5.0 + 10.0 * 10.0).sqrt(); // √125
    let sin_half = -5.0 / hyp;  // negative → tapers in +Z direction
    let cos_half = 10.0 / hyp;

    // ── Point (1) — seam point on base circle ──────────────────────
    let p0 = sat.add_point(5.0, 0.0, 0.0);

    // ── Surfaces (2) ────────────────────────────────────────────────
    let surf_base = sat.add_plane_surface([0.0, 0.0, 0.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]);
    let surf_cone = sat.add_cone_surface(
        [0.0, 0.0, 0.0],   // center (on axis, at base level)
        [0.0, 0.0, 1.0],   // axis direction (toward apex)
        [5.0, 0.0, 0.0],   // major_axis (radius = 5 at center)
        1.0,                // ratio (circular)
        cos_half,           // cos(half-angle)
        sin_half,           // sin(half-angle)
    );

    // ── Curve (1) — base circle ─────────────────────────────────────
    let crv_base = sat.add_ellipse_curve(
        [0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [5.0, 0.0, 0.0], 1.0,
    );

    // ── Vertex (1) ──────────────────────────────────────────────────
    let v0 = sat.add_vertex(SatPointer::NULL, ptr(p0));

    // ── Edge (1) — closed circle ────────────────────────────────────
    let e0 = sat.add_edge(ptr(v0), 0.0, ptr(v0), tau, SatPointer::NULL, ptr(crv_base), Sense::Forward);

    // ── Coedge pre-computed indices ─────────────────────────────────
    let base = sat.records.len() as i32;
    let co = |i: i32| base + i;   // 2 coedges: base+0..1
    let loop_base = base + 2;      // 2 loops: base+2..3
    let face_base = base + 4;      // 2 faces: base+4..5
    let shell_idx = base + 6;
    let lump_idx  = base + 7;

    // ── Base cap: single coedge (circle reversed = CW from below) ───
    sat.add_coedge(ptr(co(0)), ptr(co(0)), ptr(co(1)), ptr(e0), Sense::Reversed, ptr(loop_base));     // co0

    // ── Cone lateral: single coedge (circle forward) ────────────────
    sat.add_coedge(ptr(co(1)), ptr(co(1)), ptr(co(0)), ptr(e0), Sense::Forward, ptr(loop_base + 1));  // co1

    // ── Loops (2) ───────────────────────────────────────────────────
    sat.add_loop(SatPointer::NULL, ptr(co(0)), ptr(face_base));       // base cap
    sat.add_loop(SatPointer::NULL, ptr(co(1)), ptr(face_base + 1));   // cone lateral

    // ── Faces (2, linked list) ──────────────────────────────────────
    sat.add_face(ptr(face_base + 1), ptr(loop_base),     ptr(shell_idx), ptr(surf_base), Sense::Forward, Sidedness::Single);
    sat.add_face(SatPointer::NULL,   ptr(loop_base + 1), ptr(shell_idx), ptr(surf_cone), Sense::Forward, Sidedness::Single);

    // ── Shell → Lump → Body ─────────────────────────────────────────
    sat.add_shell(ptr(face_base), ptr(lump_idx));
    sat.add_lump(ptr(shell_idx), body_idx);

    if let Some(body_rec) = sat.record_mut(0) {
        body_rec.tokens[1] = SatToken::Pointer(ptr(lump_idx));
    }

    sat
}

// ════════════════════════════════════════════════════════════════════════════
//  Sphere — radius 5
// ════════════════════════════════════════════════════════════════════════════

/// Build a sphere of radius 5 centered at the origin.
///
/// 1 face, 0 edges, 0 vertices. Closed surface — no loop entity needed;
/// the face's first_loop is NULL, meaning it covers the entire sphere.
fn build_sphere_sat() -> SatDocument {
    let mut sat = SatDocument::new_body();
    let body_idx = SatPointer::new(0);
    let ptr = |i: i32| SatPointer::new(i);

    // ── Surface (1) ─────────────────────────────────────────────────
    let surf = sat.add_sphere_surface(
        [0.0, 0.0, 0.0],  // center
        5.0,               // radius
        [1.0, 0.0, 0.0],  // u direction
        [0.0, 0.0, 1.0],  // pole direction
    );

    // ── Closed surface: no loop, face.first_loop = NULL ─────────────
    let base = sat.records.len() as i32;
    let face_idx  = base;
    let shell_idx = base + 1;
    let lump_idx  = base + 2;

    // ── Face (1) ────────────────────────────────────────────────────
    sat.add_face(SatPointer::NULL, SatPointer::NULL, ptr(shell_idx), ptr(surf), Sense::Forward, Sidedness::Single);

    // ── Shell → Lump → Body ─────────────────────────────────────────
    sat.add_shell(ptr(face_idx), ptr(lump_idx));
    sat.add_lump(ptr(shell_idx), body_idx);

    if let Some(body_rec) = sat.record_mut(0) {
        body_rec.tokens[1] = SatToken::Pointer(ptr(lump_idx));
    }

    sat
}

// ════════════════════════════════════════════════════════════════════════════
//  Torus — major radius 5, minor radius 2
// ════════════════════════════════════════════════════════════════════════════

/// Build a torus centered at the origin with major radius 5 and minor radius 2.
/// The axis of revolution is the Z-axis.
///
/// 1 face, 0 edges, 0 vertices. Closed surface — no loop entity needed;
/// the face's first_loop is NULL, meaning it covers the entire torus.
fn build_torus_sat() -> SatDocument {
    let mut sat = SatDocument::new_body();
    let body_idx = SatPointer::new(0);
    let ptr = |i: i32| SatPointer::new(i);

    // ── Surface (1) ─────────────────────────────────────────────────
    let surf = sat.add_torus_surface(
        [0.0, 0.0, 0.0],  // center
        [0.0, 0.0, 1.0],  // normal (axis of revolution)
        5.0,               // major radius
        2.0,               // minor radius
        [1.0, 0.0, 0.0],  // u direction
    );

    // ── Closed surface: no loop, face.first_loop = NULL ─────────────
    let base = sat.records.len() as i32;
    let face_idx  = base;
    let shell_idx = base + 1;
    let lump_idx  = base + 2;

    // ── Face (1) ────────────────────────────────────────────────────
    sat.add_face(SatPointer::NULL, SatPointer::NULL, ptr(shell_idx), ptr(surf), Sense::Forward, Sidedness::Single);

    // ── Shell → Lump → Body ─────────────────────────────────────────
    sat.add_shell(ptr(face_idx), ptr(lump_idx));
    sat.add_lump(ptr(shell_idx), body_idx);

    if let Some(body_rec) = sat.record_mut(0) {
        body_rec.tokens[1] = SatToken::Pointer(ptr(lump_idx));
    }

    sat
}
