use acadrust::DwgReader;

fn dump(path: &str) {
    let mut reader = match DwgReader::from_file(path) {
        Ok(reader) => reader,
        Err(err) => {
            eprintln!("open failed for {}: {:?}", path, err);
            return;
        }
    };

    let info = match reader.read_file_header() {
        Ok(info) => info,
        Err(err) => {
            eprintln!("header read failed for {}: {:?}", path, err);
            return;
        }
    };

    println!("=== {} ===", path);
    if let Some(meta) = &info.ac21_metadata {
        println!("file_size={}", meta.file_size);
        println!("pages_amount={} page_records={} pages_max_id={}", meta.pages_amount, info.page_records.len(), meta.pages_max_id);
        println!("sections_amount={} section_descriptors={}", meta.sections_amount, info.section_descriptors.len());
        println!("pages_map_offset={:#X} map2_offset={:#X} header2_offset={:#X}", meta.pages_map_offset, meta.map2_offset, meta.header2_offset);
        println!("pages_map_comp={} pages_map_uncomp={} correction_factor={}", meta.pages_map_size_compressed, meta.pages_map_size_uncompressed, meta.pages_map_correction_factor);
        println!("sections_map_comp={} sections_map_uncomp={} correction_factor={}", meta.sections_map_size_compressed, meta.sections_map_size_uncompressed, meta.sections_map_correction_factor);
        println!("pages_map_crc_comp={:#018X} pages_map_crc_uncomp={:#018X}", meta.pages_map_crc_compressed, meta.pages_map_crc_uncompressed);
        println!("sections_map_crc_comp={:#018X} sections_map_crc_uncomp={:#018X}", meta.sections_map_crc_compressed, meta.sections_map_crc_uncompressed);
        println!("header_crc64={:#018X} crc_seed={:#018X}", meta.header_crc64, meta.crc_seed);

        let mut ordered_pages: Vec<_> = info.page_records.iter().collect();
        ordered_pages.sort_by_key(|(_, (offset, _))| *offset);
        println!("first_pages:");
        for (id, (offset, size)) in ordered_pages.iter().take(8) {
            println!("  id={} offset={} size={}", id, offset, size);
        }
        println!("last_pages:");
        for (id, (offset, size)) in ordered_pages.iter().rev().take(8).rev() {
            println!("  id={} offset={} size={}", id, offset, size);
        }

        println!("sections:");
        for section in &info.section_descriptors {
            let total_page_file_size: u64 = section
                .pages
                .iter()
                .filter_map(|page| info.page_records.get(&(page.page_number as i32)).map(|(_, size)| *size as u64))
                .sum();
            println!(
                "  {} encoding={} pages={} data={} max_page={} page_file_total={}",
                section.name,
                section.encoding,
                section.pages.len(),
                section.compressed_size,
                section.decompressed_size,
                total_page_file_size
            );
        }
    } else {
        println!("not AC21");
    }
    println!();
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: cargo run --example diag_ac21_compare -- <file1.dwg> [file2.dwg ...]");
        std::process::exit(2);
    }

    for path in paths {
        dump(&path);
    }
}