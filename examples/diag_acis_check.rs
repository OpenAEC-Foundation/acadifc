//! Diagnostic: Check ACIS data sizes for 3DSOLIDs after roundtrip.
//! Usage: cargo run --example diag_acis_check

use acadrust::io::dwg::DwgReader;
use acadrust::entities::EntityType;

fn main() {
    let input = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/roundtrip/samplekitchen.dwg".to_string());

    let mut reader = DwgReader::from_file(&input).expect("open");
    let doc = reader.read().expect("read");

    let mut count = 0;
    let mut total_sat = 0usize;
    let mut total_sab = 0usize;
    let mut empty_count = 0;

    for entity in doc.entities() {
        match entity {
            EntityType::Solid3D(e) => {
                count += 1;
                let sat_len = e.acis_data.sat_data.len();
                let sab_len = e.acis_data.sab_data.len();
                total_sat += sat_len;
                total_sab += sab_len;
                let has_data = e.acis_data.has_data();
                let wires_count = e.wires.len();
                let sils_count = e.silhouettes.len();
                if !has_data { empty_count += 1; }
                if count <= 10 || !has_data {
                    println!("3DSOLID #{} handle=0x{:X}: is_binary={} sat={} sab={} wires={} sils={} has_data={}",
                        count, e.common.handle.value(), e.acis_data.is_binary,
                        sat_len, sab_len, wires_count, sils_count, has_data);
                    if sat_len > 0 {
                        let preview = &e.acis_data.sat_data[..sat_len.min(200)];
                        println!("  SAT preview: {}", preview.replace('\n', "\\n").replace('\r', ""));
                    }
                    if sab_len > 0 {
                        println!("  SAB bytes: {:02X?}", &e.acis_data.sab_data[..sab_len.min(32)]);
                    }
                    println!("  point_of_ref: {:?}", e.point_of_reference);
                }
            }
            EntityType::Region(e) => {
                count += 1;
                // Just count regions
            }
            _ => {}
        }
    }

    println!("\nTotal 3DSOLID/Region entities: {}", count);
    println!("Empty ACIS: {}", empty_count);
    println!("Total SAT text bytes: {}", total_sat);
    println!("Total SAB binary bytes: {}", total_sab);
}
