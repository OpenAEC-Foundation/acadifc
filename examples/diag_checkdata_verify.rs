/// Verify R2007 file header check_data integrity.
///
/// The check_data block at file offset 0x458 (= 0x80 + 0x3D8) consists of:
///   normal_crc (8), mirrored_crc (8), random1 (8), random2 (8), encoded_crc_seed (8)
///
/// We re-derive normal_crc and mirrored_crc from random1+random2
/// and compare to stored values, using the same computation as the writer.

use std::io::{Read, Seek, SeekFrom};
use std::fs::File;
use acadrust::io::dwg::crc::{dwg_ac21_normal_crc64, dwg_ac21_mirrored_crc64, dwg_ac21_normal_crc64_seed1};

const FILE_HEADER_OFFSET: u64 = 0x80;
const CHECK_DATA_OFFSET_IN_PAGE: u64 = 0x3D8;
const CHECK_DATA_ABS: u64 = FILE_HEADER_OFFSET + CHECK_DATA_OFFSET_IN_PAGE; // 0x458

fn encode_value(value: u64, control: u64) -> u64 {
    let shift = (control & 0x1F) as u32;
    if shift != 0 {
        (value << shift) | (value >> (64 - shift))
    } else {
        value
    }
}

fn compute_normal_crc(random1: u64, random2: u64) -> u64 {
    let mut buf = [0u64; 8];
    buf[0] = encode_value(random1, random2);
    buf[1] = encode_value(buf[0], buf[0]);
    buf[2] = encode_value(random2, buf[1]);
    buf[3] = encode_value(buf[2], buf[2]);
    buf[4] = encode_value(random1, buf[3]);
    buf[5] = encode_value(buf[4], buf[4]);
    buf[6] = encode_value(buf[5], buf[5]);
    buf[7] = encode_value(buf[6], buf[6]);

    let mut bytes = [0u8; 64];
    for (i, &val) in buf.iter().enumerate() {
        bytes[i * 8..(i + 1) * 8].copy_from_slice(&val.to_le_bytes());
    }
    dwg_ac21_normal_crc64(random2, 64, &bytes)
}

fn compute_mirrored_crc(random1: u64, random2: u64, normal_crc: u64) -> u64 {
    let mut buf = [0u64; 8];
    buf[0] = encode_value(random1, random2);
    buf[1] = encode_value(normal_crc, buf[0]);
    buf[2] = encode_value(random2, buf[1]);
    buf[3] = encode_value(normal_crc, buf[2]);
    buf[4] = encode_value(random1, buf[3]);
    buf[5] = encode_value(normal_crc, buf[4]);
    buf[6] = encode_value(random2, buf[5]);
    buf[7] = encode_value(buf[6], buf[6]);

    let mut bytes = [0u8; 64];
    for (i, &val) in buf.iter().enumerate() {
        bytes[i * 8..(i + 1) * 8].copy_from_slice(&val.to_le_bytes());
    }
    dwg_ac21_mirrored_crc64(random1, 64, &bytes)
}

fn verify_file(path: &str) {
    println!("=== Check_data verification: {} ===", path);

    let mut file = File::open(path).expect("Failed to open file");
    let mut check_data = [0u8; 0x28];
    file.seek(SeekFrom::Start(CHECK_DATA_ABS)).unwrap();
    file.read_exact(&mut check_data).unwrap();

    let normal_crc_stored = u64::from_le_bytes(check_data[0..8].try_into().unwrap());
    let mirrored_crc_stored = u64::from_le_bytes(check_data[8..16].try_into().unwrap());
    let r1 = u64::from_le_bytes(check_data[16..24].try_into().unwrap());
    let r2 = u64::from_le_bytes(check_data[24..32].try_into().unwrap());
    let encoded_crc_seed = u64::from_le_bytes(check_data[32..40].try_into().unwrap());

    println!("  R1 (check_data[16..24]) = {:#018X}", r1);
    println!("  R2 (check_data[24..32]) = {:#018X}", r2);
    println!("  encoded_crc_seed        = {:#018X}", encoded_crc_seed);
    println!("  stored normal_crc       = {:#018X}", normal_crc_stored);
    println!("  stored mirrored_crc     = {:#018X}", mirrored_crc_stored);

    // Try all combinations: different seeds, different algorithms
    let combinations: &[(&str, u64, u64, bool)] = &[
        // (label, seed_for_normal, seed_for_mirrored, use_r1_r2_swap)
        ("seed2(R2), seed1(R1)", r2, r1, false),
        ("seed2(R1), seed1(R2)", r1, r2, false),
        ("seed1(R2), seed1(R1)", r2, r1, true),  // use seed1 for both
        ("seed1(R1), seed1(R2)", r1, r2, true),
    ];

    for &(label, norm_seed, mirr_seed, use_seed1_for_norm) in combinations {
        // Compute normal_crc buffer (uses R1, R2)
        let mut nbuf = [0u64; 8];
        nbuf[0] = encode_value(r1, r2);
        nbuf[1] = encode_value(nbuf[0], nbuf[0]);
        nbuf[2] = encode_value(r2, nbuf[1]);
        nbuf[3] = encode_value(nbuf[2], nbuf[2]);
        nbuf[4] = encode_value(r1, nbuf[3]);
        nbuf[5] = encode_value(nbuf[4], nbuf[4]);
        nbuf[6] = encode_value(nbuf[5], nbuf[5]);
        nbuf[7] = encode_value(nbuf[6], nbuf[6]);
        let mut nbytes = [0u8; 64];
        for (i, &v) in nbuf.iter().enumerate() { nbytes[i*8..(i+1)*8].copy_from_slice(&v.to_le_bytes()); }
        
        let computed_normal = if use_seed1_for_norm {
            dwg_ac21_normal_crc64_seed1(norm_seed, 64, &nbytes)
        } else {
            dwg_ac21_normal_crc64(norm_seed, 64, &nbytes)
        };

        // Compute mirrored_crc buffer (uses R1, R2, and computed_normal)
        let mut mbuf = [0u64; 8];
        mbuf[0] = encode_value(r1, r2);
        mbuf[1] = encode_value(computed_normal, mbuf[0]);
        mbuf[2] = encode_value(r2, mbuf[1]);
        mbuf[3] = encode_value(computed_normal, mbuf[2]);
        mbuf[4] = encode_value(r1, mbuf[3]);
        mbuf[5] = encode_value(computed_normal, mbuf[4]);
        mbuf[6] = encode_value(r2, mbuf[5]);
        mbuf[7] = encode_value(mbuf[6], mbuf[6]);
        let mut mbytes = [0u8; 64];
        for (i, &v) in mbuf.iter().enumerate() { mbytes[i*8..(i+1)*8].copy_from_slice(&v.to_le_bytes()); }
        
        let computed_mirrored = dwg_ac21_mirrored_crc64(mirr_seed, 64, &mbytes);

        let norm_ok = computed_normal == normal_crc_stored;
        let mirr_ok = computed_mirrored == mirrored_crc_stored;
        if norm_ok || mirr_ok {
            println!("  [{}] normal={} ({:#018X}) mirrored={} ({:#018X})",
                label, if norm_ok {"OK"} else {"NO"}, computed_normal,
                if mirr_ok {"OK"} else {"NO"}, computed_mirrored);
        }
    }

    // Short summary using original implementation
    let normal_computed = compute_normal_crc(r1, r2);
    let mirrored_computed = compute_mirrored_crc(r1, r2, normal_computed);
    let norm_ok = normal_computed == normal_crc_stored;
    let mirr_ok = mirrored_computed == mirrored_crc_stored;
    println!("\n  Original impl: normal={} mirrored={}", if norm_ok {"OK"} else {"FAIL"}, if mirr_ok {"OK"} else {"FAIL"});
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: diag_checkdata_verify <file1.dwg> [file2.dwg ...]");
        std::process::exit(1);
    }
    for path in &args[1..] {
        verify_file(path);
        println!();
    }
}
