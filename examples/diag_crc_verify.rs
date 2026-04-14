//! Diagnostic: Verify CRC-16 for every object record
//!
//! AutoCAD validates CRC-16 on every object load. If any CRCs are wrong,
//! that could explain "Object Null has invalid data" errors.
//!
//! Usage: cargo run --example diag_crc_verify

use acadrust::io::dwg::{DwgReader, DwgWriter};
use acadrust::io::dwg::crc::{crc16, CRC16_SEED};

fn read_ms(data: &[u8], offset: usize) -> Option<(usize, usize)> {
    if offset + 1 >= data.len() { return None; }
    let word = u16::from_le_bytes([data[offset], data[offset + 1]]);
    if (word & 0x8000) == 0 {
        Some((word as usize, 2))
    } else {
        if offset + 3 >= data.len() { return None; }
        let word2 = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
        let val = (word as usize & 0x7FFF) | ((word2 as usize) << 15);
        Some((val, 4))
    }
}

fn main() {
    let input = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/roundtrip/samplekitchen.dwg".to_string());

    let doc = {
        let mut reader = DwgReader::from_file(&input).expect("open");
        reader.read().expect("read")
    };

    let out_path = "target/diag_crc.dwg";
    DwgWriter::write_to_file(out_path, &doc).expect("write");

    let mut reader = DwgReader::from_file(out_path).expect("open written");
    let info = reader.read_file_header().expect("read header");
    let handle_buf = reader.get_section_buffer("AcDb:Handles", &info).expect("handles");
    let handle_map = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&handle_buf).expect("parse handles");
    let objects_buf = reader.get_section_buffer("AcDb:AcDbObjects", &info).expect("objects");

    println!("Handle entries: {}, Objects buf: {} bytes", handle_map.len(), objects_buf.len());

    // Sort by offset
    let mut entries: Vec<(u64, i64)> = handle_map.into_iter().collect();
    entries.sort_by_key(|&(_, off)| off);

    let mut crc_ok = 0u64;
    let mut crc_fail = 0u64;
    let mut parse_fail = 0u64;
    let mut first_failures: Vec<(u64, usize, u16, u16)> = Vec::new();

    for &(handle, offset) in &entries {
        let off = offset as usize;
        if let Some((size, ms_len)) = read_ms(&objects_buf, off) {
            // CRC covers [MS][data] (no MC for pre-R2010)
            let crc_start = off;
            let crc_end = off + ms_len + size; // end of data, before CRC bytes
            if crc_end + 2 > objects_buf.len() {
                parse_fail += 1;
                continue;
            }
            let computed = crc16(CRC16_SEED, &objects_buf[crc_start..crc_end]);
            let stored = u16::from_le_bytes([objects_buf[crc_end], objects_buf[crc_end + 1]]);
            if computed == stored {
                crc_ok += 1;
            } else {
                crc_fail += 1;
                if first_failures.len() < 20 {
                    first_failures.push((handle, off, computed, stored));
                }
            }
        } else {
            parse_fail += 1;
        }
    }

    println!("\n=== CRC-16 Verification ===");
    println!("CRC OK:        {}", crc_ok);
    println!("CRC FAIL:      {}", crc_fail);
    println!("Parse fail:    {}", parse_fail);

    if !first_failures.is_empty() {
        println!("\nFirst failures:");
        for (handle, off, computed, stored) in &first_failures {
            println!("  Handle 0x{:X} at offset 0x{:X}: computed=0x{:04X}, stored=0x{:04X}",
                handle, off, computed, stored);
        }
    }
}
