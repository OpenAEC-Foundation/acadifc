use acadrust::{DwgReader, DwgWriter};
use acadrust::entities::EntityType;

fn count_entity(entity: &EntityType) -> &'static str {
    match entity {
        EntityType::LwPolyline(_) => "LwPolyline",
        EntityType::Line(_) => "Line",
        EntityType::Circle(_) => "Circle",
        EntityType::Arc(_) => "Arc",
        EntityType::MText(_) => "MText",
        EntityType::Text(_) => "Text",
        EntityType::Insert(_) => "Insert",
        EntityType::Point(_) => "Point",
        EntityType::Hatch(_) => "Hatch",
        EntityType::Ellipse(_) => "Ellipse",
        EntityType::Dimension(_) => "Dimension",
        EntityType::Solid(_) => "Solid",
        EntityType::Spline(_) => "Spline",
        EntityType::Polyline(_) => "Polyline",
        EntityType::Unknown(u) => {
            // Leak a string so we can return a &'static str for unknown types
            // This is fine for diagnostic use
            Box::leak(format!("Unknown({})", u.dxf_name).into_boxed_str())
        },
        _ => "Other",
    }
}

struct LwPolyStats {
    lwpoly_count: u32,
    garbage_count: u32,
    total_vertices: u32,
    zero_vertex_count: u32,
    max_vertices: usize,
}

fn analyze_lwpolylines(entities: impl Iterator<Item = impl std::borrow::Borrow<EntityType>>, label: &str) -> LwPolyStats {
    let mut stats = LwPolyStats {
        lwpoly_count: 0,
        garbage_count: 0,
        total_vertices: 0,
        zero_vertex_count: 0,
        max_vertices: 0,
    };

    for entity in entities {
        if let EntityType::LwPolyline(lw) = entity.borrow() {
            stats.lwpoly_count += 1;
            if lw.vertices.is_empty() {
                stats.zero_vertex_count += 1;
            }
            if lw.vertices.len() > stats.max_vertices {
                stats.max_vertices = lw.vertices.len();
            }
            for v in &lw.vertices {
                stats.total_vertices += 1;
                if v.location.x.abs() > 1e100 || v.location.y.abs() > 1e100
                   || v.location.x.is_nan() || v.location.y.is_nan()
                {
                    stats.garbage_count += 1;
                }
            }
        }
    }

    println!("\n--- {} ---", label);
    println!("Total LwPolylines: {}", stats.lwpoly_count);
    println!("Zero-vertex polylines: {}", stats.zero_vertex_count);
    println!("Max vertices in single polyline: {}", stats.max_vertices);
    println!("Total vertices: {}", stats.total_vertices);
    println!("Garbage vertices: {}", stats.garbage_count);
    if stats.garbage_count == 0 {
        println!("OK: No garbage coordinates!");
    } else {
        println!("FAIL: {} garbage vertices found!", stats.garbage_count);
    }
    stats
}

fn main() {
    let input_path = "tests/issue14/General.dwg";
    let output_path = "target/General_roundtrip.dwg";

    // ── Step 1: Read original ──────────────────────────────────────
    println!("=== STEP 1: Read original {} ===", input_path);
    let mut reader = DwgReader::from_file(input_path).expect("Failed to open DWG file");
    let doc = reader.read().expect("Failed to read document");

    let mut type_counts = std::collections::HashMap::new();
    for entity in doc.entities() {
        *type_counts.entry(count_entity(entity)).or_insert(0u32) += 1;
    }
    println!("Entity type counts:");
    for (name, count) in &type_counts {
        println!("  {}: {}", name, count);
    }

    let orig_stats = analyze_lwpolylines(doc.entities(), "Original read");

    // ── Step 2: Write roundtrip DWG ────────────────────────────────
    println!("\n=== STEP 2: Write to {} ===", output_path);
    DwgWriter::write_to_file(output_path, &doc).expect("Failed to write roundtrip DWG");
    let file_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);
    println!("Written {} bytes", file_size);

    // ── Step 3: Re-read roundtrip DWG ──────────────────────────────
    println!("\n=== STEP 3: Re-read {} ===", output_path);
    let mut reader2 = DwgReader::from_file(output_path).expect("Failed to open roundtrip DWG");
    let doc2 = reader2.read().expect("Failed to read roundtrip DWG");

    let mut type_counts2 = std::collections::HashMap::new();
    for entity in doc2.entities() {
        *type_counts2.entry(count_entity(entity)).or_insert(0u32) += 1;
    }
    println!("Entity type counts after roundtrip:");
    for (name, count) in &type_counts2 {
        println!("  {}: {}", name, count);
    }

    let rt_stats = analyze_lwpolylines(doc2.entities(), "Roundtrip read");

    // ── Step 4: Compare ────────────────────────────────────────────
    println!("\n=== COMPARISON ===");
    let mut ok = true;

    if orig_stats.lwpoly_count != rt_stats.lwpoly_count {
        println!("MISMATCH: LwPolyline count {} -> {}", orig_stats.lwpoly_count, rt_stats.lwpoly_count);
        ok = false;
    } else {
        println!("LwPolyline count: {} (match)", orig_stats.lwpoly_count);
    }

    if orig_stats.total_vertices != rt_stats.total_vertices {
        println!("MISMATCH: Total vertices {} -> {}", orig_stats.total_vertices, rt_stats.total_vertices);
        ok = false;
    } else {
        println!("Total vertices: {} (match)", orig_stats.total_vertices);
    }

    if rt_stats.garbage_count > 0 {
        println!("FAIL: Roundtrip has {} garbage vertices!", rt_stats.garbage_count);
        ok = false;
    }

    if orig_stats.garbage_count > 0 {
        println!("FAIL: Original read has {} garbage vertices!", orig_stats.garbage_count);
        ok = false;
    }

    // Compare total entity counts
    let orig_total: u32 = type_counts.values().sum();
    let rt_total: u32 = type_counts2.values().sum();
    println!("Total entities: {} -> {}", orig_total, rt_total);

    if ok {
        println!("\nSUCCESS: Roundtrip verified — all LWPOLYLINE data intact!");
    } else {
        println!("\nFAILURE: Roundtrip verification found issues.");
        std::process::exit(1);
    }
}
