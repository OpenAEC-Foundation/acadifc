//! Example: write DXF files containing 3DSOLID entities with valid SAT data.
//!
//! Builds all seven ACIS primitive shapes using the SAT builder API,
//! wraps each in a `Solid3D` entity, and writes R2013 DXF files.
//!
//! Primitives:
//! - **Box** (10×10×10) — 6 faces, 12 edges, 8 vertices
//! - **Wedge** (right triangular prism) — 5 faces, 9 edges, 6 vertices
//! - **Pyramid** (square base, apex) — 5 faces, 8 edges, 5 vertices
//! - **Cylinder** (radius 5, height 10) — 3 faces, 3 edges, 2 vertices
//! - **Cone** (base radius 5, height 10) — 2 faces, 1 edge, 1 vertex
//! - **Sphere** (radius 5) — 1 face, 0 edges, 0 vertices
//! - **Torus** (major 5, minor 2) — 1 face, 0 edges, 0 vertices
//!
//! ```
//! cargo run --example write_3dsolid_dxf
//! ```

use acadrust::{CadDocument, DxfWriter, DxfVersion, EntityType};
use acadrust::entities::Solid3D;
use acadrust::entities::acis::{SatDocument, SatPointer, SatToken, Sense, Sidedness};

fn main() -> acadrust::Result<()> {
    let version = DxfVersion::AC1027;

    let shapes: Vec<(&str, SatDocument)> = vec![
        ("box",      build_box_sat()),
        ("wedge",    build_wedge_sat()),
        ("pyramid",  build_pyramid_sat()),
        ("cylinder", build_cylinder_sat()),
        ("cone",     build_cone_sat()),
        ("sphere",   build_sphere_sat()),
        ("torus",    build_torus_sat()),
    ];

    println!("=== Building SAT primitives ===");
    for (name, sat) in &shapes {
        let errors = sat.validate();
        println!("  {:10} {} bodies, {} faces, {} edges, {} vertices, {} warnings",
            name,
            sat.bodies().len(),
            sat.faces().len(),
            sat.edges().len(),
            sat.vertices().len(),
            errors.len(),
        );
        for e in &errors {
            println!("    WARNING: {:?}", e);
        }
    }

    println!("\n=== Writing DXF files (version: {:?}) ===", version);
    for (name, sat) in &shapes {
        let path = format!("{}.dxf", name);

        let mut solid = Solid3D::new();
        solid.set_sat_document(sat);
        solid.common.layer = "0".to_string();

        let mut doc = CadDocument::with_version(version);
        doc.add_entity(EntityType::Solid3D(solid))?;

        let writer = DxfWriter::new(doc);
        writer.write_to_file(&path)?;

        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        println!("  {} ({} bytes)", path, size);
    }

    println!("\nDone! Open any .dxf file in AutoCAD/IntelliCAD.");
    Ok(())
}

// ════════════════════════════════════════════════════════════════════════════
//  Helpers
// ════════════════════════════════════════════════════════════════════════════

fn ptr(i: i32) -> SatPointer {
    SatPointer::new(i)
}

// ════════════════════════════════════════════════════════════════════════════
//  Box — 10×10×10 centered at origin
// ════════════════════════════════════════════════════════════════════════════

fn build_box_sat() -> SatDocument {
    let mut sat = SatDocument::new_body();
    let body_idx = SatPointer::new(0);

    let p0 = sat.add_point(-5.0, -5.0, -5.0);
    let p1 = sat.add_point( 5.0, -5.0, -5.0);
    let p2 = sat.add_point( 5.0,  5.0, -5.0);
    let p3 = sat.add_point(-5.0,  5.0, -5.0);
    let p4 = sat.add_point(-5.0, -5.0,  5.0);
    let p5 = sat.add_point( 5.0, -5.0,  5.0);
    let p6 = sat.add_point( 5.0,  5.0,  5.0);
    let p7 = sat.add_point(-5.0,  5.0,  5.0);

    let surf_top    = sat.add_plane_surface([0.0, 0.0,  5.0], [0.0, 0.0,  1.0], [1.0, 0.0, 0.0]);
    let surf_bottom = sat.add_plane_surface([0.0, 0.0, -5.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]);
    let surf_front  = sat.add_plane_surface([0.0, -5.0, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]);
    let surf_back   = sat.add_plane_surface([0.0,  5.0, 0.0], [0.0,  1.0, 0.0], [1.0, 0.0, 0.0]);
    let surf_right  = sat.add_plane_surface([ 5.0, 0.0, 0.0], [ 1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    let surf_left   = sat.add_plane_surface([-5.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);

    let crv_b0 = sat.add_straight_curve([-5.0, -5.0, -5.0], [ 1.0, 0.0, 0.0]);
    let crv_b1 = sat.add_straight_curve([ 5.0, -5.0, -5.0], [ 0.0, 1.0, 0.0]);
    let crv_b2 = sat.add_straight_curve([ 5.0,  5.0, -5.0], [-1.0, 0.0, 0.0]);
    let crv_b3 = sat.add_straight_curve([-5.0,  5.0, -5.0], [ 0.0,-1.0, 0.0]);
    let crv_t0 = sat.add_straight_curve([-5.0, -5.0,  5.0], [ 1.0, 0.0, 0.0]);
    let crv_t1 = sat.add_straight_curve([ 5.0, -5.0,  5.0], [ 0.0, 1.0, 0.0]);
    let crv_t2 = sat.add_straight_curve([ 5.0,  5.0,  5.0], [-1.0, 0.0, 0.0]);
    let crv_t3 = sat.add_straight_curve([-5.0,  5.0,  5.0], [ 0.0,-1.0, 0.0]);
    let crv_v0 = sat.add_straight_curve([-5.0, -5.0, -5.0], [ 0.0, 0.0, 1.0]);
    let crv_v1 = sat.add_straight_curve([ 5.0, -5.0, -5.0], [ 0.0, 0.0, 1.0]);
    let crv_v2 = sat.add_straight_curve([ 5.0,  5.0, -5.0], [ 0.0, 0.0, 1.0]);
    let crv_v3 = sat.add_straight_curve([-5.0,  5.0, -5.0], [ 0.0, 0.0, 1.0]);

    let v0 = sat.add_vertex(SatPointer::NULL, ptr(p0));
    let v1 = sat.add_vertex(SatPointer::NULL, ptr(p1));
    let v2 = sat.add_vertex(SatPointer::NULL, ptr(p2));
    let v3 = sat.add_vertex(SatPointer::NULL, ptr(p3));
    let v4 = sat.add_vertex(SatPointer::NULL, ptr(p4));
    let v5 = sat.add_vertex(SatPointer::NULL, ptr(p5));
    let v6 = sat.add_vertex(SatPointer::NULL, ptr(p6));
    let v7 = sat.add_vertex(SatPointer::NULL, ptr(p7));

    let e0  = sat.add_edge(ptr(v0), 0.0, ptr(v1), 10.0, SatPointer::NULL, ptr(crv_b0), Sense::Forward);
    let e1  = sat.add_edge(ptr(v1), 0.0, ptr(v2), 10.0, SatPointer::NULL, ptr(crv_b1), Sense::Forward);
    let e2  = sat.add_edge(ptr(v2), 0.0, ptr(v3), 10.0, SatPointer::NULL, ptr(crv_b2), Sense::Forward);
    let e3  = sat.add_edge(ptr(v3), 0.0, ptr(v0), 10.0, SatPointer::NULL, ptr(crv_b3), Sense::Forward);
    let e4  = sat.add_edge(ptr(v4), 0.0, ptr(v5), 10.0, SatPointer::NULL, ptr(crv_t0), Sense::Forward);
    let e5  = sat.add_edge(ptr(v5), 0.0, ptr(v6), 10.0, SatPointer::NULL, ptr(crv_t1), Sense::Forward);
    let e6  = sat.add_edge(ptr(v6), 0.0, ptr(v7), 10.0, SatPointer::NULL, ptr(crv_t2), Sense::Forward);
    let e7  = sat.add_edge(ptr(v7), 0.0, ptr(v4), 10.0, SatPointer::NULL, ptr(crv_t3), Sense::Forward);
    let e8  = sat.add_edge(ptr(v0), 0.0, ptr(v4), 10.0, SatPointer::NULL, ptr(crv_v0), Sense::Forward);
    let e9  = sat.add_edge(ptr(v1), 0.0, ptr(v5), 10.0, SatPointer::NULL, ptr(crv_v1), Sense::Forward);
    let e10 = sat.add_edge(ptr(v2), 0.0, ptr(v6), 10.0, SatPointer::NULL, ptr(crv_v2), Sense::Forward);
    let e11 = sat.add_edge(ptr(v3), 0.0, ptr(v7), 10.0, SatPointer::NULL, ptr(crv_v3), Sense::Forward);

    let base = sat.records.len() as i32;
    let co_base   = base;
    let loop_base = base + 24;
    let face_base = base + 30;
    let shell_idx = base + 36;
    let lump_idx  = base + 37;
    let co = |i: i32| co_base + i;

    // Bottom face
    sat.add_coedge(ptr(co(1)),  ptr(co(3)),  ptr(co(8)),  ptr(e0), Sense::Reversed, ptr(loop_base));
    sat.add_coedge(ptr(co(2)),  ptr(co(0)),  ptr(co(20)), ptr(e3), Sense::Reversed, ptr(loop_base));
    sat.add_coedge(ptr(co(3)),  ptr(co(1)),  ptr(co(12)), ptr(e2), Sense::Reversed, ptr(loop_base));
    sat.add_coedge(ptr(co(0)),  ptr(co(2)),  ptr(co(16)), ptr(e1), Sense::Reversed, ptr(loop_base));
    // Top face
    sat.add_coedge(ptr(co(5)),  ptr(co(7)),  ptr(co(10)), ptr(e4), Sense::Forward, ptr(loop_base + 1));
    sat.add_coedge(ptr(co(6)),  ptr(co(4)),  ptr(co(18)), ptr(e5), Sense::Forward, ptr(loop_base + 1));
    sat.add_coedge(ptr(co(7)),  ptr(co(5)),  ptr(co(14)), ptr(e6), Sense::Forward, ptr(loop_base + 1));
    sat.add_coedge(ptr(co(4)),  ptr(co(6)),  ptr(co(22)), ptr(e7), Sense::Forward, ptr(loop_base + 1));
    // Front face
    sat.add_coedge(ptr(co(9)),  ptr(co(11)), ptr(co(0)),  ptr(e0), Sense::Forward,  ptr(loop_base + 2));
    sat.add_coedge(ptr(co(10)), ptr(co(8)),  ptr(co(19)), ptr(e9), Sense::Forward,  ptr(loop_base + 2));
    sat.add_coedge(ptr(co(11)), ptr(co(9)),  ptr(co(4)),  ptr(e4), Sense::Reversed, ptr(loop_base + 2));
    sat.add_coedge(ptr(co(8)),  ptr(co(10)), ptr(co(21)), ptr(e8), Sense::Reversed, ptr(loop_base + 2));
    // Back face
    sat.add_coedge(ptr(co(13)), ptr(co(15)), ptr(co(2)),  ptr(e2),  Sense::Forward,  ptr(loop_base + 3));
    sat.add_coedge(ptr(co(14)), ptr(co(12)), ptr(co(23)), ptr(e11), Sense::Forward,  ptr(loop_base + 3));
    sat.add_coedge(ptr(co(15)), ptr(co(13)), ptr(co(6)),  ptr(e6),  Sense::Reversed, ptr(loop_base + 3));
    sat.add_coedge(ptr(co(12)), ptr(co(14)), ptr(co(17)), ptr(e10), Sense::Reversed, ptr(loop_base + 3));
    // Right face
    sat.add_coedge(ptr(co(17)), ptr(co(19)), ptr(co(3)),  ptr(e1),  Sense::Forward,  ptr(loop_base + 4));
    sat.add_coedge(ptr(co(18)), ptr(co(16)), ptr(co(15)), ptr(e10), Sense::Forward,  ptr(loop_base + 4));
    sat.add_coedge(ptr(co(19)), ptr(co(17)), ptr(co(5)),  ptr(e5),  Sense::Reversed, ptr(loop_base + 4));
    sat.add_coedge(ptr(co(16)), ptr(co(18)), ptr(co(9)),  ptr(e9),  Sense::Reversed, ptr(loop_base + 4));
    // Left face
    sat.add_coedge(ptr(co(21)), ptr(co(23)), ptr(co(1)),  ptr(e3),  Sense::Forward,  ptr(loop_base + 5));
    sat.add_coedge(ptr(co(22)), ptr(co(20)), ptr(co(11)), ptr(e8),  Sense::Forward,  ptr(loop_base + 5));
    sat.add_coedge(ptr(co(23)), ptr(co(21)), ptr(co(7)),  ptr(e7),  Sense::Reversed, ptr(loop_base + 5));
    sat.add_coedge(ptr(co(20)), ptr(co(22)), ptr(co(13)), ptr(e11), Sense::Reversed, ptr(loop_base + 5));

    sat.add_loop(SatPointer::NULL, ptr(co(0)),  ptr(face_base));
    sat.add_loop(SatPointer::NULL, ptr(co(4)),  ptr(face_base + 1));
    sat.add_loop(SatPointer::NULL, ptr(co(8)),  ptr(face_base + 2));
    sat.add_loop(SatPointer::NULL, ptr(co(12)), ptr(face_base + 3));
    sat.add_loop(SatPointer::NULL, ptr(co(16)), ptr(face_base + 4));
    sat.add_loop(SatPointer::NULL, ptr(co(20)), ptr(face_base + 5));

    sat.add_face(ptr(face_base + 1), ptr(loop_base),     ptr(shell_idx), ptr(surf_bottom), Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 2), ptr(loop_base + 1), ptr(shell_idx), ptr(surf_top),    Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 3), ptr(loop_base + 2), ptr(shell_idx), ptr(surf_front),  Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 4), ptr(loop_base + 3), ptr(shell_idx), ptr(surf_back),   Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 5), ptr(loop_base + 4), ptr(shell_idx), ptr(surf_right),  Sense::Forward, Sidedness::Single);
    sat.add_face(SatPointer::NULL,   ptr(loop_base + 5), ptr(shell_idx), ptr(surf_left),   Sense::Forward, Sidedness::Single);

    sat.add_shell(ptr(face_base), ptr(lump_idx));
    sat.add_lump(ptr(shell_idx), body_idx);

    if let Some(body_rec) = sat.record_mut(0) {
        body_rec.tokens[1] = SatToken::Pointer(ptr(lump_idx));
    }
    sat
}

// ════════════════════════════════════════════════════════════════════════════
//  Wedge — Right Triangular Prism
// ════════════════════════════════════════════════════════════════════════════

fn build_wedge_sat() -> SatDocument {
    let mut sat = SatDocument::new_body();
    let body_idx = SatPointer::new(0);

    let s = std::f64::consts::FRAC_1_SQRT_2;
    let hyp_len = 10.0 * 2.0_f64.sqrt();

    let p_a = sat.add_point(0.0,  0.0,  0.0);
    let p_b = sat.add_point(10.0, 0.0,  0.0);
    let p_c = sat.add_point(0.0,  10.0, 0.0);
    let p_d = sat.add_point(0.0,  0.0,  10.0);
    let p_e = sat.add_point(10.0, 0.0,  10.0);
    let p_f = sat.add_point(0.0,  10.0, 10.0);

    let surf_bot = sat.add_plane_surface([0.0, 0.0, 0.0],  [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]);
    let surf_top = sat.add_plane_surface([0.0, 0.0, 10.0], [0.0, 0.0,  1.0], [1.0, 0.0, 0.0]);
    let surf_frt = sat.add_plane_surface([0.0, 0.0, 0.0],  [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]);
    let surf_lft = sat.add_plane_surface([0.0, 0.0, 0.0],  [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    let surf_hyp = sat.add_plane_surface([5.0, 5.0, 0.0],  [s,    s,   0.0], [0.0, 0.0, 1.0]);

    let crv0 = sat.add_straight_curve([0.0, 0.0, 0.0],   [1.0, 0.0, 0.0]);
    let crv1 = sat.add_straight_curve([10.0, 0.0, 0.0],  [-s,   s,  0.0]);
    let crv2 = sat.add_straight_curve([0.0, 10.0, 0.0],  [0.0, -1.0, 0.0]);
    let crv3 = sat.add_straight_curve([0.0, 0.0, 10.0],  [1.0, 0.0, 0.0]);
    let crv4 = sat.add_straight_curve([10.0, 0.0, 10.0], [-s,   s,  0.0]);
    let crv5 = sat.add_straight_curve([0.0, 10.0, 10.0], [0.0, -1.0, 0.0]);
    let crv6 = sat.add_straight_curve([0.0, 0.0, 0.0],   [0.0, 0.0, 1.0]);
    let crv7 = sat.add_straight_curve([10.0, 0.0, 0.0],  [0.0, 0.0, 1.0]);
    let crv8 = sat.add_straight_curve([0.0, 10.0, 0.0],  [0.0, 0.0, 1.0]);

    let v_a = sat.add_vertex(SatPointer::NULL, ptr(p_a));
    let v_b = sat.add_vertex(SatPointer::NULL, ptr(p_b));
    let v_c = sat.add_vertex(SatPointer::NULL, ptr(p_c));
    let v_d = sat.add_vertex(SatPointer::NULL, ptr(p_d));
    let v_e = sat.add_vertex(SatPointer::NULL, ptr(p_e));
    let v_f = sat.add_vertex(SatPointer::NULL, ptr(p_f));

    let e0 = sat.add_edge(ptr(v_a), 0.0, ptr(v_b), 10.0,    SatPointer::NULL, ptr(crv0), Sense::Forward);
    let e1 = sat.add_edge(ptr(v_b), 0.0, ptr(v_c), hyp_len, SatPointer::NULL, ptr(crv1), Sense::Forward);
    let e2 = sat.add_edge(ptr(v_c), 0.0, ptr(v_a), 10.0,    SatPointer::NULL, ptr(crv2), Sense::Forward);
    let e3 = sat.add_edge(ptr(v_d), 0.0, ptr(v_e), 10.0,    SatPointer::NULL, ptr(crv3), Sense::Forward);
    let e4 = sat.add_edge(ptr(v_e), 0.0, ptr(v_f), hyp_len, SatPointer::NULL, ptr(crv4), Sense::Forward);
    let e5 = sat.add_edge(ptr(v_f), 0.0, ptr(v_d), 10.0,    SatPointer::NULL, ptr(crv5), Sense::Forward);
    let e6 = sat.add_edge(ptr(v_a), 0.0, ptr(v_d), 10.0,    SatPointer::NULL, ptr(crv6), Sense::Forward);
    let e7 = sat.add_edge(ptr(v_b), 0.0, ptr(v_e), 10.0,    SatPointer::NULL, ptr(crv7), Sense::Forward);
    let e8 = sat.add_edge(ptr(v_c), 0.0, ptr(v_f), 10.0,    SatPointer::NULL, ptr(crv8), Sense::Forward);

    let base = sat.records.len() as i32;
    let co = |i: i32| base + i;
    let loop_base = base + 18;
    let face_base = base + 23;
    let shell_idx = base + 28;
    let lump_idx  = base + 29;

    // Bottom: A→C→B
    sat.add_coedge(ptr(co(1)),  ptr(co(2)),  ptr(co(13)), ptr(e2), Sense::Reversed, ptr(loop_base));
    sat.add_coedge(ptr(co(2)),  ptr(co(0)),  ptr(co(14)), ptr(e1), Sense::Reversed, ptr(loop_base));
    sat.add_coedge(ptr(co(0)),  ptr(co(1)),  ptr(co(6)),  ptr(e0), Sense::Reversed, ptr(loop_base));
    // Top: D→E→F
    sat.add_coedge(ptr(co(4)),  ptr(co(5)),  ptr(co(8)),  ptr(e3), Sense::Forward, ptr(loop_base + 1));
    sat.add_coedge(ptr(co(5)),  ptr(co(3)),  ptr(co(16)), ptr(e4), Sense::Forward, ptr(loop_base + 1));
    sat.add_coedge(ptr(co(3)),  ptr(co(4)),  ptr(co(11)), ptr(e5), Sense::Forward, ptr(loop_base + 1));
    // Front: A→B→E→D
    sat.add_coedge(ptr(co(7)),  ptr(co(9)),  ptr(co(2)),  ptr(e0), Sense::Forward,  ptr(loop_base + 2));
    sat.add_coedge(ptr(co(8)),  ptr(co(6)),  ptr(co(17)), ptr(e7), Sense::Forward,  ptr(loop_base + 2));
    sat.add_coedge(ptr(co(9)),  ptr(co(7)),  ptr(co(3)),  ptr(e3), Sense::Reversed, ptr(loop_base + 2));
    sat.add_coedge(ptr(co(6)),  ptr(co(8)),  ptr(co(10)), ptr(e6), Sense::Reversed, ptr(loop_base + 2));
    // Left: A→D→F→C
    sat.add_coedge(ptr(co(11)), ptr(co(13)), ptr(co(9)),  ptr(e6), Sense::Forward,  ptr(loop_base + 3));
    sat.add_coedge(ptr(co(12)), ptr(co(10)), ptr(co(5)),  ptr(e5), Sense::Reversed, ptr(loop_base + 3));
    sat.add_coedge(ptr(co(13)), ptr(co(11)), ptr(co(15)), ptr(e8), Sense::Reversed, ptr(loop_base + 3));
    sat.add_coedge(ptr(co(10)), ptr(co(12)), ptr(co(0)),  ptr(e2), Sense::Forward,  ptr(loop_base + 3));
    // Hypotenuse: B→C→F→E
    sat.add_coedge(ptr(co(15)), ptr(co(17)), ptr(co(1)),  ptr(e1), Sense::Forward,  ptr(loop_base + 4));
    sat.add_coedge(ptr(co(16)), ptr(co(14)), ptr(co(12)), ptr(e8), Sense::Forward,  ptr(loop_base + 4));
    sat.add_coedge(ptr(co(17)), ptr(co(15)), ptr(co(4)),  ptr(e4), Sense::Reversed, ptr(loop_base + 4));
    sat.add_coedge(ptr(co(14)), ptr(co(16)), ptr(co(7)),  ptr(e7), Sense::Reversed, ptr(loop_base + 4));

    sat.add_loop(SatPointer::NULL, ptr(co(0)),  ptr(face_base));
    sat.add_loop(SatPointer::NULL, ptr(co(3)),  ptr(face_base + 1));
    sat.add_loop(SatPointer::NULL, ptr(co(6)),  ptr(face_base + 2));
    sat.add_loop(SatPointer::NULL, ptr(co(10)), ptr(face_base + 3));
    sat.add_loop(SatPointer::NULL, ptr(co(14)), ptr(face_base + 4));

    sat.add_face(ptr(face_base + 1), ptr(loop_base),     ptr(shell_idx), ptr(surf_bot), Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 2), ptr(loop_base + 1), ptr(shell_idx), ptr(surf_top), Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 3), ptr(loop_base + 2), ptr(shell_idx), ptr(surf_frt), Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 4), ptr(loop_base + 3), ptr(shell_idx), ptr(surf_lft), Sense::Forward, Sidedness::Single);
    sat.add_face(SatPointer::NULL,   ptr(loop_base + 4), ptr(shell_idx), ptr(surf_hyp), Sense::Forward, Sidedness::Single);

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

fn build_pyramid_sat() -> SatDocument {
    let mut sat = SatDocument::new_body();
    let body_idx = SatPointer::new(0);

    let s5 = 5.0_f64.sqrt();
    let n1 = 2.0 / s5;
    let n2 = 1.0 / s5;
    let s6 = 6.0_f64.sqrt();
    let lat_len = 5.0 * s6;

    let p_a = sat.add_point(-5.0, -5.0, 0.0);
    let p_b = sat.add_point( 5.0, -5.0, 0.0);
    let p_c = sat.add_point( 5.0,  5.0, 0.0);
    let p_d = sat.add_point(-5.0,  5.0, 0.0);
    let p_e = sat.add_point( 0.0,  0.0, 10.0);

    let surf_base  = sat.add_plane_surface([0.0, 0.0, 0.0], [0.0,  0.0, -1.0], [1.0, 0.0, 0.0]);
    let surf_front = sat.add_plane_surface([0.0, -5.0, 0.0], [0.0, -n1,  n2], [1.0, 0.0, 0.0]);
    let surf_right = sat.add_plane_surface([5.0,  0.0, 0.0], [n1,   0.0, n2], [0.0, 1.0, 0.0]);
    let surf_back  = sat.add_plane_surface([0.0,  5.0, 0.0], [0.0,  n1,  n2], [1.0, 0.0, 0.0]);
    let surf_left  = sat.add_plane_surface([-5.0, 0.0, 0.0], [-n1,  0.0, n2], [0.0, 1.0, 0.0]);

    let crv0 = sat.add_straight_curve([-5.0, -5.0, 0.0], [ 1.0, 0.0, 0.0]);
    let crv1 = sat.add_straight_curve([ 5.0, -5.0, 0.0], [ 0.0, 1.0, 0.0]);
    let crv2 = sat.add_straight_curve([ 5.0,  5.0, 0.0], [-1.0, 0.0, 0.0]);
    let crv3 = sat.add_straight_curve([-5.0,  5.0, 0.0], [ 0.0,-1.0, 0.0]);
    let crv4 = sat.add_straight_curve([-5.0, -5.0, 0.0], [ 1.0/s6,  1.0/s6, 2.0/s6]);
    let crv5 = sat.add_straight_curve([ 5.0, -5.0, 0.0], [-1.0/s6,  1.0/s6, 2.0/s6]);
    let crv6 = sat.add_straight_curve([ 5.0,  5.0, 0.0], [-1.0/s6, -1.0/s6, 2.0/s6]);
    let crv7 = sat.add_straight_curve([-5.0,  5.0, 0.0], [ 1.0/s6, -1.0/s6, 2.0/s6]);

    let v_a = sat.add_vertex(SatPointer::NULL, ptr(p_a));
    let v_b = sat.add_vertex(SatPointer::NULL, ptr(p_b));
    let v_c = sat.add_vertex(SatPointer::NULL, ptr(p_c));
    let v_d = sat.add_vertex(SatPointer::NULL, ptr(p_d));
    let v_e = sat.add_vertex(SatPointer::NULL, ptr(p_e));

    let e0 = sat.add_edge(ptr(v_a), 0.0, ptr(v_b), 10.0,    SatPointer::NULL, ptr(crv0), Sense::Forward);
    let e1 = sat.add_edge(ptr(v_b), 0.0, ptr(v_c), 10.0,    SatPointer::NULL, ptr(crv1), Sense::Forward);
    let e2 = sat.add_edge(ptr(v_c), 0.0, ptr(v_d), 10.0,    SatPointer::NULL, ptr(crv2), Sense::Forward);
    let e3 = sat.add_edge(ptr(v_d), 0.0, ptr(v_a), 10.0,    SatPointer::NULL, ptr(crv3), Sense::Forward);
    let e4 = sat.add_edge(ptr(v_a), 0.0, ptr(v_e), lat_len, SatPointer::NULL, ptr(crv4), Sense::Forward);
    let e5 = sat.add_edge(ptr(v_b), 0.0, ptr(v_e), lat_len, SatPointer::NULL, ptr(crv5), Sense::Forward);
    let e6 = sat.add_edge(ptr(v_c), 0.0, ptr(v_e), lat_len, SatPointer::NULL, ptr(crv6), Sense::Forward);
    let e7 = sat.add_edge(ptr(v_d), 0.0, ptr(v_e), lat_len, SatPointer::NULL, ptr(crv7), Sense::Forward);

    let base = sat.records.len() as i32;
    let co = |i: i32| base + i;
    let loop_base = base + 16;
    let face_base = base + 21;
    let shell_idx = base + 26;
    let lump_idx  = base + 27;

    // Base: A→D→C→B
    sat.add_coedge(ptr(co(1)),  ptr(co(3)),  ptr(co(15)), ptr(e3), Sense::Reversed, ptr(loop_base));
    sat.add_coedge(ptr(co(2)),  ptr(co(0)),  ptr(co(10)), ptr(e2), Sense::Reversed, ptr(loop_base));
    sat.add_coedge(ptr(co(3)),  ptr(co(1)),  ptr(co(7)),  ptr(e1), Sense::Reversed, ptr(loop_base));
    sat.add_coedge(ptr(co(0)),  ptr(co(2)),  ptr(co(4)),  ptr(e0), Sense::Reversed, ptr(loop_base));
    // Front: A→B→E
    sat.add_coedge(ptr(co(5)),  ptr(co(6)),  ptr(co(3)),  ptr(e0), Sense::Forward,  ptr(loop_base + 1));
    sat.add_coedge(ptr(co(6)),  ptr(co(4)),  ptr(co(9)),  ptr(e5), Sense::Forward,  ptr(loop_base + 1));
    sat.add_coedge(ptr(co(4)),  ptr(co(5)),  ptr(co(14)), ptr(e4), Sense::Reversed, ptr(loop_base + 1));
    // Right: B→C→E
    sat.add_coedge(ptr(co(8)),  ptr(co(9)),  ptr(co(2)),  ptr(e1), Sense::Forward,  ptr(loop_base + 2));
    sat.add_coedge(ptr(co(9)),  ptr(co(7)),  ptr(co(12)), ptr(e6), Sense::Forward,  ptr(loop_base + 2));
    sat.add_coedge(ptr(co(7)),  ptr(co(8)),  ptr(co(5)),  ptr(e5), Sense::Reversed, ptr(loop_base + 2));
    // Back: C→D→E
    sat.add_coedge(ptr(co(11)), ptr(co(12)), ptr(co(1)),  ptr(e2), Sense::Forward,  ptr(loop_base + 3));
    sat.add_coedge(ptr(co(12)), ptr(co(10)), ptr(co(15)), ptr(e7), Sense::Forward,  ptr(loop_base + 3));
    sat.add_coedge(ptr(co(10)), ptr(co(11)), ptr(co(8)),  ptr(e6), Sense::Reversed, ptr(loop_base + 3));
    // Left: D→A→E
    sat.add_coedge(ptr(co(14)), ptr(co(15)), ptr(co(0)),  ptr(e3), Sense::Forward,  ptr(loop_base + 4));
    sat.add_coedge(ptr(co(15)), ptr(co(13)), ptr(co(6)),  ptr(e4), Sense::Forward,  ptr(loop_base + 4));
    sat.add_coedge(ptr(co(13)), ptr(co(14)), ptr(co(11)), ptr(e7), Sense::Reversed, ptr(loop_base + 4));

    sat.add_loop(SatPointer::NULL, ptr(co(0)),  ptr(face_base));
    sat.add_loop(SatPointer::NULL, ptr(co(4)),  ptr(face_base + 1));
    sat.add_loop(SatPointer::NULL, ptr(co(7)),  ptr(face_base + 2));
    sat.add_loop(SatPointer::NULL, ptr(co(10)), ptr(face_base + 3));
    sat.add_loop(SatPointer::NULL, ptr(co(13)), ptr(face_base + 4));

    sat.add_face(ptr(face_base + 1), ptr(loop_base),     ptr(shell_idx), ptr(surf_base),  Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 2), ptr(loop_base + 1), ptr(shell_idx), ptr(surf_front), Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 3), ptr(loop_base + 2), ptr(shell_idx), ptr(surf_right), Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 4), ptr(loop_base + 3), ptr(shell_idx), ptr(surf_back),  Sense::Forward, Sidedness::Single);
    sat.add_face(SatPointer::NULL,   ptr(loop_base + 4), ptr(shell_idx), ptr(surf_left),  Sense::Forward, Sidedness::Single);

    sat.add_shell(ptr(face_base), ptr(lump_idx));
    sat.add_lump(ptr(shell_idx), body_idx);

    if let Some(body_rec) = sat.record_mut(0) {
        body_rec.tokens[1] = SatToken::Pointer(ptr(lump_idx));
    }
    sat
}

// ════════════════════════════════════════════════════════════════════════════
//  Cylinder — radius 5, height 10
// ════════════════════════════════════════════════════════════════════════════

fn build_cylinder_sat() -> SatDocument {
    let mut sat = SatDocument::new_body();
    let body_idx = SatPointer::new(0);
    let tau = std::f64::consts::TAU;

    let p0 = sat.add_point(5.0, 0.0, 0.0);
    let p1 = sat.add_point(5.0, 0.0, 10.0);

    let surf_bot = sat.add_plane_surface([0.0, 0.0, 0.0],  [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]);
    let surf_top = sat.add_plane_surface([0.0, 0.0, 10.0], [0.0, 0.0,  1.0], [1.0, 0.0, 0.0]);
    let surf_cyl = sat.add_cone_surface(
        [0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [5.0, 0.0, 0.0],
        1.0, 1.0, 0.0,
    );

    let crv_bot  = sat.add_ellipse_curve([0.0, 0.0, 0.0],  [0.0, 0.0, 1.0], [5.0, 0.0, 0.0], 1.0);
    let crv_top  = sat.add_ellipse_curve([0.0, 0.0, 10.0], [0.0, 0.0, 1.0], [5.0, 0.0, 0.0], 1.0);
    let crv_seam = sat.add_straight_curve([5.0, 0.0, 0.0],  [0.0, 0.0, 1.0]);

    let v0 = sat.add_vertex(SatPointer::NULL, ptr(p0));
    let v1 = sat.add_vertex(SatPointer::NULL, ptr(p1));

    let e_bot  = sat.add_edge(ptr(v0), 0.0, ptr(v0), tau, SatPointer::NULL, ptr(crv_bot),  Sense::Forward);
    let e_top  = sat.add_edge(ptr(v1), 0.0, ptr(v1), tau, SatPointer::NULL, ptr(crv_top),  Sense::Forward);
    let e_seam = sat.add_edge(ptr(v0), 0.0, ptr(v1), 10.0, SatPointer::NULL, ptr(crv_seam), Sense::Forward);

    let base = sat.records.len() as i32;
    let co = |i: i32| base + i;
    let loop_base = base + 6;
    let face_base = base + 9;
    let shell_idx = base + 12;
    let lump_idx  = base + 13;

    sat.add_coedge(ptr(co(0)), ptr(co(0)), ptr(co(4)), ptr(e_bot), Sense::Reversed, ptr(loop_base));
    sat.add_coedge(ptr(co(1)), ptr(co(1)), ptr(co(2)), ptr(e_top), Sense::Forward, ptr(loop_base + 1));
    sat.add_coedge(ptr(co(5)), ptr(co(3)), ptr(co(1)),  ptr(e_top),  Sense::Reversed, ptr(loop_base + 2));
    sat.add_coedge(ptr(co(2)), ptr(co(4)), ptr(co(5)),  ptr(e_seam), Sense::Forward,  ptr(loop_base + 2));
    sat.add_coedge(ptr(co(3)), ptr(co(5)), ptr(co(0)),  ptr(e_bot),  Sense::Forward,  ptr(loop_base + 2));
    sat.add_coedge(ptr(co(4)), ptr(co(2)), ptr(co(3)),  ptr(e_seam), Sense::Reversed, ptr(loop_base + 2));

    sat.add_loop(SatPointer::NULL, ptr(co(0)), ptr(face_base));
    sat.add_loop(SatPointer::NULL, ptr(co(1)), ptr(face_base + 1));
    sat.add_loop(SatPointer::NULL, ptr(co(2)), ptr(face_base + 2));

    sat.add_face(ptr(face_base + 1), ptr(loop_base),     ptr(shell_idx), ptr(surf_bot), Sense::Forward, Sidedness::Single);
    sat.add_face(ptr(face_base + 2), ptr(loop_base + 1), ptr(shell_idx), ptr(surf_top), Sense::Forward, Sidedness::Single);
    sat.add_face(SatPointer::NULL,   ptr(loop_base + 2), ptr(shell_idx), ptr(surf_cyl), Sense::Forward, Sidedness::Single);

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

fn build_cone_sat() -> SatDocument {
    let mut sat = SatDocument::new_body();
    let body_idx = SatPointer::new(0);
    let tau = std::f64::consts::TAU;
    let hyp = (5.0_f64 * 5.0 + 10.0 * 10.0).sqrt();
    let sin_half = -5.0 / hyp;
    let cos_half = 10.0 / hyp;

    let p0 = sat.add_point(5.0, 0.0, 0.0);

    let surf_base = sat.add_plane_surface([0.0, 0.0, 0.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]);
    let surf_cone = sat.add_cone_surface(
        [0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [5.0, 0.0, 0.0],
        1.0, cos_half, sin_half,
    );

    let crv_base = sat.add_ellipse_curve(
        [0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [5.0, 0.0, 0.0], 1.0,
    );

    let v0 = sat.add_vertex(SatPointer::NULL, ptr(p0));
    let e0 = sat.add_edge(ptr(v0), 0.0, ptr(v0), tau, SatPointer::NULL, ptr(crv_base), Sense::Forward);

    let base = sat.records.len() as i32;
    let co = |i: i32| base + i;
    let loop_base = base + 2;
    let face_base = base + 4;
    let shell_idx = base + 6;
    let lump_idx  = base + 7;

    sat.add_coedge(ptr(co(0)), ptr(co(0)), ptr(co(1)), ptr(e0), Sense::Reversed, ptr(loop_base));
    sat.add_coedge(ptr(co(1)), ptr(co(1)), ptr(co(0)), ptr(e0), Sense::Forward,  ptr(loop_base + 1));

    sat.add_loop(SatPointer::NULL, ptr(co(0)), ptr(face_base));
    sat.add_loop(SatPointer::NULL, ptr(co(1)), ptr(face_base + 1));

    sat.add_face(ptr(face_base + 1), ptr(loop_base),     ptr(shell_idx), ptr(surf_base), Sense::Forward, Sidedness::Single);
    sat.add_face(SatPointer::NULL,   ptr(loop_base + 1), ptr(shell_idx), ptr(surf_cone), Sense::Forward, Sidedness::Single);

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

fn build_sphere_sat() -> SatDocument {
    let mut sat = SatDocument::new_body();
    let body_idx = SatPointer::new(0);

    let surf = sat.add_sphere_surface(
        [0.0, 0.0, 0.0], 5.0, [1.0, 0.0, 0.0], [0.0, 0.0, 1.0],
    );

    // Closed surface: no loop entity, face.first_loop = NULL
    let base = sat.records.len() as i32;
    let face_idx  = base;
    let shell_idx = base + 1;
    let lump_idx  = base + 2;

    sat.add_face(SatPointer::NULL, SatPointer::NULL, ptr(shell_idx), ptr(surf), Sense::Forward, Sidedness::Single);
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

fn build_torus_sat() -> SatDocument {
    let mut sat = SatDocument::new_body();
    let body_idx = SatPointer::new(0);

    let surf = sat.add_torus_surface(
        [0.0, 0.0, 0.0], [0.0, 0.0, 1.0], 5.0, 2.0, [1.0, 0.0, 0.0],
    );

    // Closed surface: no loop entity, face.first_loop = NULL
    let base = sat.records.len() as i32;
    let face_idx  = base;
    let shell_idx = base + 1;
    let lump_idx  = base + 2;

    sat.add_face(SatPointer::NULL, SatPointer::NULL, ptr(shell_idx), ptr(surf), Sense::Forward, Sidedness::Single);
    sat.add_shell(ptr(face_idx), ptr(lump_idx));
    sat.add_lump(ptr(shell_idx), body_idx);

    if let Some(body_rec) = sat.record_mut(0) {
        body_rec.tokens[1] = SatToken::Pointer(ptr(lump_idx));
    }
    sat
}