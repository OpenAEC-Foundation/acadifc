//! Diagnostic: validates the object stream produced by DwgObjectWriter.
//!
//! Reads a DWG file, runs it through the writer, and checks:
//! 1. Duplicate handles in the handle map
//! 2. CRC-16 integrity of every object record
//! 3. Zero-size records (MS=0)
//! 4. Type codes at handle-map offsets
//!
//! Usage:
//!   cargo run --example diag_object_stream -- [path/to/file.dwg]

use std::collections::HashMap;
use std::path::PathBuf;

use acadrust::DwgReader;

fn main() {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("tests/roundtrip/samplekitchen.dwg"));

    eprintln!("Reading: {}", path.display());
    let doc = DwgReader::from_file(&path).unwrap().read().unwrap();
    eprintln!("Version: {:?}, Entities: {}", doc.version, doc.entities().count());

    // Write to get the raw object stream + handle map
    eprintln!("\nRunning DwgObjectWriter...");
    let obj_writer = acadrust::io::dwg::dwg_stream_writers::DwgObjectWriter::new(&doc).unwrap();
    let (obj_data, handle_map, _extents, _sab) = obj_writer.write();
    eprintln!("Object data: {} bytes", obj_data.len());
    eprintln!("Handle map: {} entries", handle_map.len());

    // ── Check 1: Duplicate handles ──
    let mut seen: HashMap<u64, Vec<u32>> = HashMap::new();
    for &(h, offset) in &handle_map {
        seen.entry(h).or_default().push(offset);
    }
    let dups: Vec<_> = seen.iter()
        .filter(|(_, offsets)| offsets.len() > 1)
        .collect();
    if dups.is_empty() {
        eprintln!("\n✓ No duplicate handles");
    } else {
        eprintln!("\n✗ {} DUPLICATE HANDLES:", dups.len());
        for (h, offsets) in &dups {
            eprint!("  Handle {:#X} at offsets:", h);
            for o in *offsets {
                let tc = read_type_code_at(&obj_data, *o as usize);
                eprint!(" {} (type={})", o, tc);
            }
            eprintln!();
        }
    }

    // ── Check 2: Walk object stream sequentially ──
    let is_r2004_plus = doc.version >= acadrust::types::DxfVersion::AC1018;
    let is_r2010_plus = doc.version >= acadrust::types::DxfVersion::AC1024;

    let mut pos = if is_r2004_plus { 4 } else { 0 }; // Skip 0x0DCA marker
    let mut record_count = 0u64;
    let mut zero_size_count = 0u64;
    let mut crc_fail_count = 0u64;
    let mut null_type_count = 0u64;

    while pos < obj_data.len() {
        let record_start = pos;

        // Read MS (modular short) size
        let (size, ms_len) = read_modular_short(&obj_data[pos..]);
        if ms_len == 0 {
            eprintln!("  ERROR: Cannot read MS at offset {:#X}", pos);
            break;
        }
        pos += ms_len;

        if size == 0 {
            // Zero-size record: [MS(0)] [CRC-16]
            zero_size_count += 1;
            if pos + 2 <= obj_data.len() {
                pos += 2; // skip CRC
            }
            record_count += 1;
            continue;
        }

        // Skip MC (handle bits) for R2010+
        if is_r2010_plus {
            let (_, mc_len) = read_modular_char(&obj_data[pos..]);
            pos += mc_len;
        }

        // Data
        if pos + size > obj_data.len() {
            eprintln!("  ERROR: Record at {:#X} extends past data (size={}, remain={})",
                record_start, size, obj_data.len() - pos);
            break;
        }
        let data_start = pos;
        pos += size;

        // CRC-16
        if pos + 2 > obj_data.len() {
            eprintln!("  ERROR: No room for CRC at {:#X}", pos);
            break;
        }
        let stored_crc = u16::from_le_bytes([obj_data[pos], obj_data[pos + 1]]);
        pos += 2;

        // Verify CRC: computed over [MS bytes + MC bytes + data]
        let record_bytes = &obj_data[record_start..pos - 2];
        let computed_crc = crc16(0xC0C1, record_bytes);
        if computed_crc != stored_crc {
            crc_fail_count += 1;
            if crc_fail_count <= 20 {
                eprintln!("  CRC MISMATCH at {:#X}: stored={:#06X}, computed={:#06X}, size={}",
                    record_start, stored_crc, computed_crc, size);
            }
        }

        // Read type code from data
        let type_code = read_type_code_from_data(&obj_data[data_start..data_start + size.min(4)]);
        if type_code == 0 {
            null_type_count += 1;
        }

        record_count += 1;
    }

    eprintln!("\n── Object Stream Summary ──");
    eprintln!("  Total records: {}", record_count);
    eprintln!("  Zero-size records: {}", zero_size_count);
    eprintln!("  CRC mismatches: {}", crc_fail_count);
    eprintln!("  Null type code: {}", null_type_count);

    // ── Check 3: Verify handle map offsets point to valid records ──
    let mut bad_offset_count = 0u64;
    let mut type_histogram: HashMap<i16, u64> = HashMap::new();
    for &(h, offset) in &handle_map {
        let o = offset as usize;
        if o >= obj_data.len() {
            bad_offset_count += 1;
            continue;
        }
        let tc = read_type_code_at(&obj_data, o);
        *type_histogram.entry(tc).or_default() += 1;
        if tc == 0 && bad_offset_count < 20 {
            eprintln!("  Handle {:#X} at offset {:#X} → type code 0 (null)", h, o);
            bad_offset_count += 1;
        }
    }

    eprintln!("\n── Handle Map Offsets ──");
    eprintln!("  Bad offsets: {}", bad_offset_count);
    eprintln!("  Type code distribution (top 20):");
    let mut sorted_types: Vec<_> = type_histogram.into_iter().collect();
    sorted_types.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
    for (tc, count) in sorted_types.iter().take(20) {
        eprintln!("    type {:>3} = {:>6} objects", tc, count);
    }
}

// ── Helpers ──

fn read_type_code_at(data: &[u8], offset: usize) -> i16 {
    if offset >= data.len() { return -1; }
    let (size, ms_len) = read_modular_short(&data[offset..]);
    if ms_len == 0 || size == 0 { return 0; }
    let data_start = offset + ms_len;
    if data_start >= data.len() { return -1; }
    read_type_code_from_data(&data[data_start..])
}

fn read_type_code_from_data(data: &[u8]) -> i16 {
    if data.is_empty() { return 0; }
    // BS (bit short) encoding: first 2 bits select format
    let first = data[0];
    let bb = (first >> 6) & 0x03;
    match bb {
        0b00 => {
            // 2-byte raw value follows the 2-bit prefix
            // 00 + 16 bits = 18 bits total → 3 bytes
            if data.len() < 3 { return -1; }
            let val = ((data[0] as u16 & 0x3F) << 10)
                | ((data[1] as u16) << 2)
                | ((data[2] as u16) >> 6);
            val as i16
        }
        0b01 => {
            // 1-byte value: 01 + 8 bits
            if data.len() < 2 { return -1; }
            let val = ((data[0] as u16 & 0x3F) << 2)
                | ((data[1] as u16) >> 6);
            val as i16
        }
        0b10 => 0, // value = 0
        0b11 => 256, // value = 256
        _ => unreachable!(),
    }
}

fn read_modular_short(data: &[u8]) -> (usize, usize) {
    let mut pos = 0;
    let mut value: usize = 0;
    let mut shift = 0;
    loop {
        if pos + 2 > data.len() { return (0, 0); }
        let word = u16::from_le_bytes([data[pos], data[pos + 1]]);
        pos += 2;
        value |= ((word & 0x7FFF) as usize) << shift;
        shift += 15;
        if (word & 0x8000) == 0 { break; }
    }
    (value, pos)
}

fn read_modular_char(data: &[u8]) -> (usize, usize) {
    let mut pos = 0;
    let mut value: usize = 0;
    let mut shift = 0;
    loop {
        if pos >= data.len() { return (0, 0); }
        let b = data[pos];
        pos += 1;
        value |= ((b & 0x7F) as usize) << shift;
        shift += 7;
        if (b & 0x80) == 0 { break; }
    }
    (value, pos)
}

fn crc16(seed: u16, data: &[u8]) -> u16 {
    // Use the same CRC table as the DWG writer
    static CRC_TABLE: [u16; 256] = [
        0x0000, 0xC0C1, 0xC181, 0x0140, 0xC301, 0x03C0, 0x0280, 0xC241,
        0xC601, 0x06C0, 0x0780, 0xC741, 0x0500, 0xC5C1, 0xC481, 0x0440,
        0xCC01, 0x0CC0, 0x0D80, 0xCD41, 0x0F00, 0xCFC1, 0xCE81, 0x0E40,
        0x0A00, 0xCAC1, 0xCB81, 0x0B40, 0xC901, 0x09C0, 0x0880, 0xC841,
        0xD801, 0x18C0, 0x1980, 0xD941, 0x1B00, 0xDBC1, 0xDA81, 0x1A40,
        0x1E00, 0xDEC1, 0xDF81, 0x1F40, 0xDD01, 0x1DC0, 0x1C80, 0xDC41,
        0x1400, 0xD4C1, 0xD581, 0x1540, 0xD701, 0x17C0, 0x1680, 0xD641,
        0xD201, 0x12C0, 0x1380, 0xD341, 0x1100, 0xD1C1, 0xD081, 0x1040,
        0xF001, 0x30C0, 0x3180, 0xF141, 0x3300, 0xF3C1, 0xF281, 0x3240,
        0x3600, 0xF6C1, 0xF781, 0x3740, 0xF501, 0x35C0, 0x3480, 0xF441,
        0x3C00, 0xFCC1, 0xFD81, 0x3D40, 0xFF01, 0x3FC0, 0x3E80, 0xFE41,
        0xFA01, 0x3AC0, 0x3B80, 0xFB41, 0x3900, 0xF9C1, 0xF881, 0x3840,
        0x2800, 0xE8C1, 0xE981, 0x2940, 0xEB01, 0x2BC0, 0x2A80, 0xEA41,
        0xEE01, 0x2EC0, 0x2F80, 0xEF41, 0x2D00, 0xEDC1, 0xEC81, 0x2C40,
        0xE401, 0x24C0, 0x2580, 0xE541, 0x2700, 0xE7C1, 0xE681, 0x2640,
        0x2200, 0xE2C1, 0xE381, 0x2340, 0xE101, 0x21C0, 0x2080, 0xE041,
        0xA001, 0x60C0, 0x6180, 0xA141, 0x6300, 0xA3C1, 0xA281, 0x6240,
        0x6600, 0xA6C1, 0xA781, 0x6740, 0xA501, 0x65C0, 0x6480, 0xA441,
        0x6C00, 0xACC1, 0xAD81, 0x6D40, 0xAF01, 0x6FC0, 0x6E80, 0xAE41,
        0xAA01, 0x6AC0, 0x6B80, 0xAB41, 0x6900, 0xA9C1, 0xA881, 0x6840,
        0x7800, 0xB8C1, 0xB981, 0x7940, 0xBB01, 0x7BC0, 0x7A80, 0xBA41,
        0xBE01, 0x7EC0, 0x7F80, 0xBF41, 0x7D00, 0xBDC1, 0xBC81, 0x7C40,
        0xB401, 0x74C0, 0x7580, 0xB541, 0x7700, 0xB7C1, 0xB681, 0x7640,
        0x7200, 0xB2C1, 0xB381, 0x7340, 0xB101, 0x71C0, 0x7080, 0xB041,
        0x5000, 0x90C1, 0x9181, 0x5140, 0x9301, 0x53C0, 0x5280, 0x9241,
        0x9601, 0x56C0, 0x5780, 0x9741, 0x5500, 0x95C1, 0x9481, 0x5440,
        0x9C01, 0x5CC0, 0x5D80, 0x9D41, 0x5F00, 0x9FC1, 0x9E81, 0x5E40,
        0x5A00, 0x9AC1, 0x9B81, 0x5B40, 0x9901, 0x59C0, 0x5880, 0x9841,
        0x8801, 0x48C0, 0x4980, 0x8941, 0x4B00, 0x8BC1, 0x8A81, 0x4A40,
        0x4E00, 0x8EC1, 0x8F81, 0x4F40, 0x8D01, 0x4DC0, 0x4C80, 0x8C41,
        0x4400, 0x84C1, 0x8581, 0x4540, 0x8701, 0x47C0, 0x4680, 0x8641,
        0x8201, 0x42C0, 0x4380, 0x8341, 0x4100, 0x81C1, 0x8081, 0x4040,
    ];

    let mut crc = seed;
    for &b in data {
        let index = (crc as u8) ^ b;
        crc = (crc >> 8) ^ CRC_TABLE[index as usize];
    }
    crc
}
