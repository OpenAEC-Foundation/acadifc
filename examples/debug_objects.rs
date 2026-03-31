use acadrust::DwgReader;
use acadrust::io::dwg::dwg_stream_readers::handle_reader;
use std::collections::HashSet;

fn main() {
    // Read roundtrip file raw
    let roundtrip_path = "target/General_roundtrip.dwg";
    
    // First, read it normally to get handle mapping
    let mut reader = DwgReader::from_file(roundtrip_path).expect("open roundtrip");
    let doc = reader.read().expect("read roundtrip");
    
    // Also re-read to get raw section data
    let mut reader2 = DwgReader::from_file(roundtrip_path).expect("open roundtrip 2");
    
    // Check total entities
    let total = doc.entities().count();
    println!("Total entities in roundtrip: {}", total);
    
    // Check the non-entity objects that BricsCAD reported errors for
    let problem_handles: Vec<u64> = vec![
        0xD8, 0xE5, // AcDbMLeaderStyle
        0xD7,       // AcDbDictionary
        0x66,       // AcDbDictionary (named objects dict?)
        0x89, 0x1B7, 0x1B8, // Dictionary entries
        0x18,       // AcDbMlineStyle
    ];
    
    for h in &problem_handles {
        let handle = acadrust::types::Handle::new(*h);
        if let Some(obj) = doc.objects.get(&handle) {
            println!("Object {:#X}: type={}", h, obj.type_name());
        } else if let Some(entity) = doc.get_entity(handle) {
            println!("Entity {:#X}: type={:?}", h, std::mem::discriminant(entity));
        } else {
            println!("Object {:#X}: NOT FOUND in document!", h);
        }
    }
    
    // Check if we're writing mline style correctly
    if let Some(mline_style) = doc.mline_styles.get("Standard") {
        println!("\nMLineStyle 'Standard': handle={:#X}, elements={}", 
            mline_style.handle.value(), mline_style.elements.len());
    } else {
        println!("\nMLineStyle 'Standard': NOT FOUND");
    }
    
    // Check mleader styles  
    for (name, style) in &doc.mleader_styles {
        println!("MLeaderStyle '{}': handle={:#X}", name, style.handle.value());
    }
    
    // Show which dictionaries we have
    println!("\n=== Named objects dictionary entries ===");
    if let Some(root_dict) = doc.objects.values().find(|o| o.type_name() == "Dictionary" && o.common().owner_handle.is_null()) {
        println!("Root dictionary: {:#X}", root_dict.common().handle.value());
    }
    
    // Check first 10 and last 10 handle mappings (after re-read)
    println!("\n=== Checking all LwPolylines can be re-read ===");
    let failing: HashSet<u64> = [
        0x241B6, 0x241B9, 0x241BF, 0x241CE, 0x241E6, 0x2420D, 0x24211,
        0x24217, 0x2421C, 0x2421F, 0x24223, 0x24226, 0x2422E, 0x24237,
        0x24345,
    ].iter().copied().collect();
    
    // Read roundtrip and verify entity handles are in the output
    let mut lw_count = 0;
    let mut lw_fail_found = 0;
    for entity in doc.entities() {
        if matches!(entity, acadrust::entities::EntityType::LwPolyline(_)) {
            lw_count += 1;
            let h = entity.common().handle.value();
            if failing.contains(&h) {
                lw_fail_found += 1;
            }
        }
    }
    println!("LwPolylines total: {}, failing handles found: {}", lw_count, lw_fail_found);
}
