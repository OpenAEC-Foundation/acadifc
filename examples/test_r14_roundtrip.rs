/// Round-trip test: write an R14 DWG with a circle, read it back, check extents and entities
use acadrust::entities::*;
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgWriter};
use acadrust::io::dwg::DwgReader;

fn main() {
    // Create document
    let version = DxfVersion::AC1014;
    let mut doc = CadDocument::with_version(version);
    let circle = Circle::from_coords(50.0, 50.0, 0.0, 25.0);
    doc.add_entity(EntityType::Circle(circle)).unwrap();
    
    // Write to file
    let path = "test_r14_roundtrip.dwg";
    DwgWriter::write_to_file(path, &doc).unwrap();
    println!("Written {}", path);
    
    // Read back
    match DwgReader::from_file(path) {
        Ok(mut reader) => {
            match reader.read() {
                Ok(read_doc) => {
                    println!("Read back successfully!");
                    println!("Version: {:?}", read_doc.version);
                    
                    // Check header extents
                    println!("EXTMIN: ({},{},{})", 
                        read_doc.header.model_space_extents_min.x,
                        read_doc.header.model_space_extents_min.y,
                        read_doc.header.model_space_extents_min.z);
                    println!("EXTMAX: ({},{},{})", 
                        read_doc.header.model_space_extents_max.x,
                        read_doc.header.model_space_extents_max.y,
                        read_doc.header.model_space_extents_max.z);
                    
                    // Check VPorts
                    for vp in read_doc.vports.iter() {
                        println!("VPort '{}': center=({},{}), height={}, aspect={}", 
                            vp.name, vp.view_center.x, vp.view_center.y, vp.view_height, vp.aspect_ratio);
                    }
                    
                    // Check entities
                    println!("Entity count: {}", read_doc.entity_count());
                    for e in read_doc.entities() {
                        println!("  {:?} handle={:?}", e.as_entity().entity_type(), e.common().handle);
                    }
                    
                    // Check block records
                    if let Some(ms) = read_doc.block_records.get("*Model_Space") {
                        println!("*Model_Space entities: {}", ms.entities.len());
                        for e in &ms.entities {
                            println!("  {} handle={:?}", e.as_entity().entity_type(), e.common().handle);
                        }
                    }
                },
                Err(e) => println!("Read error: {:?}", e),
            }
        },
        Err(e) => println!("Open error: {:?}", e),
    }

    // Cleanup
    let _ = std::fs::remove_file(path);
}
