/// Diagnostic: dump the raw bitstream of a specific ACIS entity from a DWG file.
///
/// This reads the raw merged record bytes and manually parses the ACIS
/// header to understand how the data is structured.

use acadrust::io::dwg::crc::crc16;
use acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles;
use acadrust::io::dwg::DwgReader;
use std::collections::HashMap;
use std::io::Cursor;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        String::from(r"tests\roundtrip\samplekitchen.dwg")
    });
    // Target handle — one of the 128K Solid3D entities
    let target_handle: u64 = u64::from_str_radix(
        &std::env::args().nth(2).unwrap_or("22290D".to_string()),
        16,
    ).unwrap();

    let bytes = std::fs::read(&path).expect("Failed to read file");
    let mut reader = DwgReader::from_stream(Cursor::new(&bytes));
    let info = reader.read_file_header().expect("Failed to read header");

    let obj_data = reader.get_section_buffer("AcDb:AcDbObjects", &info)
        .expect("Failed to read objects section");
    let handle_data = reader.get_section_buffer("AcDb:Handles", &info)
        .expect("Failed to read handles section");
    let handle_map = read_handles(&handle_data).expect("Failed to parse handles");

    let offset = *handle_map.get(&target_handle).expect("Handle not found") as usize;

    // Read MS(size)
    let (size, ms_len) = read_modular_short(&obj_data[offset..]);
    let merged_data = &obj_data[offset + ms_len .. offset + ms_len + size];
    let crc_bytes = &obj_data[offset + ms_len + size .. offset + ms_len + size + 2];
    let stored_crc = u16::from_le_bytes([crc_bytes[0], crc_bytes[1]]);
    let computed_crc = crc16(0xC0C1, &obj_data[offset .. offset + ms_len + size]);

    println!("=== Entity at handle {:#X}, offset {} ===", target_handle, offset);
    println!("Record size (MS): {} bytes", size);
    println!("MS encoding: {} bytes", ms_len);
    println!("CRC: stored={:#06X} computed={:#06X} {}", stored_crc, computed_crc,
        if stored_crc == computed_crc { "OK" } else { "MISMATCH!" });

    // For AC21 three-stream merge, the layout is:
    // [type_code_BS][RL_value][---main_data---][---text_data---][flag_words][flag_bit][---handle_stream---]
    // The RL value = main_bits + text_bits + 1 + flag_words_bits
    // Handle stream starts at bit position RL from the start of merged data.

    println!("\nFirst 64 bytes of merged data (hex):");
    let show = merged_data.len().min(64);
    for i in 0..show {
        if i % 16 == 0 && i > 0 { println!(); }
        print!("{:02X} ", merged_data[i]);
    }
    println!("\n");

    // Parse type code (BS at bit 0)
    let mut bit_pos = 0usize;
    let type_code = read_bs(merged_data, &mut bit_pos);
    println!("Type code (BS): {} at bits 0..{}", type_code, bit_pos);

    // RL (raw long, 32 bits) — size value for three-stream merge
    let rl = read_rl(merged_data, &mut bit_pos);
    println!("RL value: {} (0x{:08X}) at bits {}..{}", rl, rl, bit_pos - 32, bit_pos);
    println!("  → Handle stream starts at bit {}", rl);
    println!("  → Total bits in merged data: {}", size * 8);

    // The main stream continues after the RL.
    // Now we're in the entity common data.
    println!("\n--- Entity Common Data ---");

    // Handle (BH)
    let handle = read_handle(merged_data, &mut bit_pos);
    println!("Handle: {:#X} at bit {}", handle, bit_pos);

    // EED (Extended Entity Data)
    let eed_size = read_bs(merged_data, &mut bit_pos);
    println!("EED size (BS): {} at bit {}", eed_size, bit_pos);
    if eed_size > 0 {
        println!("  (skipping {} bytes of EED)", eed_size);
        // Skip EED data
        let eed_bits = eed_size as usize * 8;
        // Simplified: skip handle + data
        // Actually EED is complex, just note we have it
        println!("  WARNING: EED present, manual parsing needed");
    }

    // After EED: graphic present (B)
    let graphic_present = read_bit(merged_data, &mut bit_pos);
    println!("Graphic present: {} at bit {}", graphic_present, bit_pos);

    // If graphic present, skip graphic data
    if graphic_present {
        let graphic_size = read_rl(merged_data, &mut bit_pos);
        println!("Graphic size: {} bytes", graphic_size);
        bit_pos += graphic_size as usize * 8;
        println!("(skipped graphic data, now at bit {})", bit_pos);
    }

    // Entity mode (BB)
    let entity_mode = read_bb(merged_data, &mut bit_pos);
    println!("Entity mode (BB): {} at bit {}", entity_mode, bit_pos);

    // Num reactors (BL)
    let num_reactors = read_bl(merged_data, &mut bit_pos);
    println!("Num reactors (BL): {} at bit {}", num_reactors, bit_pos);

    // R2004+: xdic_missing_flag (B)
    let xdic_missing = read_bit(merged_data, &mut bit_pos);
    println!("XDic missing: {} at bit {}", xdic_missing, bit_pos);

    // R2013+: has_ds (B) — only for version >= AC1027
    // For AC1021, this is not present.

    // Subentity reference (B)
    // R2004+: "isbylayerlt" B  -- actually no, let me look at the actual order

    // Actually the exact order depends on version. Let me just skip ahead
    // and show bits around where ACIS data should start.
    // The exact common entity data is complex, so let me just show the
    // next 200 bits as raw binary for manual inspection.

    println!("\n--- Raw bits from current position (bit {}) ---", bit_pos);
    print_bits(merged_data, bit_pos, 400);

    println!("\n\n--- Searching for ACIS marker patterns ---");
    // In SAT text (version 1), after acis_empty=0, unknown_bit, version_BS=1:
    // The first BL is the block size, which would be large (thousands).
    // In SAB binary (version 2), after acis_empty=0, unknown_bit, version_BS=2:
    // The BL is the total size.
    // Let's search for both patterns in the bit stream

    // Show the first 1000 bits of merged data    
    println!("\nFirst 200 bytes as bit groups:");
    for byte_i in 0..merged_data.len().min(200) {
        if byte_i % 20 == 0 { print!("\n  [{:4}] ", byte_i); }
        print!("{:08b} ", merged_data[byte_i]);
    }
    println!();
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

/// Read a single bit from packed MSB-first bitstream
fn read_bit(data: &[u8], pos: &mut usize) -> bool {
    let byte_idx = *pos / 8;
    let bit_idx = 7 - (*pos % 8); // MSB first
    *pos += 1;
    if byte_idx >= data.len() { return false; }
    (data[byte_idx] >> bit_idx) & 1 == 1
}

/// Read 2 bits (BB)
fn read_bb(data: &[u8], pos: &mut usize) -> u8 {
    let b1 = read_bit(data, pos) as u8;
    let b2 = read_bit(data, pos) as u8;
    (b1 << 1) | b2
}

/// Read BS (bit short)
fn read_bs(data: &[u8], pos: &mut usize) -> i16 {
    let mode = read_bb(data, pos);
    match mode {
        0b00 => {
            // raw 16-bit value
            let mut val: u16 = 0;
            for i in 0..16 {
                if read_bit(data, pos) { val |= 1 << i; }
            }
            val as i16
        }
        0b01 => {
            // raw 8-bit unsigned
            let mut val: u8 = 0;
            for i in 0..8 {
                if read_bit(data, pos) { val |= 1 << i; }
            }
            val as i16
        }
        0b10 => 0,
        0b11 => 256,
        _ => unreachable!(),
    }
}

/// Read BL (bit long)
fn read_bl(data: &[u8], pos: &mut usize) -> i32 {
    let mode = read_bb(data, pos);
    match mode {
        0b00 => {
            let mut val: u32 = 0;
            for i in 0..32 {
                if read_bit(data, pos) { val |= 1 << i; }
            }
            val as i32
        }
        0b01 => {
            let mut val: u8 = 0;
            for i in 0..8 {
                if read_bit(data, pos) { val |= 1 << i; }
            }
            val as i32
        }
        0b10 => 0,
        0b11 => {
            // BL mode 11: not standard — might mean something different
            // ODA says for BL: 00=raw 32, 01=raw 8, 10=0
            // Actually BL only has 3 modes (0b00, 0b01, 0b10)
            // Mode 0b11 is not defined for BL
            0
        }
        _ => unreachable!(),
    }
}

/// Read RL (raw long, 32 bits, little-endian)
fn read_rl(data: &[u8], pos: &mut usize) -> u32 {
    let mut val: u32 = 0;
    for i in 0..32 {
        if read_bit(data, pos) { val |= 1 << i; }
    }
    val
}

/// Read handle (DWG handle encoding)
fn read_handle(data: &[u8], pos: &mut usize) -> u64 {
    // Handle: 4-bit code + 4-bit counter, then counter bytes
    let mut code_counter: u8 = 0;
    for i in 0..8 {
        if read_bit(data, pos) { code_counter |= 1 << i; }
    }
    let _code = code_counter >> 4;
    let counter = (code_counter & 0x0F) as usize;
    let mut handle: u64 = 0;
    for _ in 0..counter {
        let mut byte: u8 = 0;
        for i in 0..8 {
            if read_bit(data, pos) { byte |= 1 << i; }
        }
        handle = (handle << 8) | byte as u64;
    }
    handle
}

fn print_bits(data: &[u8], start_bit: usize, count: usize) {
    for i in 0..count {
        let bit = start_bit + i;
        let byte_idx = bit / 8;
        let bit_idx = 7 - (bit % 8);
        if byte_idx >= data.len() { break; }
        if i > 0 && i % 50 == 0 { println!(); }
        if i > 0 && i % 10 == 0 && i % 50 != 0 { print!(" "); }
        let val = (data[byte_idx] >> bit_idx) & 1;
        print!("{}", val);
    }
    println!();
}
