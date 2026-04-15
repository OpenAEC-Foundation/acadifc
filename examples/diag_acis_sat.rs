/// Diagnostic: dump SAT text from 3dSolid entities
use acadrust::io::dwg::DwgReader;
use acadrust::entities::EntityType;
use std::io::Cursor;

fn main() {
    let path = std::env::args().nth(1).expect("Usage: diag_acis_sat <file.dwg>");
    let data = std::fs::read(&path).unwrap();
    let mut reader = DwgReader::from_stream(Cursor::new(&data));
    let doc = reader.read().unwrap();

    let mut count = 0;
    for entity in doc.entities() {
        let (acis, handle, name) = match entity {
            EntityType::Solid3D(e) => (&e.acis_data, e.common.handle, "3DSOLID"),
            EntityType::Region(e) => (&e.acis_data, e.common.handle, "REGION"),
            EntityType::Body(e) => (&e.acis_data, e.common.handle, "BODY"),
            _ => continue,
        };
        count += 1;
        println!("=== {} handle=0x{:X} ===", name, handle.value());
        println!("  sat_data len={}, sab_data len={}, is_binary={}", acis.sat_data.len(), acis.sab_data.len(), acis.is_binary);
        
        if !acis.sat_data.is_empty() {
            println!("  SAT text (first 500 chars):");
            let preview: String = acis.sat_data.chars().take(500).collect();
            println!("{}", preview);
        }
        
        if !acis.sab_data.is_empty() && acis.sat_data.is_empty() {
            // Try SAB→SAT conversion
            match acadrust::entities::acis::SabReader::read(&acis.sab_data) {
                Ok(sat_doc) => {
                    let sat_text = sat_doc.to_sat_string();
                    println!("  SAB→SAT conversion OK, {} chars", sat_text.len());
                    let preview: String = sat_text.chars().take(500).collect();
                    println!("{}", preview);
                }
                Err(e) => {
                    println!("  SAB→SAT conversion FAILED: {}", e);
                    println!("  First 32 bytes: {:?}", &acis.sab_data[..acis.sab_data.len().min(32)]);
                }
            }
        }
        println!();
    }
    println!("Total ACIS entities: {}", count);
}
