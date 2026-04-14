/// Diagnostic: Inspect PolyFaceMesh entities and BlockReference→BlockTableRecord references.
use std::io::Cursor;
use acadrust::{CadDocument, DwgReader};
use acadrust::entities::EntityType;

fn load_dwg(path: &str) -> Option<CadDocument> {
    let bytes = std::fs::read(path).ok()?;
    let mut r = DwgReader::from_stream(Cursor::new(bytes));
    r.read().ok()
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: cargo run --example diag_polyface -- <file1.dwg> [file2.dwg ...]");
        std::process::exit(2);
    }

    for path in &paths {
        println!("=== {} ===\n", path);
        let doc = match load_dwg(path) {
            Some(d) => d,
            None => {
                eprintln!("Failed to load {}", path);
                continue;
            }
        };

        // Find all polyface meshes
        let mut pface_count = 0;
        for entity in doc.entities() {
            if let EntityType::PolyfaceMesh(ref e) = entity {
                pface_count += 1;
                let handle = e.common.handle;
                println!("  PolyfaceMesh handle={:X} vertices={} faces={}",
                         handle.value(), e.vertices.len(), e.faces.len());

                // Show first few face indices
                for (i, face) in e.faces.iter().enumerate().take(5) {
                    println!("    Face[{}]: indices=({}, {}, {}, {})",
                             i, face.index1, face.index2, face.index3, face.index4);
                }
                if e.faces.len() > 5 {
                    println!("    ... and {} more faces", e.faces.len() - 5);
                }

                // Validate: check if any face index is out of range
                let num_verts = e.vertices.len() as i16;
                let mut invalid_count = 0;
                for (i, face) in e.faces.iter().enumerate() {
                    let indices = [face.index1, face.index2, face.index3, face.index4];
                    for &idx in &indices {
                        if idx != 0 {
                            let abs_idx = idx.abs();
                            if abs_idx < 1 || abs_idx > num_verts {
                                if invalid_count == 0 {
                                    println!("    INVALID Face[{}]: idx={} out of range [1..{}]",
                                             i, idx, num_verts);
                                }
                                invalid_count += 1;
                            }
                        }
                    }
                }
                if invalid_count > 0 {
                    println!("    Total invalid indices: {}", invalid_count);
                } else {
                    println!("    All face indices valid (range 1..{})", num_verts);
                }
                println!();
            }
        }
        println!("Total PolyfaceMesh entities: {}\n", pface_count);

        // Check BlockTableRecord integrity
        println!("--- BlockTableRecord checks ---");
        let mut block_issues = 0;
        for br in doc.block_records.iter() {
            let block_h = br.block_entity_handle;
            let endblk_h = br.block_end_handle;

            // Check if block entity handle exists
            let has_block = !block_h.is_null() && doc.get_entity(block_h).is_some();
            let has_endblk = !endblk_h.is_null() && doc.get_entity(endblk_h).is_some();

            if !has_block || !has_endblk {
                println!("  BlockRecord '{}' handle={:X}: block_entity={:X}({}) endblk={:X}({}) entities={}",
                         br.name, br.handle.value(),
                         block_h.value(), if has_block { "OK" } else { "MISSING" },
                         endblk_h.value(), if has_endblk { "OK" } else { "MISSING" },
                         br.entity_handles.len());
                block_issues += 1;
            }
        }
        if block_issues == 0 {
            println!("  All BlockTableRecords have valid BLOCK/ENDBLK handles");
        } else {
            println!("  {} BlockTableRecords with missing BLOCK/ENDBLK entities", block_issues);
        }

        // Check specific handles from BricsCAD error
        let problem_handles: Vec<u64> = vec![
            0x209C73, 0x2237E6, 0x2237EB, 0x22347D, 0x202F57,
            0x2024DC, 0x202FDE, 0x2067F0, 0x206C0D, 0x209165,
            0x2230C8, 0x224C2D, 0x2253C3, 0x2254C3, 0x225533,
            0x225573, 0x225EB9, 0x22633C, 0x228FEC,
        ];
        println!("\n--- BricsCAD-reported invalid BlockTableRecord handles ---");
        for &h in &problem_handles {
            let found = doc.block_records.iter().find(|br| br.handle.value() == h);
            if let Some(br) = found {
                let has_block = !br.block_entity_handle.is_null() && doc.get_entity(br.block_entity_handle).is_some();
                let has_endblk = !br.block_end_handle.is_null() && doc.get_entity(br.block_end_handle).is_some();
                println!("  {:X}: name='{}' block={:X}({}) endblk={:X}({}) entities={} layout={:X}",
                         h, br.name,
                         br.block_entity_handle.value(), if has_block { "OK" } else { "MISS" },
                         br.block_end_handle.value(), if has_endblk { "OK" } else { "MISS" },
                         br.entity_handles.len(),
                         br.layout.value());
            } else {
                println!("  {:X}: NOT FOUND in block records", h);
            }
        }

        // Check BlockReference → BlockTableRecord references
        println!("\n--- BlockReference checks ---");
        let mut invalid_brefs = 0;
        for entity in doc.entities() {
            if let EntityType::Insert(ref ins) = entity {
                // Check if block_name references an existing block record
                let has_record = doc.block_records.iter().any(|br| br.name == ins.block_name);
                if !has_record {
                    println!("  INSERT handle={:X} block_name='{}' → NO BlockRecord found!",
                             ins.common.handle.value(), ins.block_name);
                    invalid_brefs += 1;
                }
            }
        }
        if invalid_brefs == 0 {
            println!("  All BlockReferences have valid BlockTableRecord names");
        } else {
            println!("  {} BlockReferences with missing BlockTableRecords", invalid_brefs);
        }
        println!();
    }
}
