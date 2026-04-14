use acadrust::io::dwg::DwgReader;
fn main() {
    let mut reader = DwgReader::from_file("tests/roundtrip/samplekitchen.dwg").unwrap();
    let doc = reader.read().unwrap();
    for br in doc.block_records.iter() {
        let have = br.entity_handles.len();
        let desc_len = br.description.len();
        let ic = br.insert_count_bytes.len();
        let pd = br.preview_data.len();
        if br.flags.is_xref || desc_len > 0 || pd > 0 || ic > 0 {
            println!("Block '{}' handle={:?}: entities={} xref={} desc_len={} insert_count_bytes={} preview_data={}",
                br.name, br.handle, have, br.flags.is_xref, desc_len, ic, pd);
        }
    }
}
