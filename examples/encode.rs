use std::{ffi::CString, fs::File, io::Write, path::PathBuf, str::FromStr};

use binhex4::{encode::binhex, HQXConfig};
use nom::HexDisplay;

fn main() -> std::io::Result<()> {
    let file_data = include_bytes!(concat!("../test/bin/orange.txt"));
    let result = binhex(HQXConfig {
        name: Some(CString::new("orange.txt").unwrap()),
        file_type: None,
        author: None,
        flags: None,
        data: Some(file_data),
        resource: None,
    });
    assert!(result.is_ok());

    let result = result.unwrap();
    let hex_str = result.vec.to_hex(16);

    let mut file = File::create(concat!("./test/raw/orange.out"))?;
    file.write_all(hex_str.as_bytes())?;

    let hqx_ref = result.borrow();
    hqx_ref.encode_to_file(PathBuf::from_str("./test/bin/orange.hqx").unwrap())?;

    Ok(())
}
