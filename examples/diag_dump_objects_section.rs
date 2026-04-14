//! Diagnostic: Dump decompressed AcDbObjects data at suspect offsets
//!
//! Checks what's actually at the addresses where AutoCAD reports "invalid size 0"
//!
//! Usage: cargo run --example diag_dump_objects_section

use std::io::Cursor;
use acadrust::io::dwg::{DwgReader, DwgWriter};

fn dump_hex(data: &[u8], offset: usize, count: usize) {
    let end = (offset + count).min(data.len());
    for start in (offset..end).step_by(16) {
        print!("  {:08X}: ", start);
        let line_end = (start + 16).min(end);
        for i in start..line_end {
            print!("{:02X} ", data[i]);
        }
        for _ in line_end..start + 16 {
            print!("   ");
        }
        print!(" |");
        for i in start..line_end {
            let c = data[i];
            if c >= 0x20 && c < 0x7F {
                print!("{}", c as char);
            } else {
                print!(".");
            }
        }
        println!("|");
    }
}

fn read_ms(data: &[u8], offset: usize) -> (usize, usize) {
    if offset + 1 >= data.len() {
        return (0, 0);
    }
    let word = u16::from_le_bytes([data[offset], data[offset + 1]]);
    if (word & 0x8000) == 0 {
        (word as usize, 2)
    } else {
        if offset + 3 >= data.len() {
            return (0, 0);
        }
        let word2 = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
        let val = (word as usize & 0x7FFF) | ((word2 as usize) << 15);
        (val, 4)
    }
}

fn main() {
    let input = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/roundtrip/samplekitchen.dwg".to_string());

    // Step 1: Read original and write roundtripped
    println!("Reading: {}", input);
    let doc = {
        let mut reader = DwgReader::from_file(&input).expect("open");
        reader.read().expect("read")
    };

    // Write to buffer
    let mut buf = Vec::new();
    DwgWriter::write_to_writer(Cursor::new(&mut buf), &doc).expect("write");

    // Step 2: Read back to get the decompressed AcDbObjects section data
    // We need to use the internal reader to get section data
    let out_path = "target/diag_dump_obj.dwg";
    DwgWriter::write_to_file(out_path, &doc).expect("write to file");

    // Re-read the written file to get section data
    let mut reader = DwgReader::from_file(out_path).expect("open written");
    let info = reader.read_file_header().expect("read header");

    println!("File header info:");
    println!("  Page records: {}", info.page_records.len());
    println!("  Section descriptors: {}", info.section_descriptors.len());

    // Find AcDbObjects section
    if let Some(objects_section) = info.section_descriptors.iter()
        .find(|s| s.name == "AcDb:AcDbObjects")
    {
        println!("\nAcDb:AcDbObjects section:");
        println!("  Data size: {} bytes", objects_section.compressed_size);
        println!("  Encoding: {}", objects_section.encoding);
        println!("  Pages: {}", objects_section.pages.len());
        for (i, p) in objects_section.pages.iter().enumerate() {
            let page_info = info.page_records.get(&(p.page_number as i32));
            println!("  Page {:2}: id={}, decomp={}, comp={}, file_offset={:?}",
                i, p.page_number, p.decompressed_size, p.compressed_size,
                page_info.map(|&(o, _)| o));
        }
    }

    // Get decompressed section buffer
    let obj_buf = reader.get_section_buffer("AcDb:AcDbObjects", &info)
        .expect("get objects section");
    println!("\nDecompressed AcDbObjects size: {} bytes ({:.1} KB)",
        obj_buf.len(), obj_buf.len() as f64 / 1024.0);

    // Step 3: Check page boundaries
    let page_size = 0xF800usize;
    let total_pages = (obj_buf.len() + page_size - 1) / page_size;
    println!("Total pages: {} (at 0x{:X} bytes each)", total_pages, page_size);

    // Suspect addresses from AutoCAD audit (first few "invalid size 0"):
    let suspect_offsets: Vec<usize> = vec![
        0x000F802D, 0x000F804A, 0x000F8189, 0x000F81E0,
        0x000F82AB, 0x000F83B0, 0x000F83CD,
        0x001A9B1E, 0x001A30F0, 0x001A3189,
    ];

    println!("\n=== Checking suspect offsets ===");
    for &addr in &suspect_offsets {
        if addr >= obj_buf.len() {
            println!("\n  0x{:08X}: OUT OF RANGE (buf len = 0x{:X})", addr, obj_buf.len());
            continue;
        }
        let (ms_size, ms_len) = read_ms(&obj_buf, addr);
        println!("\n  0x{:08X}: MS(size) = {} ({}B header), page {}", 
            addr, ms_size, ms_len, addr / page_size);
        dump_hex(&obj_buf, addr, 32);
    }

    // Step 4: Check page boundary transitions
    println!("\n=== Page boundary analysis ===");
    for page_idx in [15, 16, 17, 26, 27, 28] {
        let boundary = page_idx * page_size;
        if boundary >= obj_buf.len() { continue; }
        println!("\nPage {} boundary (0x{:X}):", page_idx, boundary);
        // Show last 16 bytes of previous page
        if boundary >= 16 {
            println!("  End of page {}:", page_idx - 1);
            dump_hex(&obj_buf, boundary - 16, 16);
        }
        println!("  Start of page {}:", page_idx);
        dump_hex(&obj_buf, boundary, 32);
        // Read MS at boundary
        let (ms, ms_len) = read_ms(&obj_buf, boundary);
        println!("  MS at boundary: size={}, header={}B", ms, ms_len);
    }

    // Step 5: Walk object records around page 16 boundary
    println!("\n=== Object record walk near page 16 boundary ===");
    let target = 16 * page_size; // 0xF8000
    // Find the last valid record before the boundary
    let mut offset = 0usize;
    let mut record_count = 0usize;
    let mut last_before_boundary = 0usize;
    while offset + 2 <= obj_buf.len() && offset < target + 256 {
        let (ms_size, ms_len) = read_ms(&obj_buf, offset);
        if ms_size == 0 || ms_size > 100000 {
            if offset > target - 64 {
                println!("  Record {} at 0x{:08X}: MS(size)={} — INVALID", 
                    record_count, offset, ms_size);
            }
            break;
        }
        let record_end = offset + ms_len + ms_size + 2; // +2 for CRC16
        if offset < target && record_end > target {
            println!("  *** CROSS-BOUNDARY: Record {} at 0x{:08X}, size={}, ends at 0x{:X} (crosses page 16 at 0x{:X})",
                record_count, offset, ms_size, record_end, target);
        }
        if offset >= target - 128 && offset <= target + 128 {
            println!("  Record {} at 0x{:08X}: size={}", record_count, offset, ms_size);
        }
        if offset < target {
            last_before_boundary = offset;
        }
        offset = record_end;
        record_count += 1;
    }
    println!("  Last record before boundary: 0x{:08X}", last_before_boundary);
    println!("  Total records walked: {}", record_count);
}
