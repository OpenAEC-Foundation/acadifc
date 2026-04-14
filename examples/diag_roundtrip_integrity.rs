//! Roundtrip integrity diagnostic for samplekitchen.dwg
//!
//! Tests whether the DWG writer produces output that the DWG reader can parse back.
//! Then compares entity/object counts to detect data loss.
//!
//! Usage: cargo run --example diag_roundtrip_integrity

use std::io::Cursor;

use acadrust::entities::EntityType;
use acadrust::io::dwg::{DwgReader, DwgWriter};

fn entity_type_name(e: &EntityType) -> &'static str {
    match e {
        EntityType::Point(_) => "Point",
        EntityType::Line(_) => "Line",
        EntityType::Circle(_) => "Circle",
        EntityType::Arc(_) => "Arc",
        EntityType::Ellipse(_) => "Ellipse",
        EntityType::Text(_) => "Text",
        EntityType::MText(_) => "MText",
        EntityType::Insert(_) => "Insert",
        EntityType::LwPolyline(_) => "LwPolyline",
        EntityType::Polyline(_) => "Polyline",
        EntityType::Polyline3D(_) => "Polyline3D",
        EntityType::PolyfaceMesh(_) => "PolyfaceMesh",
        EntityType::PolygonMesh(_) => "PolygonMesh",
        EntityType::Solid(_) => "Solid",
        EntityType::Solid3D(_) => "Solid3D",
        EntityType::Face3D(_) => "Face3D",
        EntityType::Spline(_) => "Spline",
        EntityType::Hatch(_) => "Hatch",
        EntityType::Dimension(_) => "Dimension",
        EntityType::Leader(_) => "Leader",
        EntityType::MultiLeader(_) => "MultiLeader",
        EntityType::Mesh(_) => "Mesh",
        EntityType::RasterImage(_) => "RasterImage",
        EntityType::Viewport(_) => "Viewport",
        EntityType::Ray(_) => "Ray",
        EntityType::XLine(_) => "XLine",
        EntityType::MLine(_) => "MLine",
        EntityType::Tolerance(_) => "Tolerance",
        EntityType::Shape(_) => "Shape",
        EntityType::Wipeout(_) => "Wipeout",
        EntityType::Ole2Frame(_) => "Ole2Frame",
        EntityType::AttributeDefinition(_) => "AttributeDefinition",
        EntityType::Block(_) => "Block",
        EntityType::BlockEnd(_) => "BlockEnd",
        _ => "Other",
    }
}

fn count_sub_entities(entities: impl Iterator<Item = impl std::borrow::Borrow<EntityType>>) -> (usize, usize, usize) {
    let mut vertices = 0usize;
    let mut faces = 0usize;
    let mut seqend = 0usize;
    for e in entities {
        let e = e.borrow();
        if let EntityType::PolyfaceMesh(pf) = e {
            vertices += pf.vertices.len();
            faces += pf.faces.len();
            seqend += 1;
        }
        if let EntityType::PolygonMesh(pm) = e {
            vertices += pm.vertices.len();
            seqend += 1;
        }
        if let EntityType::Polyline(pl) = e {
            vertices += pl.vertices.len();
            seqend += 1;
        }
        if let EntityType::Polyline3D(pl) = e {
            vertices += pl.vertices.len();
            seqend += 1;
        }
    }
    (vertices, faces, seqend)
}

fn print_doc_summary(label: &str, doc: &acadrust::CadDocument) {
    let entities: Vec<&EntityType> = doc.entities().collect();
    let objects = doc.objects.len();
    let (verts, faces, seqend) = count_sub_entities(entities.iter().copied());

    println!("{}:", label);
    println!("  Entities: {}", entities.len());
    println!("  Objects:  {}", objects);
    println!("  Sub-entities: {} vertices, {} faces, {} seqend", verts, faces, seqend);

    let mut type_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for e in &entities {
        *type_counts.entry(entity_type_name(e)).or_default() += 1;
    }
    let mut sorted: Vec<_> = type_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (name, count) in &sorted {
        println!("    {}: {}", name, count);
    }

}

fn main() {
    let input = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/roundtrip/samplekitchen.dwg".to_string());

    println!("=== Roundtrip Integrity Diagnostic ===");
    println!("Input: {}\n", input);

    // Step 1: Read original
    let doc = {
        let mut reader = DwgReader::from_file(&input).expect("Failed to open input");
        reader.read().expect("Failed to read input")
    };
    println!("  Version: {:?}", doc.version);
    let orig_ent_count = doc.entities().count();
    let orig_obj_count = doc.objects.len();
    let (orig_verts, orig_faces, _) = count_sub_entities(doc.entities());
    print_doc_summary("ORIGINAL", &doc);

    // Step 2: Write to file (so we can also open in AutoCAD)
    let out_path = "target/diag_rt_integrity.dwg";
    println!("\nWriting DWG to {}...", out_path);
    DwgWriter::write_to_file(out_path, &doc).expect("Failed to write DWG");
    let file_size = std::fs::metadata(out_path).map(|m| m.len()).unwrap_or(0);
    println!("  Output size: {} bytes ({:.1} KB)", file_size, file_size as f64 / 1024.0);

    // Also write to memory buffer for read-back test
    let mut buf = Vec::new();
    DwgWriter::write_to_writer(Cursor::new(&mut buf), &doc).expect("Failed to write DWG to mem");

    // Step 3: Read back from buffer
    println!("\nReading back from memory...");
    let doc2 = {
        let mut reader = DwgReader::from_stream(Cursor::new(&buf));
        match reader.read() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("  FAILED to read back: {}", e);
                return;
            }
        }
    };

    println!();
    let rt_ent_count = doc2.entities().count();
    let rt_obj_count = doc2.objects.len();
    let (rt_verts, rt_faces, _) = count_sub_entities(doc2.entities());
    print_doc_summary("ROUNDTRIPPED", &doc2);

    // Step 4: Compare
    println!("\n=== COMPARISON ===");
    println!("  Entity delta: {:+}", rt_ent_count as i64 - orig_ent_count as i64);
    println!("  Object delta: {:+}", rt_obj_count as i64 - orig_obj_count as i64);
    println!("  Vertex delta: {:+}", rt_verts as i64 - orig_verts as i64);
    println!("  Face delta:   {:+}", rt_faces as i64 - orig_faces as i64);

    if rt_ent_count == orig_ent_count && rt_obj_count == orig_obj_count {
        println!("\n  OK: LOSSLESS roundtrip (count-level)");
    } else {
        println!("\n  DATA LOSS detected");
    }
}
