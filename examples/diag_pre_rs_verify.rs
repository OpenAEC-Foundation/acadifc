/// Verify the pre-RS block CRCs in the file header page.
///
/// The file header page (at file offset 0x80) contains:
/// - Bytes 0x000..0x3D8: RS(255,239)×3 = 765 bytes, RS-encoded pre-RS block of 717 bytes
/// - Bytes 0x3D8..0x400: check_data (plaintext)
///
/// The pre-RS block has:
/// - [0x00..0x08]: sequence_crc    = dwg_ac21_normal_crc64_seed1(0, 16, [val1, val2])
/// - [0x08..0x10]: sequence_key    = check_seq_val1
/// - [0x10..0x18]: compr_crc       = dwg_ac21_normal_crc64(0, compr_len, compr_data)
/// - [0x18..0x1C]: compr_len
/// - [0x20..] : compressed metadata

use acadrust::io::dwg::crc::{dwg_ac21_normal_crc64, dwg_ac21_normal_crc64_seed1, dwg_ac21_header_crc64};
use acadrust::io::dwg::reed_solomon::reed_solomon_decode;
use acadrust::io::dwg::decompressor_ac21::decompress_ac21;
use std::fs;

fn encode_value(value: u64, control: u64) -> u64 {
    let shift = (control & 0x1F) as u32;
    if shift != 0 {
        (value << shift) | (value >> (64 - shift))
    } else {
        value
    }
}

fn verify_file(path: &str) {
    println!("=== Pre-RS block CRC verification: {} ===", path);
    let data = fs::read(path).expect("Failed to read file");

    // Read the 0x400-byte header page at file offset 0x80
    let page_start = 0x80usize;
    let page_end = page_start + 0x400;
    if data.len() < page_end {
        println!("  ERROR: file too short");
        return;
    }
    let header_page = &data[page_start..page_end];

    // Read the first 0x3D8 bytes (RS-encoded pre-RS block)
    let rs_encoded = &header_page[..0x3D8];
    let mut pre_rs = vec![0u8; 3 * 239]; // 717 bytes
    reed_solomon_decode(rs_encoded, &mut pre_rs, 3, 239);

    // Extract sequence fields
    let seq_crc_stored = u64::from_le_bytes(pre_rs[0..8].try_into().unwrap());
    let seq_key = u64::from_le_bytes(pre_rs[8..16].try_into().unwrap()); // check_seq_val1
    let compr_crc_stored = u64::from_le_bytes(pre_rs[16..24].try_into().unwrap());
    let compr_len = i32::from_le_bytes(pre_rs[24..28].try_into().unwrap());

    println!("  seq_crc_stored   = {:#018X}", seq_crc_stored);
    println!("  seq_key (val1)   = {:#018X}", seq_key);
    println!("  compr_crc_stored = {:#018X}", compr_crc_stored);
    println!("  compr_len        = {}", compr_len);

    // Verify sequence CRC:
    // val2 = encode_value(val1, val1)
    // seq_bytes = [val1_le, val2_le] (16 bytes)
    // seq_crc = dwg_ac21_normal_crc64_seed1(0, 16, &seq_bytes)
    let val2 = encode_value(seq_key, seq_key);
    let mut seq_bytes = [0u8; 16];
    seq_bytes[0..8].copy_from_slice(&seq_key.to_le_bytes());
    seq_bytes[8..16].copy_from_slice(&val2.to_le_bytes());
    let seq_crc_computed = dwg_ac21_normal_crc64_seed1(0, 16, &seq_bytes);

    if seq_crc_computed == seq_crc_stored {
        println!("  sequence_crc: OK (computed == stored = {:#018X})", seq_crc_stored);
    } else {
        println!("  sequence_crc: MISMATCH  stored={:#018X}  computed={:#018X}", seq_crc_stored, seq_crc_computed);
    }

    // Verify compr_crc:
    // compr_data starts at pre_rs[0x20]
    // compr_len bytes of compressed data
    let data_start = 0x20usize;
    let actual_compr_len = if compr_len < 0 { (-compr_len) as usize } else { compr_len as usize };
    if data_start + actual_compr_len > pre_rs.len() {
        println!("  compr_crc: SKIP (compr_len={} exceeds pre_rs bounds)", actual_compr_len);
    } else {
        let compr_data = &pre_rs[data_start..data_start + actual_compr_len];
        let compr_crc_computed = dwg_ac21_normal_crc64(0, actual_compr_len as u32, compr_data);
        if compr_crc_computed == compr_crc_stored {
            println!("  compr_crc: OK (computed == stored = {:#018X})", compr_crc_stored);
        } else {
            println!("  compr_crc: MISMATCH  stored={:#018X}  computed={:#018X}", compr_crc_stored, compr_crc_computed);
        }

        // Verify header_crc64:
        // Decompress the metadata block and compute dwg_ac21_header_crc64
        let mut meta_buf = vec![0u8; 0x110 + 64];
        let decomp_len = if compr_len < 0 { (-compr_len) as u32 } else { compr_len as u32 };
        let mut padded_src = pre_rs[data_start..].to_vec();
        padded_src.resize(padded_src.len() + 64, 0);
        decompress_ac21(&padded_src, 0, decomp_len, &mut meta_buf);
        meta_buf.truncate(0x110);
        if meta_buf.len() == 0x110 {
            let stored_header_crc = u64::from_le_bytes(meta_buf[0x108..0x110].try_into().unwrap());
            let computed_header_crc = dwg_ac21_header_crc64(&meta_buf);
            if computed_header_crc == stored_header_crc {
                println!("  header_crc64: OK (computed == stored = {:#018X})", stored_header_crc);
            } else {
                println!("  header_crc64: MISMATCH  stored={:#018X}  computed={:#018X}", stored_header_crc, computed_header_crc);
            }
        }
    }
    println!();
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("Usage: diag_pre_rs_verify <file.dwg> [file2.dwg ...]");
        std::process::exit(1);
    }
    for p in &paths {
        verify_file(p);
    }
}
