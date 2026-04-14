/// Compare handle maps between original and roundtripped DWG files.
/// Usage: cargo run --example diag_handle_map -- path/to/file.dwg

use acadrust::io::dwg::{DwgReader, DwgWriter};
use std::io::Cursor;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        String::from(r"tests\roundtrip\samplekitchen.dwg")
    });

    let bytes = std::fs::read(&path).expect("Failed to read file");
    
    // Read original
    let mut reader = DwgReader::from_stream(Cursor::new(&bytes));
    let doc = reader.read().expect("Failed to read DWG");
    
    // Write roundtrip
    let rt_bytes = DwgWriter::write_to_vec(&doc).expect("Failed to write DWG");
    
    // Save to disk for external testing
    let rt_path = path.replace(".dwg", "_rt.dwg");
    std::fs::write(&rt_path, &rt_bytes).expect("Failed to write RT DWG");
    println!("Wrote roundtripped file to: {} ({} bytes)", rt_path, rt_bytes.len());
    
    // Read RT handle map
    let mut rt_hdr_reader = DwgReader::from_stream(Cursor::new(&rt_bytes));
    let rt_info = rt_hdr_reader.read_file_header().expect("read RT header");
    let rt_handles_section = rt_hdr_reader.get_section_buffer("AcDb:Handles", &rt_info).expect("read handles");
    let rt_handle_map = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&rt_handles_section).expect("parse handles");
    let rt_objects_section = rt_hdr_reader.get_section_buffer("AcDb:AcDbObjects", &rt_info).expect("read objects");
    
    println!("\n--- RT Handle Map ---");
    println!("  RT handle map entries:    {}", rt_handle_map.len());
    println!("  RT objects section size:  {} bytes", rt_objects_section.len());
    
    // Validate: check each handle offset points to valid data
    let mut valid = 0u32;
    let mut out_of_range = 0u32;
    for (&handle, &offset) in &rt_handle_map {
        let off = offset as usize;
        if off >= rt_objects_section.len() {
            out_of_range += 1;
            if out_of_range <= 5 {
                println!("  OUT OF RANGE: handle {:#X} offset {}", handle, offset);
            }
        } else {
            valid += 1;
        }
    }
    println!("  Valid offsets:    {}", valid);
    println!("  Out of range:    {}", out_of_range);
    
    // Read original handle map
    let mut orig_reader = DwgReader::from_stream(Cursor::new(&bytes));
    let orig_info = orig_reader.read_file_header().expect("read orig header");
    let orig_handles_section = orig_reader.get_section_buffer("AcDb:Handles", &orig_info).expect("read orig handles");
    let orig_handle_map = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&orig_handles_section).expect("parse orig handles");
    
    println!("\n--- Original Handle Map ---");
    println!("  Original handle map entries: {}", orig_handle_map.len());
    
    // Compare
    let rt_handles: std::collections::HashSet<u64> = rt_handle_map.keys().copied().collect();
    let orig_handles: std::collections::HashSet<u64> = orig_handle_map.keys().copied().collect();
    let only_in_orig: usize = orig_handles.difference(&rt_handles).count();
    let only_in_rt: usize = rt_handles.difference(&orig_handles).count();
    let in_both: usize = orig_handles.intersection(&rt_handles).count();
    println!("  Handles in both:          {}", in_both);
    println!("  Only in original:         {}", only_in_orig);
    println!("  Only in roundtrip:        {}", only_in_rt);
    
    // Show first few handles only in original (missing from RT)
    if only_in_orig > 0 {
        let mut missing: Vec<u64> = orig_handles.difference(&rt_handles).copied().collect();
        missing.sort();
        println!("\n  First 20 handles only in original:");
        for &h in missing.iter().take(20) {
            println!("    {:#X} (offset {})", h, orig_handle_map[&h]);
        }
    }
    
    // Check HANDSEED values
    let mut rt_reader2 = DwgReader::from_stream(Cursor::new(&rt_bytes));
    let doc2 = rt_reader2.read().expect("read RT");
    println!("\n--- Handle Seeds ---");
    println!("  Original HANDSEED:  {:#X}", doc.header.handle_seed);
    println!("  RT HANDSEED:        {:#X}", doc2.header.handle_seed);
}
