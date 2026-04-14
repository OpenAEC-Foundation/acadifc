/// Diagnostic: Compare R2007 three-stream merge between original and RT.
/// Reads both files and prints the RL (total_size_bits) field for each
/// POLYLINE_PFACE entity (type 29).

use acadrust::io::dwg::DwgReader;
use acadrust::io::dwg::DwgWriter;

fn dump_rl_values(label: &str, path: &str) {
    let mut reader = match DwgReader::from_file(path) {
        Ok(r) => r,
        Err(e) => { eprintln!("Failed to open {}: {:?}", path, e); return; }
    };

    let info = match reader.read_file_header() {
        Ok(i) => i,
        Err(e) => { eprintln!("Failed to read header: {:?}", e); return; }
    };

    let metadata = info.ac21_metadata.as_ref().expect("AC21");
    let dxf_version = reader.dxf_version();

    // Read raw objects section
    let (objects_section, handle_map) = reader.read_raw_objects_and_handles().expect("sections");

    println!("=== {} ({}) ===", label, path);
    println!("  Objects section: {} bytes", objects_section.len());
    println!("  Handle entries: {}", handle_map.len());

    // Find POLYLINE_PFACE objects (type 29) and dump RL
    let mut pos = 0usize;
    let mut count = 0;
    let mut pface_count = 0;

    while pos < objects_section.len() {
        // Read ModularShort (object size)
        let start = pos;
        let mut obj_size: usize = 0;
        let mut shift = 0;
        loop {
            if pos + 1 >= objects_section.len() { break; }
            let word = (objects_section[pos] as u16) | ((objects_section[pos + 1] as u16) << 8);
            pos += 2;
            obj_size |= ((word & 0x7FFF) as usize) << shift;
            if (word & 0x8000) == 0 { break; }
            shift += 15;
        }

        if obj_size == 0 { break; }
        let data_start = pos;
        let data_end = (data_start + obj_size).min(objects_section.len());
        let data = &objects_section[data_start..data_end];

        // Skip CRC (2 bytes after data)
        pos = data_end + 2;
        count += 1;

        // Decode type code (BS)
        if data.len() < 2 { continue; }
        let first_byte = data[0];
        let bits01 = first_byte >> 6;
        let type_code: i16 = match bits01 {
            0b00 => {
                if data.len() >= 3 {
                    let raw = ((first_byte & 0x3F) as u16)
                        | ((data[1] as u16) << 6)
                        | (((data[2] & 0x03) as u16) << 14);
                    raw as i16
                } else { continue; }
            },
            0b01 => {
                ((first_byte & 0x3F) << 2 | (data[1] >> 6)) as i16
            },
            0b10 => 0,
            0b11 => 256,
            _ => continue,
        };

        // Only care about POLYLINE_PFACE (type 29) and first few of each type
        if type_code == 29 {
            // Read RL (raw long = 4 bytes at known bit position)
            // After type code BS, the RL follows. For type 29 with BS encoding 01:
            // BS takes: 2 bits (selector) + 8 bits (value) = 10 bits total.
            // For BS encoding 00: 2 + 16 = 18 bits.
            // RL is 4 bytes (32 bits) at the next bit position.
            // Just decode it from the bit stream.
            let mut bit_pos: usize = match bits01 {
                0b01 => 10,  // 2 + 8 bits 
                0b00 => 18,  // 2 + 16 bits
                0b10 => 2,   // just the 2 bits
                0b11 => 2,   // just the 2 bits
                _ => 0,
            };

            // Read 32-bit RL at bit_pos
            let byte_idx = bit_pos / 8;
            let bit_shift = bit_pos % 8;
            if byte_idx + 4 < data.len() {
                let mut rl_bytes = [0u8; 4];
                for i in 0..4 {
                    let b0 = data[byte_idx + i];
                    let b1 = data[byte_idx + i + 1];
                    rl_bytes[i] = (b0 >> bit_shift) as u8 | ((b1 as u8) << (8 - bit_shift));
                }
                let rl = u32::from_le_bytes(rl_bytes);

                pface_count += 1;
                if pface_count <= 5 {
                    println!("  POLYLINE_PFACE #{}: obj_size={}, RL={}, data_hex={}",
                        pface_count, obj_size, rl,
                        data[..data.len().min(32)].iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "));
                }
            }
        }
    }

    println!("  Total objects scanned: {}", count);
    println!("  POLYLINE_PFACE found: {}", pface_count);
    println!();
}

fn main() {
    let orig = std::env::args().nth(1).unwrap_or_else(|| {
        "tests/roundtrip/samplekitchen.dwg".to_string()
    });

    dump_rl_values("ORIGINAL", &orig);

    // Write RT
    let rt_path = "tests/roundtrip/samplekitchen_rt.dwg";
    let mut reader = DwgReader::from_file(&orig).expect("open");
    let doc = reader.read().expect("read");
    DwgWriter::write_to_file(rt_path, &doc).expect("write");

    dump_rl_values("ROUNDTRIP", rt_path);
}
