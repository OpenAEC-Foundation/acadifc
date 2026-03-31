/// Diagnostic: validate CRC and record structure for specific entities
/// in the roundtrip DWG file.
use acadrust::DwgReader;
use acadrust::io::dwg::crc;
use acadrust::io::dwg::dwg_stream_readers::handle_reader;
use std::collections::HashSet;

fn read_modular_short(data: &[u8]) -> (usize, usize) {
    let mut result: usize = 0;
    let mut shift = 0;
    let mut pos = 0;
    loop {
        if pos + 1 >= data.len() { return (0, pos); }
        let word = u16::from_le_bytes([data[pos], data[pos + 1]]);
        pos += 2;
        result |= ((word & 0x7FFF) as usize) << shift;
        shift += 15;
        if word & 0x8000 == 0 { break; }
    }
    (result, pos)
}

fn read_modular_char(data: &[u8]) -> (usize, usize) {
    let mut result: usize = 0;
    let mut shift = 0;
    let mut pos = 0;
    loop {
        if pos >= data.len() { return (0, pos); }
        let b = data[pos];
        pos += 1;
        result |= ((b & 0x7F) as usize) << shift;
        shift += 7;
        if b & 0x80 == 0 { break; }
    }
    (result, pos)
}

fn main() {
    let roundtrip_path = "target/General_roundtrip.dwg";
    
    // Read the roundtrip file to get section buffers
    let file_data = std::fs::read(roundtrip_path).expect("read file");
    println!("File size: {} bytes", file_data.len());
    
    // Use DwgReader to get section buffers
    let mut reader = DwgReader::from_file(roundtrip_path).expect("open");
    let doc = reader.read().expect("read");
    
    // Re-open to access sections
    let mut reader2 = DwgReader::from_file(roundtrip_path).expect("reopen");
    let info = reader2.read_info().expect("read info");
    
    let handles_buf = reader2.get_section_buffer("AcDb:Handles", &info).expect("handles section");
    let objects_buf = reader2.get_section_buffer("AcDb:AcDbObjects", &info).expect("objects section");
    
    println!("Objects section: {} bytes", objects_buf.len());
    println!("Handles section: {} bytes", handles_buf.len());
    
    let handle_map = handle_reader::read_handles(&handles_buf).expect("read handles");
    println!("Handle map entries: {}", handle_map.len());
    
    let failing: HashSet<u64> = [
        0x241B6, 0x241B9, 0x241BF, 0x241CE, 0x241E6,
        0x2432A, 0x2432D, 0x24330, 0x24333, 0x24345,
    ].iter().copied().collect();
    
    // Also check some passing LwPolyline handles
    let passing: HashSet<u64> = [
        0x36AB, 0x36AC, 0x36AF,  // first passing LwPolylines  
        0x241A5, 0x241A8, 0x241AE, // last passing LwPolylines
    ].iter().copied().collect();
    
    let check_handles: Vec<u64> = failing.iter().chain(passing.iter()).copied().collect();
    
    for &h in &check_handles {
        let is_fail = failing.contains(&h);
        let label = if is_fail { "FAIL" } else { "PASS" };
        
        let offset = match handle_map.get(&h) {
            Some(&o) => o as usize,
            None => {
                println!("{:#X} ({}): NOT IN HANDLE MAP", h, label);
                continue;
            }
        };
        
        if offset >= objects_buf.len() {
            println!("{:#X} ({}): offset {} >= section size {}", h, label, offset, objects_buf.len());
            continue;
        }
        
        // Read record
        let data = &objects_buf[offset..];
        let (size, ms_len) = read_modular_short(data);
        let (handle_bits, mc_len) = read_modular_char(&data[ms_len..]);
        let header_len = ms_len + mc_len;
        
        if offset + header_len + size + 2 > objects_buf.len() {
            println!("{:#X} ({}): record extends past section end", h, label);
            continue;
        }
        
        // CRC check
        let record_bytes = &objects_buf[offset..offset + header_len + size];
        let expected_crc = u16::from_le_bytes([
            objects_buf[offset + header_len + size],
            objects_buf[offset + header_len + size + 1],
        ]);
        let computed_crc = crc::crc16(crc::CRC16_SEED, record_bytes);
        
        // Read type code from merged data
        let merged = &data[header_len..header_len + size];
        let type_code = if size > 0 {
            // R2010 compact: BB + conditional
            let first_bits = merged[0];
            let pair = (first_bits >> 6) & 0x03;
            match pair {
                0 => (merged[0] & 0x3F) as i16 * 4 + ((merged[1] >> 6) as i16), // approximate
                _ => -1,
            }
        } else { -1 };
        
        let crc_ok = expected_crc == computed_crc;
        println!("{:#X} ({}): offset={} size={} handle_bits={} crc={} (expected={:#06X} computed={:#06X}) first_4_bytes={:02X?}",
            h, label, offset, size, handle_bits, 
            if crc_ok { "OK" } else { "MISMATCH!" },
            expected_crc, computed_crc,
            &merged[..4.min(size)]);
    }
}
