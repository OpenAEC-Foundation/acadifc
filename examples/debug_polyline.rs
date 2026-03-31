use acadrust::{DwgReader};
use acadrust::entities::EntityType;

fn main() {
    let input_path = "tests/issue14/General.dwg";
    let mut reader = DwgReader::from_file(input_path).expect("open");
    let doc = reader.read().expect("read");

    // Check these specific handles
    let target_handles: Vec<u64> = vec![
        0x2432A, // NOT reported as error (control)
        0x2432D, 0x24330, 0x24333, 0x24336, 0x24339, 0x2433C, 0x2433F, 0x24342, 0x24345,
    ];

    for &h in &target_handles {
        let handle = acadrust::types::Handle::new(h);
        if let Some(entity) = doc.get_entity(handle) {
            let c = entity.common();
            if let EntityType::LwPolyline(lw) = entity {
                println!("Handle {:#X}: verts={} closed={} const_w={} elev={} thick={} normal=({:.1},{:.1},{:.1})",
                    h, lw.vertices.len(), lw.is_closed, lw.constant_width,
                    lw.elevation, lw.thickness, lw.normal.x, lw.normal.y, lw.normal.z);
                println!("  owner={:#X} layer='{}' color={:?} lw={:?} lt_scale={} invisible={} transparency={:?}",
                    c.owner_handle.value(), c.layer, c.color, c.line_weight, c.linetype_scale, c.invisible, c.transparency);
                println!("  reactors={} xdict={:?}",
                    c.reactors.len(), c.xdictionary_handle);
                // Print vertex data
                for (i, v) in lw.vertices.iter().enumerate() {
                    println!("  v[{}]: ({}, {}) bulge={} sw={} ew={}", i, v.location.x, v.location.y, v.bulge, v.start_width, v.end_width);
                }
            }
        }
    }

    // Also check surrounding handles +/-1 to understand the pattern
    println!("\n--- Handles around 0x2432D ---");
    for h in 0x2432A..=0x24348 {
        let handle = acadrust::types::Handle::new(h);
        if let Some(entity) = doc.get_entity(handle) {
            let tn = match entity {
                EntityType::LwPolyline(lw) => format!("LwPolyline(verts={})", lw.vertices.len()),
                EntityType::Line(_) => "Line".to_string(),
                EntityType::Solid(_) => "Solid".to_string(),
                EntityType::MText(_) => "MText".to_string(),
                EntityType::Point(_) => "Point".to_string(),
                EntityType::Arc(_) => "Arc".to_string(),
                EntityType::Dimension(_) => "Dimension".to_string(),
                _ => format!("{:?}", std::mem::discriminant(entity)),
            };
            println!("  {:#X}: {} owner={:#X}", h, tn, entity.common().owner_handle.value());
        }
    }
}
