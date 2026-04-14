//! Diagnostic: Deep inspection of object record framing
//!
//! Usage: cargo run --example diag_record_walk

use acadrust::io::dwg::{DwgReader, DwgWriter};

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

    let out_path = "target/diag_record_walk.dwg";
    DwgWriter::write_to_file(out_path, &doc).expect("write");

    let mut reader = DwgReader::from_file(out_path).expect("open written");
    let info = reader.read_file_header().expect("read header");
    let handle_buf = reader.get_section_buffer("AcDb:Handles", &info).expect("handles");
    let handle_map = acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles(&handle_buf).expect("parse handles");
    let objects_buf = reader.get_section_buffer("AcDb:AcDbObjects", &info).expect("objects");

    println!("Objects buffer: {} bytes, Handle entries: {}", objects_buf.len(), handle_map.len());

    // Show first 128 bytes
    println!("\nFirst 128 bytes of objects buffer:");
    for row in 0..8 {
        let start = row * 16;
        print!("  {:08X}: ", start);
        for i in 0..16 {
            if start + i < objects_buf.len() {
                print!("{:02X} ", objects_buf[start + i]);
            }
        }
        println!();
    }

    // Walk first 10 records using handle map offsets
    let mut offsets: Vec<i64> = handle_map.values().copied().collect();
    offsets.sort();
    offsets.dedup();

    println!("\nFirst 20 handle map offsets (sorted):");
    for (i, &off) in offsets.iter().take(20).enumerate() {
        let ms = read_ms(&objects_buf, off as usize);
        let gap = if i > 0 { off - offsets[i - 1] } else { 0 };
        println!("  [{:3}] offset=0x{:08X} MS={:?} gap_from_prev={}", i, off, ms, gap);
    }

    // Sequential walk from the first handle offset
    println!("\nSequential walk from first offset (0x{:X}):", offsets[0]);
    let mut off = offsets[0] as usize;
    let mut count = 0;
    let mut next_expected: Vec<usize> = Vec::new();
    while off + 2 <= objects_buf.len() && count < 20 {
        if let Some((size, ms_len)) = read_ms(&objects_buf, off) {
            if size == 0 || size > 100000 {
                println!("  Record {}: offset=0x{:08X} INVALID MS={} — stopping", count, off, size);
                // Show raw bytes
                print!("    Raw: ");
                for i in 0..16.min(objects_buf.len() - off) {
                    print!("{:02X} ", objects_buf[off + i]);
                }
                println!();
                break;
            }
            let record_end = off + ms_len + size + 2; // +2 for CRC
            let in_handle_map = offsets.binary_search(&(off as i64)).is_ok();
            println!("  Record {}: offset=0x{:08X} size={} ms_len={} crc_end=0x{:X} in_hmap={}",
                count, off, size, ms_len, record_end, in_handle_map);
            next_expected.push(record_end);
            off = record_end;
            count += 1;
        } else {
            break;
        }
    }

    // Check: are consecutive handle map offsets consistent with record sizes?
    println!("\nRecord-to-record gap analysis (first 20):");
    for i in 0..20.min(offsets.len() - 1) {
        let off1 = offsets[i] as usize;
        let off2 = offsets[i + 1] as usize;
        if let Some((size, ms_len)) = read_ms(&objects_buf, off1) {
            let expected_next = off1 + ms_len + size + 2;
            let gap = off2 as i64 - expected_next as i64;
            if gap != 0 {
                println!("  [{:3}→{:3}] offset=0x{:X} size={} expected_next=0x{:X} actual_next=0x{:X} GAP={}",
                    i, i+1, off1, size, expected_next, off2, gap);
            }
        }
    }

    // Count total gaps
    let mut gap_count = 0u64;
    let mut total_gap = 0i64;
    for i in 0..offsets.len() - 1 {
        let off1 = offsets[i] as usize;
        if let Some((size, ms_len)) = read_ms(&objects_buf, off1) {
            let expected_next = off1 + ms_len + size + 2;
            let gap = offsets[i + 1] as i64 - expected_next as i64;
            if gap != 0 {
                gap_count += 1;
                total_gap += gap;
            }
        }
    }
    println!("\nGap summary: {} records with gaps out of {} total, total gap bytes: {}",
        gap_count, offsets.len(), total_gap);
}
