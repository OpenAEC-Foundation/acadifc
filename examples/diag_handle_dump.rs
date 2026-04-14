/// Diagnostic: dump object info for specific handles.
/// Usage: cargo run --example diag_handle_dump -- <file.dwg> <handle_hex> [handle_hex...]

use acadrust::io::dwg::DwgReader;
use acadrust::types::Handle;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <file.dwg> <handle_hex> [handle_hex...]", args[0]);
        return;
    }

    let path = &args[1];
    let target_handles: Vec<u64> = args[2..].iter()
        .filter_map(|s| u64::from_str_radix(s, 16).ok())
        .collect();

    let mut reader = match DwgReader::from_file(path) {
        Ok(r) => r,
        Err(e) => { eprintln!("Failed to open: {:?}", e); return; }
    };

    let doc = match reader.read() {
        Ok(d) => d,
        Err(e) => { eprintln!("Failed to read: {:?}", e); return; }
    };

    println!("Entities: {}, Objects: {}", doc.entities().count(), doc.objects.len());
    println!("Block records: {}", doc.block_records.len());
    println!();

    for &h in &target_handles {
        let handle = Handle::from(h);
        println!("=== Handle 0x{:X} ===", h);

        // Check entities
        let mut found = false;
        for entity in doc.entities() {
            if entity.common().handle == handle {
                println!("  Entity: {}", std::any::type_name_of_val(entity).rsplit("::").next().unwrap_or("?"));
                found = true;
                break;
            }
            // Check sub-entities (polyface mesh vertices/faces)
            match entity {
                acadrust::entities::EntityType::PolyfaceMesh(pfm) => {
                    for v in &pfm.vertices {
                        if v.common.handle == handle {
                            println!("  PolyfaceMesh vertex of PFM at handle 0x{:X}", pfm.common.handle.value());
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        for f in &pfm.faces {
                            if f.common.handle == handle {
                                println!("  PolyfaceMesh face of PFM at handle 0x{:X}", pfm.common.handle.value());
                                found = true;
                                break;
                            }
                        }
                    }
                },
                _ => {}
            }
            if found { break; }
        }

        // Check objects
        if !found {
            if let Some(obj) = doc.objects.get(&handle) {
                println!("  Object: {:?}", std::mem::discriminant(obj));
                found = true;
            }
        }

        // Check table entries
        if !found {
            for br in doc.block_records.iter() {
                if br.handle == handle {
                    println!("  BlockRecord: '{}'", br.name);
                    found = true;
                    break;
                }
            }
        }
        if !found {
            for l in doc.layers.iter() {
                if l.handle == handle {
                    println!("  Layer: '{}'", l.name);
                    found = true;
                    break;
                }
            }
        }

        if !found {
            println!("  NOT FOUND in document model!");
        }
        println!();
    }
}
