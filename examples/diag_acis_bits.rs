//! Diagnostic: Bit-level ACIS decode verification for a single 3DSOLID.
//!
//! Reads original file, finds a 3DSOLID by handle, extracts merged record data,
//! and manually decodes it with/without the "unknown bit" to see which produces
//! valid results.
//!
//! Usage: cargo run --example diag_acis_bits

use std::collections::HashMap;
use acadrust::io::dwg::DwgReader;
use acadrust::io::dwg::dwg_stream_readers::bit_reader::DwgBitReader;
use acadrust::io::dwg::dwg_version::DwgVersion;
use acadrust::types::DxfVersion;

fn read_handle_map(path: &str) -> HashMap<u64, i64> {
    let mut reader = DwgReader::from_file(path).expect("open");
    let info = reader.read_file_header().expect("header");
    let handle_buf = reader.get_section_buffer("AcDb:Handles", &info).expect("handles");
    acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&handle_buf)
        .expect("parse handles")
}

fn read_objects_buffer(path: &str) -> Vec<u8> {
    let mut reader = DwgReader::from_file(path).expect("open");
    let info = reader.read_file_header().expect("header");
    reader.get_section_buffer("AcDb:AcDbObjects", &info).expect("objects")
}

fn read_ms(data: &[u8], offset: usize) -> (usize, usize) {
    let mut result: usize = 0;
    let mut shift = 0;
    let mut pos = offset;
    loop {
        if pos + 1 >= data.len() { return (0, pos - offset); }
        let lo = data[pos] as usize;
        let hi = data[pos + 1] as usize;
        pos += 2;
        let word = lo | (hi << 8);
        let val = word & 0x7FFF;
        result |= val << shift;
        if (word & 0x8000) == 0 { break; }
        shift += 15;
    }
    (result, pos - offset)
}

fn read_record(data: &[u8], offset: usize) -> Option<Vec<u8>> {
    if offset >= data.len() { return None; }
    let (size, ms_bytes) = read_ms(data, offset);
    if size == 0 || offset + ms_bytes + size + 2 > data.len() { return None; }
    Some(data[offset + ms_bytes..offset + ms_bytes + size].to_vec())
}

fn main() {
    let input = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/roundtrip/samplekitchen.dwg".to_string());

    println!("=== Raw bit-level ACIS analysis ===\n");

    let handles = read_handle_map(&input);
    let objects = read_objects_buffer(&input);

    let target_handle = 0x1FAE94u64;
    if let Some(&offset) = handles.get(&target_handle) {
        let merged = read_record(&objects, offset as usize).expect("read record");
        println!("Handle 0x{:X}: {} bytes merged data", target_handle, merged.len());

        // R2007 RL (total_size_bits = handle stream start)
        let mut r0 = DwgBitReader::new(merged.clone(), DwgVersion::AC24, DxfVersion::AC1021);
        r0.read_object_type(); // type code (BS)
        let rl = r0.read_raw_long();
        println!("  RL={} main_stream_end=RL-1={}", rl, rl-1);
        
        // SAB starts at bit 188 — read raw bytes (set_position_in_bits=188)
        let mut r = DwgBitReader::new(merged.clone(), DwgVersion::AC24, DxfVersion::AC1021);
        r.set_position_in_bits(188);
        
        // Dump first 128 SAB bytes
        let sab_start_byte = 188/8;
        println!("  SAB starts at bit 188 (byte ~{})", sab_start_byte);
        println!("  First 128 SAB bytes (decoded with bit_shift=4):");
        let sab_bytes: Vec<u8> = (0..128).map(|_| r.read_byte()).collect();
        for (i, chunk) in sab_bytes.chunks(16).enumerate() {
            print!("    {:04X}: ", i * 16);
            for b in chunk { print!("{:02X} ", b); }
            print!(" | ");
            for b in chunk { print!("{}", if *b >= 32 && *b < 127 { *b as char } else { '.' }); }
            println!();
        }
        
        // Now look at the last bytes of the main stream (before handles)
        // to find the wireframe/silhouette/trailing data
        let main_end_bit = rl as i64 - 1;
        let main_end_byte = (main_end_bit / 8) as usize;
        let before_end = main_end_byte.saturating_sub(32);
        println!("  Last 64 bytes before handle stream (bytes {}-{}):", before_end, main_end_byte);
        for (i, chunk) in merged[before_end..main_end_byte+1].chunks(16).enumerate() {
            print!("    {:04X}: ", before_end + i * 16);
            for b in chunk { print!("{:02X} ", b); }
            println!();
        }
    }
}
