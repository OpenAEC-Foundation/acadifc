/// Diagnostic: validate CRC16 of every object in a DWG file's objects section.
///
/// Uses the library's handle reader and section reader to get handle→offset
/// pairs, then checks each object record's MS(size), data, and CRC16.

use acadrust::io::dwg::crc::crc16;
use acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles;
use acadrust::io::dwg::DwgReader;
use std::collections::HashMap;
use std::io::Cursor;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        String::from(r"tests\roundtrip\samplekitchen_rt.dwg")
    });
    let path2 = std::env::args().nth(2);

    let (obj_data, handle_map) = load_file(&path);

    // Sort handles by offset for sequential scan
    let mut sorted: Vec<(u64, i64)> = handle_map.iter().map(|(&h, &o)| (h, o)).collect();
    sorted.sort_by_key(|&(_, off)| off);

    // Validate each object record
    let mut valid = 0u32;
    let mut invalid_crc = 0u32;
    let mut invalid_size = 0u32;
    let mut out_of_range = 0u32;

    for &(handle, offset) in &sorted {
        let offset = offset as usize;
        if offset >= obj_data.len() {
            out_of_range += 1;
            continue;
        }

        let (size, ms_len) = read_modular_short(&obj_data[offset..]);
        if size == 0 {
            invalid_size += 1;
            continue;
        }

        let data_start = offset + ms_len;
        let data_end = data_start + size;
        let crc_end = data_end + 2;

        if crc_end > obj_data.len() {
            out_of_range += 1;
            continue;
        }

        let record = &obj_data[offset..data_end];
        let stored_crc = u16::from_le_bytes([obj_data[data_end], obj_data[data_end + 1]]);
        let computed_crc = crc16(0xC0C1, record);

        if computed_crc != stored_crc {
            invalid_crc += 1;
        } else {
            valid += 1;
        }
    }

    println!("\n=== CRC Validation: {} ===", path);
    println!("Total handles: {}", sorted.len());
    println!("Valid CRC: {} | Invalid CRC: {} | Size=0: {} | OOB: {}",
        valid, invalid_crc, invalid_size, out_of_range);

    // Compare mode
    if let Some(ref path2) = path2 {
        let (obj_data2, handle_map2) = load_file(path2);

        // Find shared handles
        let mut shared: Vec<u64> = handle_map.keys()
            .filter(|h| handle_map2.contains_key(h))
            .copied()
            .collect();
        shared.sort();

        let only_in_1: Vec<u64> = handle_map.keys()
            .filter(|h| !handle_map2.contains_key(h))
            .copied()
            .collect();
        let only_in_2: Vec<u64> = handle_map2.keys()
            .filter(|h| !handle_map.contains_key(h))
            .copied()
            .collect();

        println!("\n=== Comparison: {} vs {} ===", path, path2);
        println!("File 1 handles: {}", handle_map.len());
        println!("File 2 handles: {}", handle_map2.len());
        println!("Shared handles: {}", shared.len());
        println!("Only in file 1: {}", only_in_1.len());
        println!("Only in file 2: {}", only_in_2.len());

        // Compare sizes of shared objects
        let mut same_size = 0u32;
        let mut diff_size = 0u32;
        let mut same_data = 0u32;
        let mut diff_data = 0u32;
        let mut size_diffs: Vec<(u64, usize, usize)> = Vec::new();

        for &handle in &shared {
            let off1 = handle_map[&handle] as usize;
            let off2 = handle_map2[&handle] as usize;

            let (s1, ms1) = read_modular_short(&obj_data[off1..]);
            let (s2, ms2) = read_modular_short(&obj_data2[off2..]);

            if s1 == s2 {
                same_size += 1;
                // Also compare actual bytes
                let d1 = &obj_data[off1 + ms1..off1 + ms1 + s1];
                let d2 = &obj_data2[off2 + ms2..off2 + ms2 + s2];
                if d1 == d2 {
                    same_data += 1;
                } else {
                    diff_data += 1;
                }
            } else {
                diff_size += 1;
                size_diffs.push((handle, s1, s2));
            }
        }

        println!("\nOf {} shared handles:", shared.len());
        println!("  Same size: {} ({} identical data, {} different data)",
            same_size, same_data, diff_data);
        println!("  Different size: {}", diff_size);

        // Show size diff examples
        if !size_diffs.is_empty() {
            println!("\nSize differences (first 30):");
            size_diffs.sort_by_key(|&(_, s1, s2)| (s2 as i64 - s1 as i64).abs());
            size_diffs.reverse();
            for &(h, s1, s2) in size_diffs.iter().take(30) {
                let delta = s1 as i64 - s2 as i64;
                println!("  handle {:#X}: file1={} file2={} delta={}",
                    h, s1, s2, delta);
            }
        }

        // Show handles only in file 1 (limited)
        if !only_in_1.is_empty() {
            println!("\nHandles only in file 1 (first 20):");
            for &h in only_in_1.iter().take(20) {
                let off = handle_map[&h] as usize;
                let (size, ms) = read_modular_short(&obj_data[off..]);
                println!("  handle {:#X}: size={}", h, size);
            }
        }
    }
}

fn load_file(path: &str) -> (Vec<u8>, HashMap<u64, i64>) {
    let bytes = std::fs::read(path).expect("Failed to read file");
    let mut reader = DwgReader::from_stream(Cursor::new(&bytes));
    let info = reader.read_file_header().expect("Failed to read file header");
    let obj_data = reader.get_section_buffer("AcDb:AcDbObjects", &info)
        .expect("Failed to read objects section");
    let handle_data = reader.get_section_buffer("AcDb:Handles", &info)
        .expect("Failed to read handles section");
    let handle_map = read_handles(&handle_data).expect("Failed to parse handles");
    (obj_data, handle_map)
}

fn read_modular_short(data: &[u8]) -> (usize, usize) {
    let mut pos = 0;
    let mut result: usize = 0;
    let mut shift = 0;
    loop {
        if pos + 2 > data.len() { return (0, 0); }
        let word = u16::from_le_bytes([data[pos], data[pos + 1]]);
        pos += 2;
        result |= ((word & 0x7FFF) as usize) << shift;
        if word & 0x8000 == 0 {
            break;
        }
        shift += 15;
    }
    (result, pos)
}

fn read_type_code(data: &[u8]) -> i16 {
    if data.is_empty() { return -1; }
    // BS (bit short): first 2 bits encode the size variant
    let first_bits = (data[0] >> 6) & 0x03;
    match first_bits {
        0b00 => {
            // Full 16-bit raw value at bits 2..17
            if data.len() < 3 { return -1; }
            let val = ((data[0] & 0x3F) as i16)
                | ((data[1] as i16) << 6)
                | (((data[2] & 0x03) as i16) << 14);
            val
        },
        0b01 => {
            // 8-bit unsigned value at bits 2..9
            if data.len() < 2 { return -1; }
            let val = ((data[0] & 0x3F) as u16) | ((data[1] as u16 & 0x03) << 6);
            val as i16
        },
        0b10 => 0,
        0b11 => 256,
        _ => -1,
    }
}
