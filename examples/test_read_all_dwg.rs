/// Comprehensive DWG read-back test: reads every generated DWG file and reports results.
///
/// For each file, it checks:
///   1. File opens without error
///   2. Document reads without error
///   3. At least one entity is present (except VIEWPORT which may be paper-space only)
///   4. Entity type matches the expected type from the filename
///   5. Tables are populated (layers, linetypes, etc.)
///
/// Run after `cargo run --example gen_all_entities_all_versions`
use acadrust::io::dwg::DwgReader;

const VERSIONS: &[&str] = &[
    "AC1012", "AC1014", "AC1015", "AC1018",
    "AC1021", "AC1024", "AC1027", "AC1032",
];

fn main() {
    let root = "target/entities_dwg";
    let mut total = 0u32;
    let mut ok = 0u32;
    let mut fail = 0u32;
    let mut errors: Vec<String> = Vec::new();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║         COMPREHENSIVE DWG READ-BACK TEST                    ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    for ver in VERSIONS {
        let dir = format!("{}/{}", root, ver);
        let dir_path = std::path::Path::new(&dir);
        if !dir_path.exists() {
            println!("⚠  {} — directory not found, skipping", ver);
            continue;
        }

        let mut ver_ok = 0u32;
        let mut ver_fail = 0u32;
        let mut ver_files: Vec<std::path::PathBuf> = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "dwg").unwrap_or(false) {
                    ver_files.push(path);
                }
            }
        }
        ver_files.sort();

        println!("── {} ({} files) ──────────────────────────────────", ver, ver_files.len());

        for path in &ver_files {
            total += 1;
            let fname = path.file_name().unwrap().to_string_lossy().to_string();
            let entity_type = extract_entity_type(&fname);

            match read_and_validate(path, &entity_type) {
                Ok(summary) => {
                    println!("  ✓ {} — {}", fname, summary);
                    ok += 1;
                    ver_ok += 1;
                }
                Err(e) => {
                    let msg = format!("{}: {}", fname, e);
                    println!("  ✗ {}", msg);
                    errors.push(msg);
                    fail += 1;
                    ver_fail += 1;
                }
            }
        }

        println!("  ── {} summary: {} OK, {} FAIL\n", ver, ver_ok, ver_fail);
    }

    // Final summary
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  TOTAL: {} files tested — {} OK, {} FAIL                 ║", total, ok, fail);
    println!("╚══════════════════════════════════════════════════════════════╝");

    if !errors.is_empty() {
        println!("\n── FAILURES ────────────────────────────────────────────────");
        for e in &errors {
            println!("  ✗ {}", e);
        }
    }

    if fail > 0 {
        std::process::exit(1);
    }
}

fn read_and_validate(path: &std::path::Path, expected_type: &str) -> Result<String, String> {
    // Step 1: Open
    let mut reader = DwgReader::from_file(path)
        .map_err(|e| format!("open failed: {}", e))?;

    // Step 2: Read
    let doc = reader.read()
        .map_err(|e| format!("read failed: {}", e))?;

    // Step 3: Collect info
    let layer_count = doc.layers.len();
    let linetype_count = doc.line_types.len();
    let entity_count = doc.entity_count();

    // Step 4: Check tables exist
    if layer_count == 0 {
        return Err("no layers found".to_string());
    }
    if linetype_count == 0 {
        return Err("no linetypes found".to_string());
    }

    // Step 5: Check layer "0" exists
    let has_layer0 = doc.layers.iter().any(|l| l.name == "0");
    if !has_layer0 {
        return Err("layer '0' not found".to_string());
    }

    // Step 6: Check entities (VIEWPORT may have 0 in model space)
    if entity_count == 0 && expected_type != "VIEWPORT" {
        return Err(format!("no entities found (expected {})", expected_type));
    }

    // Step 7: Verify entity types match
    let entity_types: Vec<String> = doc.entities()
        .map(|e| get_entity_type_name(e).to_uppercase())
        .collect();

    let type_matched = if expected_type == "HATCH_LINES" || expected_type == "HATCH_SOLID" {
        entity_types.iter().any(|t: &String| t.contains("HATCH"))
    } else if expected_type == "DIMENSION" {
        entity_types.iter().any(|t: &String| t.contains("DIMENSION"))
    } else if expected_type == "POLYLINE2D" {
        entity_types.iter().any(|t: &String| t.contains("POLYLINE") || t.contains("LWPOLYLINE"))
    } else if expected_type == "POLYLINE3D" {
        entity_types.iter().any(|t: &String| t.contains("POLYLINE"))
    } else if expected_type == "POLYFACE" {
        entity_types.iter().any(|t: &String| t.contains("POLYFACE") || t.contains("POLYLINE"))
    } else if expected_type == "VIEWPORT" {
        true // viewport may not appear as a model-space entity
    } else if expected_type == "INSERT" {
        entity_types.iter().any(|t: &String| t.contains("INSERT"))
    } else if expected_type == "MULTILEADER" {
        entity_types.iter().any(|t: &String| t.contains("MULTILEADER") || t.contains("MULTI"))
    } else {
        entity_types.iter().any(|t: &String| t.contains(expected_type))
    };

    if !type_matched && entity_count > 0 {
        return Err(format!(
            "expected '{}' but found: [{}]",
            expected_type,
            entity_types.join(", ")
        ));
    }

    // Step 8: Check block records
    let model_space = doc.block_records.get("*Model_Space");
    let paper_space = doc.block_records.get("*Paper_Space");
    let block_info = format!(
        "MS={} PS={}",
        model_space.map(|b| b.entities.len()).unwrap_or(0),
        paper_space.map(|b| b.entities.len()).unwrap_or(0),
    );

    // Step 9: Check VPorts
    let vport_count = doc.vports.len();

    Ok(format!(
        "{} entities, {} layers, {} LTs, {} VPs, {}",
        entity_count, layer_count, linetype_count, vport_count, block_info
    ))
}

fn extract_entity_type(filename: &str) -> String {
    // filename format: entity_AC1014_CIRCLE.dwg
    let stem = filename.trim_end_matches(".dwg");
    // Find the part after the second underscore (version_ENTITYTYPE)
    let parts: Vec<&str> = stem.splitn(3, '_').collect();
    if parts.len() >= 3 {
        parts[2].to_uppercase()
    } else {
        stem.to_uppercase()
    }
}

fn get_entity_type_name(e: &acadrust::entities::EntityType) -> &'static str {
    use acadrust::entities::EntityType;
    match e {
        EntityType::Point(_) => "POINT",
        EntityType::Line(_) => "LINE",
        EntityType::Circle(_) => "CIRCLE",
        EntityType::Arc(_) => "ARC",
        EntityType::Ellipse(_) => "ELLIPSE",
        EntityType::Polyline(_) => "POLYLINE",
        EntityType::Polyline2D(_) => "POLYLINE2D",
        EntityType::Polyline3D(_) => "POLYLINE3D",
        EntityType::LwPolyline(_) => "LWPOLYLINE",
        EntityType::Text(_) => "TEXT",
        EntityType::MText(_) => "MTEXT",
        EntityType::Spline(_) => "SPLINE",
        EntityType::Dimension(_) => "DIMENSION",
        EntityType::Hatch(_) => "HATCH",
        EntityType::Solid(_) => "SOLID",
        EntityType::Face3D(_) => "FACE3D",
        EntityType::Insert(_) => "INSERT",
        EntityType::Block(_) => "BLOCK",
        EntityType::BlockEnd(_) => "BLOCKEND",
        EntityType::Ray(_) => "RAY",
        EntityType::XLine(_) => "XLINE",
        EntityType::Viewport(_) => "VIEWPORT",
        EntityType::AttributeDefinition(_) => "ATTDEF",
        EntityType::AttributeEntity(_) => "ATTRIB",
        EntityType::Leader(_) => "LEADER",
        EntityType::MultiLeader(_) => "MULTILEADER",
        EntityType::MLine(_) => "MLINE",
        EntityType::Mesh(_) => "MESH",
        EntityType::RasterImage(_) => "IMAGE",
        EntityType::Solid3D(_) => "SOLID3D",
        EntityType::Region(_) => "REGION",
        EntityType::Body(_) => "BODY",
        EntityType::Table(_) => "TABLE",
        EntityType::Tolerance(_) => "TOLERANCE",
        EntityType::PolyfaceMesh(_) => "POLYFACEMESH",
        EntityType::Wipeout(_) => "WIPEOUT",
        EntityType::Shape(_) => "SHAPE",
        EntityType::Underlay(_) => "UNDERLAY",
        EntityType::Seqend(_) => "SEQEND",
        EntityType::Ole2Frame(_) => "OLE2FRAME",
        EntityType::PolygonMesh(_) => "POLYGONMESH",
        EntityType::Unknown(_) => "UNKNOWN",
    }
}
