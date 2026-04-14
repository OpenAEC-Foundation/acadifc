/// Diagnostic: verify AC1021 page-level CRC-64 and Adler-32 checksums.
///
/// For each section page in the file, this reads the raw RS-encoded bytes
/// from disk, RS-decodes them, and recomputes both the CRC-64 and Adler-32.
/// It then compares these computed values with what is stored in the section map.
///
/// This catches any mismatch between what we write and what we claim in the section map.

use acadrust::io::dwg::DwgReader;
use acadrust::io::dwg::crc::{dwg_ac21_mirrored_crc64, dwg_ac21_page_checksum};
use acadrust::io::dwg::reed_solomon::reed_solomon_decode;
use acadrust::io::dwg::decompressor_ac21::decompress_ac21;

use std::io::{Read, Seek, SeekFrom};
use std::fs::File;

const AC21_FILE_HEADER_SIZE: u64 = 0x480;
const RS_DATA_K: usize = 251;
const RS_SYSTEM_K: usize = 239;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        String::from(r"tests\roundtrip\samplekitchen_rt.dwg")
    });

    println!("=== Page CRC verification: {} ===", path);

    let mut reader = match DwgReader::from_file(&path) {
        Ok(r) => r,
        Err(e) => { eprintln!("Failed to open: {:?}", e); return; }
    };

    let info = match reader.read_file_header() {
        Ok(i) => i,
        Err(e) => { eprintln!("Failed to read header: {:?}", e); return; }
    };

    let mut file = match File::open(&path) {
        Ok(f) => f,
        Err(e) => { eprintln!("Failed to open file: {:?}", e); return; }
    };

    let metadata = match &info.ac21_metadata {
        Some(m) => m.clone(),
        None => { eprintln!("Not an AC21 file"); return; }
    };

    let crc_seed = metadata.crc_seed;

    let mut total = 0;
    let mut crc_ok = 0;
    let mut crc_fail = 0;
    let mut checksum_ok = 0;
    let mut checksum_fail = 0;

    for section in &info.section_descriptors {
        for page in &section.pages {
            let page_num = page.page_number as i32;
            let file_offset = match info.page_records.get(&page_num) {
                Some(&(off, _)) => off,
                None => {
                    println!("  [{}] page {} - NOT IN PAGE MAP", section.name, page_num);
                    continue;
                }
            };

            let abs_offset = AC21_FILE_HEADER_SIZE + file_offset as u64;
            let encoding = section.encoding;

            let compressed_size = page.compressed_size;
            let uncompressed_size = page.decompressed_size;
            let stored_crc = page.crc;
            let stored_checksum = page.checksum;

            total += 1;

            if encoding == 1 {
                // Encoding=1: raw data, no RS, no LZ77
                let read_size = uncompressed_size as usize;
                file.seek(SeekFrom::Start(abs_offset)).unwrap();
                let mut raw = vec![0u8; read_size];
                file.read_exact(&mut raw).unwrap_or_else(|_| {
                    // pad with zeros if short
                });

                let computed_crc = dwg_ac21_mirrored_crc64(0, raw.len() as u32, &raw);
                let computed_checksum = dwg_ac21_page_checksum(0, &raw) as u64;

                if computed_crc == stored_crc {
                    crc_ok += 1;
                } else {
                    println!("  CRC MISMATCH [{}] page {} enc=1: stored={:#018X} computed={:#018X}",
                        section.name, page_num, stored_crc, computed_crc);
                    crc_fail += 1;
                }
                if computed_checksum == stored_checksum {
                    checksum_ok += 1;
                } else {
                    println!("  CHECKSUM MISMATCH [{}] page {} enc=1: stored={:#018X} computed={:#018X}",
                        section.name, page_num, stored_checksum, computed_checksum);
                    checksum_fail += 1;
                }
            } else {
                // Encoding=4: RS-encoded + LZ77-compressed
                // Step 1: Compute RS decoding parameters  (must match reader)
                let v = compressed_size.wrapping_add(7);
                let v1 = v & 0xFFFF_FFF8;
                let total_rs_data = v1 as usize; // correction_factor=1 for data pages
                let factor = (total_rs_data + RS_DATA_K - 1) / RS_DATA_K;
                let read_length = factor * 255;

                // Step 2: Read RS-encoded bytes from file
                file.seek(SeekFrom::Start(abs_offset)).unwrap();
                let mut encoded = vec![0u8; read_length];
                let n = file.read(&mut encoded).unwrap_or(0);
                if n < read_length {
                    encoded[n..].fill(0);
                }

                // Step 3: RS-decode
                let mut compressed_data = vec![0u8; total_rs_data];
                reed_solomon_decode(&encoded, &mut compressed_data, factor, RS_DATA_K);

                // Step 4: Compute CRC-64 on compressed data (first compressed_size bytes)
                let cs = compressed_size as usize;
                let crc_data = if cs <= compressed_data.len() {
                    &compressed_data[..cs]
                } else {
                    &compressed_data[..]
                };
                let computed_crc = dwg_ac21_mirrored_crc64(0, cs as u32, crc_data);

                if computed_crc == stored_crc {
                    crc_ok += 1;
                } else {
                    println!("  CRC MISMATCH [{}] page {} enc=4: stored={:#018X} computed={:#018X} (comp_sz={}, v1={})",
                        section.name, page_num, stored_crc, computed_crc, compressed_size, v1);
                    crc_fail += 1;
                }

                // Step 5: Decompress to get raw data (for checksum)
                let raw_data = if compressed_size != uncompressed_size {
                    let src_padded_size = compressed_data.len() + 64;
                    let mut padded_source = vec![0u8; src_padded_size];
                    padded_source[..compressed_data.len()].copy_from_slice(&compressed_data);

                    let dst_padded_size = uncompressed_size as usize + 64;
                    let mut decompressed = vec![0u8; dst_padded_size];
                    decompress_ac21(&padded_source, 0, compressed_size as u32, &mut decompressed);
                    decompressed.truncate(uncompressed_size as usize);
                    decompressed
                } else {
                    compressed_data[..uncompressed_size as usize].to_vec()
                };

                let computed_checksum = dwg_ac21_page_checksum(0, &raw_data) as u64;

                if computed_checksum == stored_checksum {
                    checksum_ok += 1;
                } else {
                    println!("  CHECKSUM MISMATCH [{}] page {} enc=4: stored={:#018X} computed={:#018X}",
                        section.name, page_num, stored_checksum, computed_checksum);
                    checksum_fail += 1;
                }
            }
        }
    }

    println!("\n=== Summary ===");
    println!("Total pages: {}", total);
    println!("CRC-64:    OK={} FAIL={}", crc_ok, crc_fail);
    println!("Adler-32:  OK={} FAIL={}", checksum_ok, checksum_fail);

    // Also verify the page map and section map system page CRCs
    println!("\n=== System page meta-CRCs ===");
    println!("Pages map CRC compressed:    {:#018X}", metadata.pages_map_crc_compressed);
    println!("Pages map CRC uncompressed:  {:#018X}", metadata.pages_map_crc_uncompressed);
    println!("Sections map CRC compressed:   {:#018X}", metadata.sections_map_crc_compressed);
    println!("Sections map CRC uncompressed: {:#018X}", metadata.sections_map_crc_uncompressed);
    println!("CRC seed: {:#018X}", crc_seed);
    println!("(Note: system page CRC verification requires re-reading them; shown for info only)");

    if crc_fail == 0 && checksum_fail == 0 {
        println!("\nAll page CRCs and Adler-32 checksums are CORRECT.");
    } else {
        println!("\nFound {} CRC failures and {} checksum failures!", crc_fail, checksum_fail);
    }

    // Verify system page (page map + section map) CRCs
    println!("\n=== System page CRC verification ===");

    // Page map: offset stored directly in metadata
    verify_system_page_crc("page map", &mut file,
        AC21_FILE_HEADER_SIZE + metadata.pages_map_offset,
        metadata.pages_map_size_compressed,
        metadata.pages_map_size_uncompressed,
        metadata.pages_map_correction_factor,
        metadata.pages_map_crc_compressed,
        metadata.pages_map_crc_uncompressed,
        metadata.crc_seed);

    // Section map: offset found via page_records[sections_map_id]
    let sm_id = metadata.sections_map_id as i32;
    if let Some(&(sm_offset, _)) = info.page_records.get(&sm_id) {
        verify_system_page_crc("section map", &mut file,
            AC21_FILE_HEADER_SIZE + sm_offset as u64,
            metadata.sections_map_size_compressed,
            metadata.sections_map_size_uncompressed,
            metadata.sections_map_correction_factor,
            metadata.sections_map_crc_compressed,
            metadata.sections_map_crc_uncompressed,
            metadata.crc_seed);
    } else {
        println!("  section map - page ID {} not found in page records", sm_id);
    }
}

fn verify_system_page_crc(
    name: &str,
    file: &mut File,
    abs_offset: u64,
    compressed_size: u64,
    uncompressed_size: u64,
    correction_factor: u64,
    stored_crc_comp: u64,
    stored_crc_uncomp: u64,
    crc_seed: u64,
) {
    // Compute RS decoding parameters for system pages (block_size=239)
    let v = compressed_size.wrapping_add(7);
    let v1 = (v & 0xFFFF_FFF8) as usize;
    let total_rs_data = v1 * correction_factor as usize;
    let factor = (total_rs_data + RS_SYSTEM_K - 1) / RS_SYSTEM_K;
    let read_length = factor * 255;

    file.seek(SeekFrom::Start(abs_offset)).unwrap();
    let mut encoded = vec![0u8; read_length];
    let n = file.read(&mut encoded).unwrap_or(0);
    if n < read_length { encoded[n..].fill(0); }

    let mut compressed_data = vec![0u8; total_rs_data];
    reed_solomon_decode(&encoded, &mut compressed_data, factor, RS_SYSTEM_K);

    let cs = compressed_size as usize;
    let crc_data = if cs <= compressed_data.len() { &compressed_data[..cs] } else { &compressed_data[..] };
    let computed_crc_comp = dwg_ac21_mirrored_crc64(crc_seed, cs as u32, crc_data);

    let raw_data = if compressed_size != uncompressed_size {
        let mut padded = vec![0u8; compressed_data.len() + 64];
        padded[..compressed_data.len()].copy_from_slice(&compressed_data);
        let dst_size = uncompressed_size as usize + 64;
        let mut decompressed = vec![0u8; dst_size];
        decompress_ac21(&padded, 0, compressed_size as u32, &mut decompressed);
        decompressed.truncate(uncompressed_size as usize);
        decompressed
    } else {
        compressed_data[..uncompressed_size as usize].to_vec()
    };

    let computed_crc_uncomp = dwg_ac21_mirrored_crc64(crc_seed, raw_data.len() as u32, &raw_data);

    let comp_ok = computed_crc_comp == stored_crc_comp;
    let uncomp_ok = computed_crc_uncomp == stored_crc_uncomp;

    println!("  {} compressed CRC: stored={:#018X} computed={:#018X} {}",
        name, stored_crc_comp, computed_crc_comp, if comp_ok { "OK" } else { "MISMATCH!" });
    println!("  {} uncompressed CRC: stored={:#018X} computed={:#018X} {}",
        name, stored_crc_uncomp, computed_crc_uncomp, if uncomp_ok { "OK" } else { "MISMATCH!" });
}
