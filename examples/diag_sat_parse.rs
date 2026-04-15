use acadrust::DwgReader;
use std::io::Cursor;

fn main() {
    for f in &["samplewardrobe.dwg", "samplekitchen_simple.dwg", "samplekitchen.dwg"] {
        let path = format!("tests/roundtrip/{}", f);
        let data = std::fs::read(&path).unwrap();
        let mut reader = DwgReader::from_stream(Cursor::new(&data));
        let doc = reader.read().unwrap();
        for ent in doc.entities() {
            if let acadrust::entities::EntityType::Solid3D(s) = ent {
                println!("{}: SAT len={} SAB len={} version={:?}", f, s.acis_data.sat_data.len(), s.acis_data.sab_data.len(), s.acis_data.version);
                break;
            }
        }
    }
}
