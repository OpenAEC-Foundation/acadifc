use acadrust::{DxfReader, DxfWriter, EntityType};

fn main() {
    // Test 1: Read our generated DXF
    println!("=== Reading solid3d_box.dxf ===");
    let doc = DxfReader::from_file("solid3d_box.dxf").unwrap().read().unwrap();
    let entities: Vec<_> = doc.entities().collect();
    println!("Entities: {}", entities.len());

    for e in &entities {
        if let EntityType::Solid3D(solid) = e {
            println!("Found 3DSOLID");
            println!("  ACIS version: {:?}", solid.acis_data.version);
            println!("  SAT data len: {}", solid.acis_data.sat_data.len());
            if let Some(parsed) = solid.parse_sat() {
                println!("  Parse OK: {} records", parsed.records.len());
            } else {
                println!("  Parse FAILED");
            }
        }
    }

    // Test 2: Read reference DXF and write it back
    println!("\n=== Reading + roundtripping UNIFIXT.dxf ===");
    let ref_doc = DxfReader::from_file("examples/sat v7 samples/UNIFIXT.dxf").unwrap().read().unwrap();
    let ref_entities: Vec<_> = ref_doc.entities().collect();
    println!("Reference entities: {}", ref_entities.len());
    
    for e in &ref_entities {
        if let EntityType::Solid3D(solid) = e {
            println!("Found reference 3DSOLID");
            println!("  ACIS version: {:?}", solid.acis_data.version);
            println!("  SAT data len: {}", solid.acis_data.sat_data.len());
            let sat = &solid.acis_data.sat_data;
            println!("  SAT first 100: {:?}", &sat[..sat.len().min(100)]);
            if let Some(parsed) = solid.parse_sat() {
                println!("  Parse OK: {} records", parsed.records.len());
            } else {
                println!("  Parse FAILED");
            }
        }
    }

    // Write back the reference as a roundtrip test
    let writer = DxfWriter::new(ref_doc);
    writer.write_to_file("unifixt_roundtrip.dxf").unwrap();
    println!("\nWrote unifixt_roundtrip.dxf");
}
