//! Diagnostic: Compare handle sections between original and roundtripped files
//!
//! If the roundtripped file has fewer handles, entities may reference handles
//! that don't exist, causing "Object Null has invalid data" errors in AutoCAD.
//!
//! Usage: cargo run --example diag_handle_diff

use std::collections::HashSet;
use acadrust::io::dwg::{DwgReader, DwgWriter};

fn main() {
    let input = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/roundtrip/samplekitchen.dwg".to_string());

    // Read original file's handle section
    println!("=== Original file: {} ===", input);
    let (doc, orig_handles) = {
        let mut reader = DwgReader::from_file(&input).expect("open original");
        let info = reader.read_file_header().expect("header");
        let handle_buf = reader.get_section_buffer("AcDb:Handles", &info).expect("handles");
        let hmap = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&handle_buf)
            .expect("parse handles");
        let orig_handles: HashSet<u64> = hmap.keys().copied().collect();
        
        let mut reader2 = DwgReader::from_file(&input).expect("open");
        let doc = reader2.read().expect("read");
        (doc, orig_handles)
    };
    println!("Original handle entries: {}", orig_handles.len());

    // Write roundtripped file
    let out_path = "target/diag_handle_diff.dwg";
    DwgWriter::write_to_file(out_path, &doc).expect("write");

    // Read roundtripped file's handle section
    let rt_handles = {
        let mut reader = DwgReader::from_file(out_path).expect("open rt");
        let info = reader.read_file_header().expect("header");
        let handle_buf = reader.get_section_buffer("AcDb:Handles", &info).expect("handles");
        let hmap = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&handle_buf)
            .expect("parse handles");
        let rt_handles: HashSet<u64> = hmap.keys().copied().collect();
        rt_handles
    };
    println!("Roundtripped handle entries: {}", rt_handles.len());

    // Compare
    let in_orig_not_rt: HashSet<u64> = orig_handles.difference(&rt_handles).copied().collect();
    let in_rt_not_orig: HashSet<u64> = rt_handles.difference(&orig_handles).copied().collect();

    println!("\nHandles in original but NOT in roundtrip: {}", in_orig_not_rt.len());
    println!("Handles in roundtrip but NOT in original: {}", in_rt_not_orig.len());

    if !in_orig_not_rt.is_empty() {
        let mut missing: Vec<u64> = in_orig_not_rt.iter().copied().collect();
        missing.sort();
        println!("\nFirst 50 missing handles (in original, not in RT):");
        for &h in missing.iter().take(50) {
            println!("  0x{:X}", h);
        }
        println!("\n→ Entities in the RT file may reference these handles.");
        println!("  AutoCAD would resolve them as NULL → 'Object Null has invalid data'.");
    }

    if !in_rt_not_orig.is_empty() {
        let mut new_handles: Vec<u64> = in_rt_not_orig.iter().copied().collect();
        new_handles.sort();
        println!("\nFirst 20 new handles (in RT, not in original):");
        for &h in new_handles.iter().take(20) {
            println!("  0x{:X}", h);
        }
    }

    // Check what types of objects the missing handles corresponded to
    // in the original by reading them from the original objects section
    println!("\n=== Sampling object types for missing handles ===");
    let mut reader = DwgReader::from_file(&input).expect("open");
    let info = reader.read_file_header().expect("header");
    let handle_buf = reader.get_section_buffer("AcDb:Handles", &info).expect("handles");
    let orig_hmap = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&handle_buf)
        .expect("parse handles");
    let objects_buf = reader.get_section_buffer("AcDb:AcDbObjects", &info).expect("objects");
    
    let mut type_counts: std::collections::HashMap<i16, usize> = std::collections::HashMap::new();
    let mut sample_count = 0;
    
    let mut missing: Vec<u64> = in_orig_not_rt.iter().copied().collect();
    missing.sort();
    
    for &h in &missing {
        if let Some(&offset) = orig_hmap.get(&h) {
            if offset >= 0 && (offset as usize) + 2 < objects_buf.len() {
                // Read MS then type code from the original objects section
                let off = offset as usize;
                let word = u16::from_le_bytes([objects_buf[off], objects_buf[off + 1]]);
                let (_, ms_len) = if (word & 0x8000) == 0 {
                    (word as usize, 2)
                } else {
                    if off + 3 >= objects_buf.len() { continue; }
                    let word2 = u16::from_le_bytes([objects_buf[off + 2], objects_buf[off + 3]]);
                    ((word as usize & 0x7FFF) | ((word2 as usize) << 15), 4)
                };
                
                // Read type code (BS) from the start of merged data
                let data_start = off + ms_len;
                if data_start + 1 < objects_buf.len() {
                    // Simple: read first 2 bytes as LE u16, lower bits
                    // This is a rough approximation - type code is bit-encoded
                    let byte0 = objects_buf[data_start];
                    // For proper BS reading: if bits 00 = read raw short,
                    // if bits 01 = read 1 byte (unsigned char), etc.
                    // For simplicity, use the object reader
                    let type_code = byte0 as i16; // rough approximation
                    *type_counts.entry(type_code).or_insert(0) += 1;
                    sample_count += 1;
                }
            }
        }
    }
    // Print raw bytes for small missing sets so we can identify the type manually
    println!("Sampled {} missing objects", sample_count);
    if missing.len() <= 10 {
        for &h in &missing {
            if let Some(&offset) = orig_hmap.get(&h) {
                let off = offset as usize;
                if off + 8 <= objects_buf.len() {
                    let slice = &objects_buf[off..off+8];
                    print!("  Handle 0x{:X} raw bytes: ", h);
                    for b in slice { print!("{:02X} ", b); }
                    println!();
                }
            }
        }
    } else {
        println!("(Exact type identification requires full bit-stream parsing)");
    }
}
