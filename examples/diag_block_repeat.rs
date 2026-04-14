/// Diagnostic: Check whether the file header's pre-RS block is repeated (spec §5.2.1.5).
use std::fs;
use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt};

use acadrust::io::dwg::reed_solomon::reed_solomon_decode;

const METADATA_BLOCK_SIZE: usize = 0x80;
const FILE_HEADER_PAGE_SIZE: usize = 0x400;
const RS_SYSTEM_K: usize = 239;
const RS_N: usize = 255;
const FILE_HEADER_RS_FACTOR: usize = 3;

fn check_file(path: &str) {
    println!("=== {} ===", path);

    let file_data = fs::read(path).expect("Failed to read file");
    let fh_page = &file_data[METADATA_BLOCK_SIZE..METADATA_BLOCK_SIZE + FILE_HEADER_PAGE_SIZE];

    // RS decode: factor=3, k=239 → 717 decoded bytes
    let decoded_size = FILE_HEADER_RS_FACTOR * RS_SYSTEM_K;
    let mut decoded = vec![0u8; decoded_size];
    reed_solomon_decode(fh_page, &mut decoded, FILE_HEADER_RS_FACTOR, RS_SYSTEM_K);

    // Read compr_len from offset 24
    let mut cursor = Cursor::new(&decoded);
    let _check_seq_crc = cursor.read_u64::<LittleEndian>().unwrap();
    let _check_seq_val1 = cursor.read_u64::<LittleEndian>().unwrap();
    let _compr_crc = cursor.read_u64::<LittleEndian>().unwrap();
    let compr_len = cursor.read_i32::<LittleEndian>().unwrap();
    let _length2 = cursor.read_i32::<LittleEndian>().unwrap();

    let data_bytes = if compr_len < 0 { (-compr_len) as usize } else { compr_len as usize };
    let block_raw = 32 + data_bytes;
    let block_padded = (block_raw + 7) & !7;

    println!("  compr_len: {}", compr_len);
    println!("  block raw: {} bytes, padded: {} bytes", block_raw, block_padded);
    let max_copies = decoded_size / block_padded;
    println!("  max copies in {} bytes: {}", decoded_size, max_copies);

    // Check if block repeats
    let first_block = &decoded[..block_padded];
    for copy_idx in 1..max_copies {
        let start = copy_idx * block_padded;
        let end = start + block_padded;
        if end > decoded_size {
            break;
        }
        let this_copy = &decoded[start..end];
        let matches = first_block == this_copy;
        println!("  copy[{}] at {}-{}: {}", copy_idx, start, end,
                 if matches { "MATCHES first block" } else { "DIFFERS from first block" });

        if !matches {
            // Show first difference
            for i in 0..block_padded {
                if first_block[i] != this_copy[i] {
                    println!("    first diff at offset {}: block[0]={:#04X} vs block[{}]={:#04X}",
                             i, first_block[i], copy_idx, this_copy[i]);
                    break;
                }
            }
        }
    }

    // Also check what's after the last copy
    let used = max_copies * block_padded;
    let remaining = decoded_size - used;
    println!("  remaining after {} copies: {} bytes (random padding)", max_copies, remaining);

    // Hexdump first 40 bytes of decoded data
    println!("\n  First 40 bytes of decoded data:");
    for i in (0..40).step_by(16) {
        let end = (i + 16).min(decoded_size);
        let hex: Vec<String> = decoded[i..end].iter().map(|b| format!("{:02x}", b)).collect();
        println!("    {:04X}: {}", i, hex.join(" "));
    }

    println!();
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: cargo run --example diag_block_repeat -- <file1.dwg> [file2.dwg ...]");
        std::process::exit(2);
    }
    for path in &paths {
        check_file(path);
    }
}
