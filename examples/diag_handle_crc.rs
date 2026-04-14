/// Verify handle section chunk CRC-16 values.

use acadrust::io::dwg::DwgReader;
use acadrust::io::dwg::crc::{crc16, CRC16_SEED};

fn main() {
    let path = std::env::args().nth(1).expect("Usage: diag_handle_crc <file.dwg>");
    let mut reader = DwgReader::from_file(&path).expect("Failed to open");
    let info = reader.read_file_header().expect("Failed to read header");
    let data = reader.get_section_buffer("AcDb:Handles", &info).expect("No Handles section");

    println!("=== Handle section chunk CRC verification: {} ===", path);
    println!("Handles section size: {} bytes", data.len());

    let mut pos = 0usize;
    let mut chunk_idx = 0usize;
    let mut ok = 0usize;
    let mut fail = 0usize;

    while pos + 2 <= data.len() {
        let size = ((data[pos] as usize) << 8) | (data[pos + 1] as usize);
        if size < 2 || size > 2050 { 
            println!("  Terminated at pos={:#X} (size={})", pos, size);
            break; 
        }
        // chunk_end = pos + size includes the 2-byte size field + (size-2) data bytes
        let chunk_end = pos + size;
        let crc_pos = chunk_end;

        if crc_pos + 2 > data.len() {
            println!("  Chunk {} truncated at pos={:#X}", chunk_idx, pos);
            break;
        }

        // CRC stored in chunk (big-endian)
        let stored_crc = ((data[crc_pos] as u16) << 8) | (data[crc_pos + 1] as u16);

        // Try: CRC over entire chunk (size header + data bytes), as libredwg does
        let crc_full = crc16(CRC16_SEED, &data[pos..chunk_end]);
        // Try: CRC over data only (skip size header)  
        let crc_data_only = crc16(CRC16_SEED, &data[pos+2..chunk_end]);

        let full_ok = crc_full == stored_crc;
        let data_ok = crc_data_only == stored_crc;

        if full_ok {
            ok += 1;
        } else if data_ok {
            println!("  Chunk {}: PASS with data-only CRC (stored={:#06X} full={:#06X} data_only={:#06X})",
                chunk_idx, stored_crc, crc_full, crc_data_only);
            ok += 1;
        } else {
            fail += 1;
            if fail <= 5 {
                println!("  Chunk {}: FAIL stored={:#06X} full={:#06X} data_only={:#06X}",
                    chunk_idx, stored_crc, crc_full, crc_data_only);
            }
        }

        pos = crc_pos + 2;
        chunk_idx += 1;
    }

    println!("\nTotal chunks: {}, OK: {}, FAIL: {}", chunk_idx, ok, fail);
}
