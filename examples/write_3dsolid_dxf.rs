//! Example: write a DXF file containing a 3DSOLID entity with valid SAT data.
//!
//! Builds a 10×10×10 box centered at the origin using the ACIS SAT builder
//! API, wraps it in a `Solid3D` entity, and writes an R2010 DXF file.
//!
//! ```
//! cargo run --example write_3dsolid_dxf
//! ```

use acadrust::{CadDocument, DxfWriter, DxfVersion, EntityType};
use acadrust::entities::Solid3D;
use acadrust::entities::acis::{SatDocument, SatPointer, SatToken, Sense, Sidedness};

fn main() -> acadrust::Result<()> {
    // ── 1. Build the SAT document describing a 10×10×10 box ──────────
    let mut sat = SatDocument::new_body();

    // The body record is at index 1 (index 0 = asmheader).
    let body_idx = SatPointer::new(1);

    // ────────────── Geometry (surfaces + curves + points) ────────────

    // 8 corner points of the box (-5..5 on each axis)
    //   p0=(-5,-5,-5)  p1=(5,-5,-5)  p2=(5,5,-5)  p3=(-5,5,-5)
    //   p4=(-5,-5, 5)  p5=(5,-5, 5)  p6=(5,5, 5)  p7=(-5,5, 5)
    let p0 = sat.add_point(-5.0, -5.0, -5.0);
    let p1 = sat.add_point( 5.0, -5.0, -5.0);
    let p2 = sat.add_point( 5.0,  5.0, -5.0);
    let p3 = sat.add_point(-5.0,  5.0, -5.0);
    let p4 = sat.add_point(-5.0, -5.0,  5.0);
    let p5 = sat.add_point( 5.0, -5.0,  5.0);
    let p6 = sat.add_point( 5.0,  5.0,  5.0);
    let p7 = sat.add_point(-5.0,  5.0,  5.0);

    // 6 plane surfaces (one per face of the box)
    //   top (+Z), bottom (-Z), front (-Y), back (+Y), right (+X), left (-X)
    let surf_top    = sat.add_plane_surface([0.0, 0.0,  5.0], [0.0, 0.0,  1.0], [1.0, 0.0, 0.0]);
    let surf_bottom = sat.add_plane_surface([0.0, 0.0, -5.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]);
    let surf_front  = sat.add_plane_surface([0.0, -5.0, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]);
    let surf_back   = sat.add_plane_surface([0.0,  5.0, 0.0], [0.0,  1.0, 0.0], [1.0, 0.0, 0.0]);
    let surf_right  = sat.add_plane_surface([ 5.0, 0.0, 0.0], [ 1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    let surf_left   = sat.add_plane_surface([-5.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);

    // 12 straight curves (one per edge of the box)
    //  Bottom face edges (Z = -5):
    let crv_b0 = sat.add_straight_curve([-5.0, -5.0, -5.0], [ 1.0, 0.0, 0.0]); // p0→p1
    let crv_b1 = sat.add_straight_curve([ 5.0, -5.0, -5.0], [ 0.0, 1.0, 0.0]); // p1→p2
    let crv_b2 = sat.add_straight_curve([ 5.0,  5.0, -5.0], [-1.0, 0.0, 0.0]); // p2→p3
    let crv_b3 = sat.add_straight_curve([-5.0,  5.0, -5.0], [ 0.0,-1.0, 0.0]); // p3→p0
    //  Top face edges (Z = 5):
    let crv_t0 = sat.add_straight_curve([-5.0, -5.0,  5.0], [ 1.0, 0.0, 0.0]); // p4→p5
    let crv_t1 = sat.add_straight_curve([ 5.0, -5.0,  5.0], [ 0.0, 1.0, 0.0]); // p5→p6
    let crv_t2 = sat.add_straight_curve([ 5.0,  5.0,  5.0], [-1.0, 0.0, 0.0]); // p6→p7
    let crv_t3 = sat.add_straight_curve([-5.0,  5.0,  5.0], [ 0.0,-1.0, 0.0]); // p7→p4
    //  Vertical edges:
    let crv_v0 = sat.add_straight_curve([-5.0, -5.0, -5.0], [ 0.0, 0.0, 1.0]); // p0→p4
    let crv_v1 = sat.add_straight_curve([ 5.0, -5.0, -5.0], [ 0.0, 0.0, 1.0]); // p1→p5
    let crv_v2 = sat.add_straight_curve([ 5.0,  5.0, -5.0], [ 0.0, 0.0, 1.0]); // p2→p6
    let crv_v3 = sat.add_straight_curve([-5.0,  5.0, -5.0], [ 0.0, 0.0, 1.0]); // p3→p7

    // ────────────── Topology ────────────────────────────────────────

    // Vertices: 8 vertices referencing the 8 points.
    // We need to create them before edges, so we use NULL for the edge
    // pointer now. In a more advanced builder we'd patch these up.
    let v0 = sat.add_vertex(SatPointer::NULL, SatPointer::new(p0));
    let v1 = sat.add_vertex(SatPointer::NULL, SatPointer::new(p1));
    let v2 = sat.add_vertex(SatPointer::NULL, SatPointer::new(p2));
    let v3 = sat.add_vertex(SatPointer::NULL, SatPointer::new(p3));
    let v4 = sat.add_vertex(SatPointer::NULL, SatPointer::new(p4));
    let v5 = sat.add_vertex(SatPointer::NULL, SatPointer::new(p5));
    let v6 = sat.add_vertex(SatPointer::NULL, SatPointer::new(p6));
    let v7 = sat.add_vertex(SatPointer::NULL, SatPointer::new(p7));

    // Edges: 12 edges. Coedge pointer set to NULL (patched later by
    // referencing from coedges — ACIS resolves via the coedge→edge link).
    let e0  = sat.add_edge(SatPointer::new(v0), SatPointer::new(v1), SatPointer::NULL, SatPointer::new(crv_b0), Sense::Forward); // bottom: p0→p1
    let e1  = sat.add_edge(SatPointer::new(v1), SatPointer::new(v2), SatPointer::NULL, SatPointer::new(crv_b1), Sense::Forward); // bottom: p1→p2
    let e2  = sat.add_edge(SatPointer::new(v2), SatPointer::new(v3), SatPointer::NULL, SatPointer::new(crv_b2), Sense::Forward); // bottom: p2→p3
    let e3  = sat.add_edge(SatPointer::new(v3), SatPointer::new(v0), SatPointer::NULL, SatPointer::new(crv_b3), Sense::Forward); // bottom: p3→p0
    let e4  = sat.add_edge(SatPointer::new(v4), SatPointer::new(v5), SatPointer::NULL, SatPointer::new(crv_t0), Sense::Forward); // top: p4→p5
    let e5  = sat.add_edge(SatPointer::new(v5), SatPointer::new(v6), SatPointer::NULL, SatPointer::new(crv_t1), Sense::Forward); // top: p5→p6
    let e6  = sat.add_edge(SatPointer::new(v6), SatPointer::new(v7), SatPointer::NULL, SatPointer::new(crv_t2), Sense::Forward); // top: p6→p7
    let e7  = sat.add_edge(SatPointer::new(v7), SatPointer::new(v4), SatPointer::NULL, SatPointer::new(crv_t3), Sense::Forward); // top: p7→p4
    let e8  = sat.add_edge(SatPointer::new(v0), SatPointer::new(v4), SatPointer::NULL, SatPointer::new(crv_v0), Sense::Forward); // vert: p0→p4
    let e9  = sat.add_edge(SatPointer::new(v1), SatPointer::new(v5), SatPointer::NULL, SatPointer::new(crv_v1), Sense::Forward); // vert: p1→p5
    let e10 = sat.add_edge(SatPointer::new(v2), SatPointer::new(v6), SatPointer::NULL, SatPointer::new(crv_v2), Sense::Forward); // vert: p2→p6
    let e11 = sat.add_edge(SatPointer::new(v3), SatPointer::new(v7), SatPointer::NULL, SatPointer::new(crv_v3), Sense::Forward); // vert: p3→p7

    // ── Reserve indices for coedges, loops, faces, shell, lump ──────
    // We need to pre-calculate indices to set up circular references.
    //
    // Current next index = sat.records.len()
    let base = sat.records.len() as i32;

    // 6 faces × 4 coedges = 24 coedges (indices base..base+23)
    // 6 loops (indices base+24..base+29)
    // 6 faces (indices base+30..base+35)
    // 1 shell (index base+36)
    // 1 lump  (index base+37)
    let co_base     = base;       // 24 coedges: co_base + 0..23
    let loop_base   = base + 24;  // 6 loops:   loop_base + 0..5
    let face_base   = base + 30;  // 6 faces:   face_base + 0..5
    let shell_idx   = base + 36;
    let lump_idx    = base + 37;

    // Helper to make SatPointer from computed index
    let ptr = |offset: i32| SatPointer::new(offset);

    // ── Bottom face (Z = -5): normal = (0,0,-1)
    //    Loop: e0(rev) → e3(rev) → e2(rev) → e1(rev)
    //    coedges: co_base+0..3
    let co0 = co_base;
    let co1 = co_base + 1;
    let co2 = co_base + 2;
    let co3 = co_base + 3;
    sat.add_coedge(ptr(co1), ptr(co3), ptr(co0 + 24), ptr(e0), Sense::Reversed, ptr(loop_base));       // will be partner'd by front face
    sat.add_coedge(ptr(co2), ptr(co0), ptr(co1 + 24), ptr(e3), Sense::Reversed, ptr(loop_base));       // partner'd by left face
    sat.add_coedge(ptr(co3), ptr(co1), ptr(co2 + 24), ptr(e2), Sense::Reversed, ptr(loop_base));       // partner'd by back face
    sat.add_coedge(ptr(co0), ptr(co2), ptr(co3 + 24), ptr(e1), Sense::Reversed, ptr(loop_base));       // partner'd by right face

    // ── Top face (Z = +5): normal = (0,0,1)
    //    Loop: e4(fwd) → e5(fwd) → e6(fwd) → e7(fwd)
    //    coedges: co_base+4..7
    let co4 = co_base + 4;
    let co5 = co_base + 5;
    let co6 = co_base + 6;
    let co7 = co_base + 7;
    sat.add_coedge(ptr(co5), ptr(co7), ptr(co4 + 24), ptr(e4), Sense::Forward, ptr(loop_base + 1));    // partner'd by front face
    sat.add_coedge(ptr(co6), ptr(co4), ptr(co5 + 24), ptr(e5), Sense::Forward, ptr(loop_base + 1));    // partner'd by right face
    sat.add_coedge(ptr(co7), ptr(co5), ptr(co6 + 24), ptr(e6), Sense::Forward, ptr(loop_base + 1));    // partner'd by back face
    sat.add_coedge(ptr(co4), ptr(co6), ptr(co7 + 24), ptr(e7), Sense::Forward, ptr(loop_base + 1));    // partner'd by left face

    // ── Front face (Y = -5): normal = (0,-1,0)
    //    Loop: e0(fwd) → e9(fwd) → e4(rev) → e8(rev)
    //    coedges: co_base+8..11
    let co8  = co_base + 8;
    let co9  = co_base + 9;
    let co10 = co_base + 10;
    let co11 = co_base + 11;
    sat.add_coedge(ptr(co9),  ptr(co11), ptr(co0),          ptr(e0), Sense::Forward,  ptr(loop_base + 2)); // partner = bottom co0
    sat.add_coedge(ptr(co10), ptr(co8),  ptr(co9  + 24-8),  ptr(e9), Sense::Forward,  ptr(loop_base + 2)); // partner'd by right face
    sat.add_coedge(ptr(co11), ptr(co9),  ptr(co4),          ptr(e4), Sense::Reversed, ptr(loop_base + 2)); // partner = top co4
    sat.add_coedge(ptr(co8),  ptr(co10), ptr(co11 + 24-8),  ptr(e8), Sense::Reversed, ptr(loop_base + 2)); // partner'd by left face

    // ── Back face (Y = +5): normal = (0,1,0)
    //    Loop: e2(fwd) → e11(fwd) → e6(rev) → e10(rev)
    //    coedges: co_base+12..15
    let co12 = co_base + 12;
    let co13 = co_base + 13;
    let co14 = co_base + 14;
    let co15 = co_base + 15;
    sat.add_coedge(ptr(co13), ptr(co15), ptr(co2),         ptr(e2),  Sense::Forward,  ptr(loop_base + 3)); // partner = bottom co2
    sat.add_coedge(ptr(co14), ptr(co12), ptr(co13 + 24 - 12), ptr(e11), Sense::Forward,  ptr(loop_base + 3)); // partner'd by left face
    sat.add_coedge(ptr(co15), ptr(co13), ptr(co6),         ptr(e6),  Sense::Reversed, ptr(loop_base + 3)); // partner = top co6
    sat.add_coedge(ptr(co12), ptr(co14), ptr(co15 + 24 - 12), ptr(e10), Sense::Reversed, ptr(loop_base + 3)); // partner'd by right face

    // ── Right face (X = +5): normal = (1,0,0)
    //    Loop: e1(fwd) → e10(fwd) → e5(rev) → e9(rev)
    //    coedges: co_base+16..19
    let co16 = co_base + 16;
    let co17 = co_base + 17;
    let co18 = co_base + 18;
    let co19 = co_base + 19;
    sat.add_coedge(ptr(co17), ptr(co19), ptr(co3),  ptr(e1),  Sense::Forward,  ptr(loop_base + 4)); // partner = bottom co3
    sat.add_coedge(ptr(co18), ptr(co16), ptr(co15), ptr(e10), Sense::Forward,  ptr(loop_base + 4)); // partner = back co15
    sat.add_coedge(ptr(co19), ptr(co17), ptr(co5),  ptr(e5),  Sense::Reversed, ptr(loop_base + 4)); // partner = top co5
    sat.add_coedge(ptr(co16), ptr(co18), ptr(co9),  ptr(e9),  Sense::Reversed, ptr(loop_base + 4)); // partner = front co9

    // ── Left face (X = -5): normal = (-1,0,0)
    //    Loop: e3(fwd) → e8(fwd) → e7(rev) → e11(rev)
    //    coedges: co_base+20..23
    let co20 = co_base + 20;
    let co21 = co_base + 21;
    let co22 = co_base + 22;
    let co23 = co_base + 23;
    sat.add_coedge(ptr(co21), ptr(co23), ptr(co1),  ptr(e3),  Sense::Forward,  ptr(loop_base + 5)); // partner = bottom co1
    sat.add_coedge(ptr(co22), ptr(co20), ptr(co11), ptr(e8),  Sense::Forward,  ptr(loop_base + 5)); // partner = front co11
    sat.add_coedge(ptr(co23), ptr(co21), ptr(co7),  ptr(e7),  Sense::Reversed, ptr(loop_base + 5)); // partner = top co7
    sat.add_coedge(ptr(co20), ptr(co22), ptr(co13), ptr(e11), Sense::Reversed, ptr(loop_base + 5)); // partner = back co13

    // ── 6 Loops ─────────────────────────────────────────────────────
    sat.add_loop(SatPointer::NULL, ptr(co0),  ptr(face_base));           // bottom
    sat.add_loop(SatPointer::NULL, ptr(co4),  ptr(face_base + 1));       // top
    sat.add_loop(SatPointer::NULL, ptr(co8),  ptr(face_base + 2));       // front
    sat.add_loop(SatPointer::NULL, ptr(co12), ptr(face_base + 3));       // back
    sat.add_loop(SatPointer::NULL, ptr(co16), ptr(face_base + 4));       // right
    sat.add_loop(SatPointer::NULL, ptr(co20), ptr(face_base + 5));       // left

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
    if let Some(body_rec) = sat.record_mut(1) {
        body_rec.tokens[0] = SatToken::Pointer(ptr(lump_idx));
    }

    // Print the generated SAT text for inspection
    let sat_text = sat.to_sat_string();
    println!("=== Generated SAT data ({} bytes) ===", sat_text.len());
    println!("{}", sat_text);

    // Validate the document
    let errors = sat.validate();
    if !errors.is_empty() {
        println!("SAT validation warnings ({}):", errors.len());
        for e in &errors {
            println!("  - {:?}", e);
        }
    }

    // ── 2. Create a Solid3D entity ───────────────────────────────────
    let mut solid = Solid3D::new();
    solid.set_sat_document(&sat);
    solid.common.layer = "0".to_string();

    println!("\nSolid3D ACIS data size: {} bytes", solid.acis_size());
    println!("Solid3D has ACIS data: {}", solid.has_acis_data());

    // Verify roundtrip: parse the SAT back and check geometry
    if let Some(parsed) = solid.parse_sat() {
        println!("\nRoundtrip parse OK:");
        println!("  Bodies:   {}", parsed.bodies().len());
        println!("  Faces:    {}", parsed.faces().len());
        println!("  Edges:    {}", parsed.edges().len());
        println!("  Vertices: {}", parsed.vertices().len());
        println!("  Records:  {}", parsed.records.len());
    }

    // ── 3. Create a CadDocument and write to DXF ─────────────────────
    let mut doc = CadDocument::with_version(DxfVersion::AC1027); // R2013
    doc.add_entity(EntityType::Solid3D(solid))?;

    let output_path = "solid3d_box.dxf";
    let writer = DxfWriter::new(doc);
    writer.write_to_file(output_path)?;

    println!("\nDXF written to: {}", output_path);
    Ok(())
}
