use acadrust::io::dwg::{DwgReader, DwgWriter};
fn main() -> acadrust::Result<()> {
    let input = "\u{00C7}izim1.dwg";
    let mut reader = DwgReader::from_file(input)?;
    let doc = reader.read()?;
    let ms1 = doc.block_records.get("*Model_Space").map(|b| b.entities.len()).unwrap_or(0);
    let ps1 = doc.block_records.get("*Paper_Space").map(|b| b.entities.len()).unwrap_or(0);
    println!("Original: MS={ms1} PS={ps1}");

    DwgWriter::write_to_file("target/space_test.dwg", &doc)?;
    let mut r2 = DwgReader::from_file("target/space_test.dwg")?;
    let doc2 = r2.read()?;
    let ms2 = doc2.block_records.get("*Model_Space").map(|b| b.entities.len()).unwrap_or(0);
    let ps2 = doc2.block_records.get("*Paper_Space").map(|b| b.entities.len()).unwrap_or(0);
    println!("Roundtrip: MS={ms2} PS={ps2}");

    if ms1 == ms2 && ps1 == ps2 {
        println!("PASS - space assignment preserved");
    } else {
        println!("FAIL - space assignment changed!");
    }
    Ok(())
}
