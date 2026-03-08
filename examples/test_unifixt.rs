use acadrust::{CadDocument, DxfReader};
fn main() {
    let doc = DxfReader::from_file("examples/sat v7 samples/UNIFIXT.dxf").unwrap().read().unwrap();
    println!("Entities: {}", doc.entity_count());
    for e in doc.entities() {
        if let acadrust::EntityType::Solid3D(s) = e {
            println!("3DSOLID: layer={}, acis_size={}", s.common.layer, s.acis_size());
            if let Some(parsed) = s.parse_sat() {
                println!("  Parsed: bodies={}, faces={}, edges={}, vertices={}", 
                    parsed.bodies().len(), parsed.faces().len(), 
                    parsed.edges().len(), parsed.vertices().len());
            } else {
                println!("  SAT data first 200 chars: {}", &s.acis_data.sat_data[..200.min(s.acis_data.sat_data.len())]);
            }
        }
    }
}
