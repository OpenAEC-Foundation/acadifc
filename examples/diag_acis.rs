/// Check ACIS data presence in Solid3D entities after reading a DWG file.

use acadrust::io::dwg::DwgReader;
use std::io::Cursor;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        String::from(r"tests\roundtrip\samplekitchen.dwg")
    });

    let bytes = std::fs::read(&path).expect("Failed to read file");
    
    // First pass: check sections
    {
        let mut reader = DwgReader::from_stream(Cursor::new(&bytes));
        let info = reader.read_file_header().expect("Failed to read header");
        println!("=== Sections in {} ===", path);
        for sd in &info.section_descriptors {
            println!("  {} (encoding={}, pages={}, size={})",
                sd.name, sd.encoding, sd.pages.len(), sd.decompressed_size);
        }
    }
    
    // Second pass: read document
    let mut reader = DwgReader::from_stream(Cursor::new(&bytes));
    let doc = reader.read().expect("Failed to read DWG");

    let mut solid3d_count = 0;
    let mut solid3d_has_data = 0;
    let mut solid3d_has_sab = 0;
    let mut solid3d_has_sat = 0;
    let mut solid3d_empty: Vec<acadrust::types::Handle> = Vec::new();
    let mut solid3d_with_data: Vec<(acadrust::types::Handle, usize, usize)> = Vec::new();

    let mut region_count = 0;
    let mut region_has_data = 0;
    let mut body_count = 0;
    let mut body_has_data = 0;

    for entity in doc.entities() {
        match entity {
            acadrust::entities::EntityType::Solid3D(e) => {
                solid3d_count += 1;
                let handle = e.common.handle;
                if e.acis_data.has_data() {
                    solid3d_has_data += 1;
                    if !e.acis_data.sab_data.is_empty() {
                        solid3d_has_sab += 1;
                    }
                    if !e.acis_data.sat_data.is_empty() {
                        solid3d_has_sat += 1;
                    }
                    solid3d_with_data.push((handle, e.acis_data.sab_data.len(), e.acis_data.sat_data.len()));
                } else {
                    solid3d_empty.push(handle);
                }
            }
            acadrust::entities::EntityType::Region(e) => {
                region_count += 1;
                if e.acis_data.has_data() { region_has_data += 1; }
            }
            acadrust::entities::EntityType::Body(e) => {
                body_count += 1;
                if e.acis_data.has_data() { body_has_data += 1; }
            }
            _ => {}
        }
    }

    println!("=== ACIS Data Status: {} ===", path);
    println!("Solid3D: {} total, {} with data ({} SAB, {} SAT), {} empty",
        solid3d_count, solid3d_has_data, solid3d_has_sab, solid3d_has_sat,
        solid3d_count - solid3d_has_data);
    println!("Region: {} total, {} with data", region_count, region_has_data);
    println!("Body: {} total, {} with data", body_count, body_has_data);

    if !solid3d_empty.is_empty() {
        println!("\nEmpty Solid3D handles:");
        for h in &solid3d_empty {
            println!("  {:#X}", h.value());
        }
    }

    println!("\nSolid3D with data (first 20):");
    for (h, sab_len, sat_len) in solid3d_with_data.iter().take(20) {
        println!("  {:#X}: sab={} sat={}", h.value(), sab_len, sat_len);
    }
}
