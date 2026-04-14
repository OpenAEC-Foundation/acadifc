/// Diagnostic: verify section-level CRC-16 in Header and Classes sections.
///
/// For AC1021 (R2007), each decompressed section ends with start_sentinel +
/// [RL(size)] + [data] + [CRC16] + end_sentinel.
/// This diagnostic extracts those CRC bytes and re-verifies them.

use acadrust::io::dwg::DwgReader;
use acadrust::io::dwg::crc::{crc16, CRC16_SEED};
use acadrust::io::dwg::file_headers::section_definition::start_sentinels;

fn verify_section_crc(section_name: &str, data: &[u8]) {
    if data.len() < 20 {
        println!("  {} - too short ({} bytes)", section_name, data.len());
        return;
    }

    // Check start sentinel (16 bytes)
    let sentinel_matches = match section_name {
        "AcDb:Header" => &data[..16] == &start_sentinels::HEADER,
        "AcDb:Classes" => &data[..16] == &start_sentinels::CLASSES,
        _ => {
            println!("  {} - no sentinel check for this section", section_name);
            return;
        }
    };

    if !sentinel_matches {
        println!("  {} - START SENTINEL MISMATCH!", section_name);
    } else {
        println!("  {} - start sentinel OK", section_name);
    }

    // Read section size at offset 16 (RL = 4 bytes LE)
    let section_size = i32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;
    let data_start = 20; // 16 (sentinel) + 4 (size)

    println!("  {} - section_size = {} bytes", section_name, section_size);

    if data_start + section_size + 2 > data.len() {
        println!("  {} - data too short for declared size (need {} have {})", 
            section_name, data_start + section_size + 2, data.len());
        return;
    }

    // Compute expected CRC over [size_field(4)] + [data(section_size)]
    let crc_input = &data[16..data_start + section_size];
    let computed_crc = crc16(CRC16_SEED, crc_input);

    // Read stored CRC (2 bytes LE)
    let crc_start = data_start + section_size;
    let stored_crc = u16::from_le_bytes([data[crc_start], data[crc_start + 1]]);

    if computed_crc == stored_crc {
        println!("  {} - CRC-16 OK (stored=computed={:#06X})", section_name, stored_crc);
    } else {
        println!("  {} - CRC-16 MISMATCH! stored={:#06X} computed={:#06X}",
            section_name, stored_crc, computed_crc);
    }
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        String::from(r"tests\roundtrip\samplekitchen_rt.dwg")
    });

    println!("=== Section-level CRC-16 Verification: {} ===", path);

    let mut reader = match DwgReader::from_file(&path) {
        Ok(r) => r,
        Err(e) => { eprintln!("Failed to open: {:?}", e); return; }
    };

    let info = match reader.read_file_header() {
        Ok(i) => i,
        Err(e) => { eprintln!("Failed to read header: {:?}", e); return; }
    };

    for section_name in &["AcDb:Header", "AcDb:Classes"] {
        match reader.get_section_buffer(section_name, &info) {
            Ok(data) => {
                println!("\nSection '{}' ({} bytes):", section_name, data.len());
                verify_section_crc(section_name, &data);
            },
            Err(e) => {
                println!("Failed to read {}: {:?}", section_name, e);
            }
        }
    }

    println!("\nDone.");
}
