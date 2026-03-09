//! Convert ringo.dxf (AC1021) to AC1027.

use acadrust::{DxfReader, DxfWriter, DxfVersion};

fn main() -> acadrust::Result<()> {
    let input = "examples/sat v7 samples/ringo.dxf";
    let output = "examples/sat v7 samples/ringo_AC1027.dxf";

    println!("Reading {}...", input);
    let reader = DxfReader::from_file(input)?;
    let mut doc = reader.read()?;

    println!("  Version: {:?}", doc.version);
    doc.version = DxfVersion::AC1027;
    println!("  Set version to AC1027");

    let writer = DxfWriter::new(doc);
    writer.write_to_file(output)?;
    println!("  Written: {}", output);

    Ok(())
}
