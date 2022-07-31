use std::{fs::File, io::Write, path::PathBuf, str::FromStr};

use binhex4::decode::hexbin;
use nom::HexDisplay;

fn main() -> std::io::Result<()> {
    let file_data = include_bytes!(concat!("../test/hex/earth.hqx"));
    let result = hexbin(file_data, true);
    assert!(result.is_ok());

    let result = result.unwrap();
    let hex_str = result.vec.to_hex(16);

    let mut file = File::create(concat!("./test/raw/earth.out"))?;
    file.write_all(hex_str.as_bytes())?;

    let hqx_ref = result.borrow();
    hqx_ref.decode_to_file(PathBuf::from_str("./test/hex/earth.gif").unwrap())?;

    Ok(())
}
