//! Diagnostic: Verify per-object CRC-16 in both original and roundtripped files
//!
//! For each object in the AcDbObjects buffer:
//! 1. Read ModularShort(size) 
//! 2. Read the data payload (size bytes)
//! 3. Compute CRC-16 over [MS_bytes + data]
//! 4. Compare with stored CRC-16 after the data
//!
//! Also compares object counts and sizes between original and RT.
//!
//! Usage: cargo run --example diag_object_crc -- <input.dwg>

use acadrust::io::dwg::DwgReader;
use acadrust::io::dwg::DwgWriter;

/// CRC-16 seed used per ODA spec for object CRCs
const CRC16_SEED: u16 = 0xC0C1;

/// CRC-16 lookup table (same as the one used in the writer)
fn crc16(seed: u16, data: &[u8]) -> u16 {
    static TABLE: [u16; 256] = {
        let mut t = [0u16; 256];
        let mut i = 0u16;
        while i < 256 {
            let mut crc = i;
            let mut j = 0;
            while j < 8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xA001;
                } else {
                    crc >>= 1;
                }
                j += 1;
            }
            t[i as usize] = crc;
            i += 1;
        }
        t
    };
    let mut crc = seed;
    for &b in data {
        crc = (crc >> 8) ^ TABLE[((crc ^ b as u16) & 0xFF) as usize];
    }
    crc
}

/// Read ModularShort (2 or 4 bytes) — returns (value, bytes_consumed)
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

/// Walk all objects in AcDbObjects buffer and verify CRC-16
fn verify_objects(label: &str, buf: &[u8]) -> (u64, u64, u64) {
    println!("\n=== {} ===", label);
    println!("Buffer size: {} bytes (0x{:X})", buf.len(), buf.len());

    // Skip 0x0DCA marker if present
    let start = if buf.len() >= 4 {
        let marker = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        if marker == 0x0DCA {
            println!("0x0DCA marker found at offset 0");
            4
        } else {
            0
        }
    } else {
        0
    };

    let mut off = start;
    let mut total = 0u64;
    let mut crc_ok = 0u64;
    let mut crc_fail = 0u64;
    let mut first_failures: Vec<(usize, usize, u16, u16)> = Vec::new();

    while off + 2 <= buf.len() {
        let (size, ms_len) = match read_ms(buf, off) {
            Some(v) => v,
            None => break,
        };
        
        if size == 0 {
            // End marker or padding
            break;
        }
        if size > 500_000 {
            println!("  WARNING: huge MS={} at offset 0x{:X}, stopping", size, off);
            break;
        }

        let data_start = off;
        let data_end = off + ms_len + size;
        let crc_off = data_end;

        if crc_off + 2 > buf.len() {
            println!("  WARNING: record at 0x{:X} overflows buffer (MS={}, ms_len={})", off, size, ms_len);
            break;
        }

        // CRC covers [MS_bytes + data_payload]
        let crc_data = &buf[data_start..data_end];
        let computed_crc = crc16(CRC16_SEED, crc_data);
        let stored_crc = u16::from_le_bytes([buf[crc_off], buf[crc_off + 1]]);

        total += 1;
        if computed_crc == stored_crc {
            crc_ok += 1;
        } else {
            crc_fail += 1;
            if first_failures.len() < 10 {
                first_failures.push((off, size, computed_crc, stored_crc));
            }
        }

        off = crc_off + 2; // advance past CRC
    }

    println!("Total objects: {}", total);
    println!("CRC OK:        {}", crc_ok);
    println!("CRC FAIL:      {}", crc_fail);

    if !first_failures.is_empty() {
        println!("First CRC failures:");
        for (off, size, computed, stored) in &first_failures {
            println!("  offset=0x{:X} size={} computed=0x{:04X} stored=0x{:04X}", 
                off, size, computed, stored);
        }
    }

    (total, crc_ok, crc_fail)
}

fn main() {
    let input = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/roundtrip/samplekitchen.dwg".to_string());

    println!("=== Per-object CRC-16 verification ===");
    println!("Input: {}", input);

    // Read original
    let mut reader = DwgReader::from_file(&input).expect("open original");
    let info = reader.read_file_header().expect("read header");
    let orig_objects = reader.get_section_buffer("AcDb:AcDbObjects", &info)
        .expect("get original objects");
    drop(reader);

    let (orig_total, orig_ok, orig_fail) = verify_objects("ORIGINAL", &orig_objects);

    // Read the document and roundtrip it
    let doc = {
        let mut r = DwgReader::from_file(&input).expect("open for read");
        r.read().expect("read doc")
    };
    let rt_path = "target/diag_object_crc_rt.dwg";
    DwgWriter::write_to_file(rt_path, &doc).expect("write RT");

    // Read RT raw structures
    let mut reader2 = DwgReader::from_file(rt_path).expect("open RT");
    let info2 = reader2.read_file_header().expect("read RT header");
    let rt_objects = reader2.get_section_buffer("AcDb:AcDbObjects", &info2)
        .expect("get RT objects");
    drop(reader2);

    let (rt_total, rt_ok, rt_fail) = verify_objects("ROUNDTRIPPED", &rt_objects);

    // Summary
    println!("\n=== SUMMARY ===");
    println!("Original:  {} objects, {} CRC OK, {} CRC FAIL", orig_total, orig_ok, orig_fail);
    println!("RT:        {} objects, {} CRC OK, {} CRC FAIL", rt_total, rt_ok, rt_fail);

    if orig_total != rt_total {
        println!("*** Object count differs: {} vs {}", orig_total, rt_total);
    }
}
