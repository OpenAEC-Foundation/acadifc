//! Diagnostic: Verify every handle→offset entry maps to a valid object record
//!
//! Walks the handle section and checks that each offset points to a valid
//! ModularShort(size) in the decompressed AcDbObjects data.
//!
//! Usage: cargo run --example diag_handle_integrity

use acadrust::io::dwg::{DwgReader, DwgWriter};

fn read_ms(data: &[u8], offset: usize) -> Option<(usize, usize)> {
    if offset + 1 >= data.len() {
        return None;
    }
    let word = u16::from_le_bytes([data[offset], data[offset + 1]]);
    if (word & 0x8000) == 0 {
        Some((word as usize, 2))
    } else {
        if offset + 3 >= data.len() {
            return None;
        }
        let word2 = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
        let val = (word as usize & 0x7FFF) | ((word2 as usize) << 15);
        Some((val, 4))
    }
}

fn main() {
    let input = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/roundtrip/samplekitchen.dwg".to_string());

    println!("Reading: {}", input);
    let doc = {
        let mut reader = DwgReader::from_file(&input).expect("open");
        reader.read().expect("read")
    };

    // Write to file
    let out_path = "target/diag_handle_check.dwg";
    DwgWriter::write_to_file(out_path, &doc).expect("write");

    // Re-read to get the raw structures
    let mut reader = DwgReader::from_file(out_path).expect("open written");
    let info = reader.read_file_header().expect("read header");

    // Get handle map
    let handle_buf = reader.get_section_buffer("AcDb:Handles", &info)
        .expect("get handles section");
    let handle_map = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&handle_buf)
        .expect("parse handles");

    // Get objects buffer
    let objects_buf = reader.get_section_buffer("AcDb:AcDbObjects", &info)
        .expect("get objects section");

    println!("Handle map entries: {}", handle_map.len());
    println!("Objects buffer size: {} bytes", objects_buf.len());

    // Walk all handle→offset entries
    let mut valid = 0u64;
    let mut invalid_offset = 0u64;
    let mut zero_size = 0u64;
    let mut huge_size = 0u64;
    let mut size_overflow = 0u64;
    let mut errors: Vec<(u64, i64, String)> = Vec::new();

    let page_size = 0xF800usize;

    for (&handle, &offset) in &handle_map {
        if offset < 0 {
            invalid_offset += 1;
            errors.push((handle, offset, "negative offset".to_string()));
            continue;
        }
        let off = offset as usize;
        if off >= objects_buf.len() {
            invalid_offset += 1;
            errors.push((handle, offset, format!("offset 0x{:X} >= buffer len 0x{:X}", off, objects_buf.len())));
            continue;
        }
        match read_ms(&objects_buf, off) {
            None => {
                invalid_offset += 1;
                errors.push((handle, offset, "can't read MS at offset".to_string()));
            }
            Some((size, _ms_len)) => {
                if size == 0 {
                    zero_size += 1;
                    errors.push((handle, offset, format!("MS=0 at page {}", off / page_size)));
                } else if size > 100000 {
                    huge_size += 1;
                    errors.push((handle, offset, format!("MS={} (huge) at page {}", size, off / page_size)));
                } else if off + size > objects_buf.len() {
                    size_overflow += 1;
                    errors.push((handle, offset, format!("MS={} overflows buffer (off=0x{:X})", size, off)));
                } else {
                    valid += 1;
                }
            }
        }
    }

    println!("\n=== Results ===");
    println!("Valid handle entries:    {}", valid);
    println!("Invalid offsets:        {}", invalid_offset);
    println!("Zero-size records:      {}", zero_size);
    println!("Huge-size records:      {}", huge_size);
    println!("Overflow records:       {}", size_overflow);

    if !errors.is_empty() {
        println!("\n=== First 20 errors ===");
        for (i, (handle, offset, msg)) in errors.iter().enumerate().take(20) {
            println!("  Handle 0x{:X} → offset 0x{:X}: {}", handle, offset, msg);
        }
    }

    // Walk objects sequentially and verify all handles match
    println!("\n=== Sequential object walk ===");
    let mut off = 0usize;
    let mut seq_count = 0u64;
    let mut seq_valid = 0u64;
    while off + 2 <= objects_buf.len() {
        if let Some((size, ms_len)) = read_ms(&objects_buf, off) {
            if size == 0 || size > 100000 {
                break;
            }
            let record_end = off + ms_len + size + 2; // +2 for CRC
            if record_end > objects_buf.len() {
                break;
            }
            seq_count += 1;
            seq_valid += 1;
            off = record_end;
        } else {
            break;
        }
    }
    println!("  Sequential records found: {}", seq_count);
    println!("  Remaining bytes: {}", objects_buf.len() - off);

    if seq_count as u64 == valid + zero_size + huge_size + size_overflow {
        println!("  Counts match: every handle entry maps to a sequentially found record ✓");
    } else {
        println!("  *** Count mismatch: handle map has {} valid, sequential walk found {}",
            valid, seq_count);
    }
}
