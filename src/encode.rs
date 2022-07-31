use lazy_static::lazy_static;
use std::ffi::CString;

use crate::{error::EncodeError, get_header_len, verify::compute_crc, HQXConfig, HQXRef, HQX};

fn default_name() -> CString {
    CString::new("Untitled.hqx").unwrap()
}

pub fn binhex(config: HQXConfig) -> Result<HQX, EncodeError> {
    let mut hqx = Vec::with_capacity(config.hqx_len());

    // name_len, name
    let name = config.name.unwrap_or_else(default_name);
    let name_bytes = name.as_bytes();
    let name_len = match name_bytes.len().try_into() {
        Ok(len) => len,
        Err(_) => {
            return Err(EncodeError::FileNameTooLong);
        }
    };
    hqx.push(name_len);
    name_bytes.iter().for_each(|b| hqx.push(*b));
    hqx.push(0); // null-terminate name

    // file_type
    let file_type = config.file_type.copied().unwrap_or_default();
    file_type.into_iter().for_each(|b| hqx.push(b));

    // author
    let author = config.author.copied().unwrap_or_default();
    author.into_iter().for_each(|b| hqx.push(b));

    // flags
    let flags = config.flags.copied().unwrap_or_default();
    flags.into_iter().for_each(|b| hqx.push(b));

    // data_len
    let data = config.data.unwrap_or_default();
    let data_len: u32 = match data.len().try_into() {
        Ok(len) => len,
        Err(_) => {
            return Err(EncodeError::DataTooLarge);
        }
    };
    data_len.to_be_bytes().into_iter().for_each(|b| hqx.push(b));

    // resource_len
    let resource = config.resource.unwrap_or_default();
    let resource_len: u32 = match resource.len().try_into() {
        Ok(len) => len,
        Err(_) => {
            return Err(EncodeError::ResourceTooLarge);
        }
    };
    resource_len
        .to_be_bytes()
        .into_iter()
        .for_each(|b| hqx.push(b));

    // add two zero bytes in place of header crc to compute actual header crc
    hqx.push(0);
    hqx.push(0);

    let header_len = get_header_len(name_len.into());

    // header crc
    let hc = compute_crc(&hqx[..header_len]).to_be_bytes();
    hqx.pop();
    hqx.pop();
    hqx.push(hc[0]);
    hqx.push(hc[1]);

    // data
    if data_len > 0 {
        let data_start = hqx.len();

        data.iter().for_each(|b| hqx.push(*b));

        // crc placeholder
        hqx.push(0);
        hqx.push(0);

        // data crc
        let dc = compute_crc(&hqx[data_start..data_start + data_len as usize + 2]);
        hqx.pop();
        hqx.pop();
        dc.to_be_bytes().into_iter().for_each(|b| hqx.push(b));
    } else {
        hqx.push(0);
        hqx.push(0);
    }

    // resource
    if resource_len > 0 {
        let resource_start = hqx.len();

        resource.iter().for_each(|b| hqx.push(*b));

        // crc placeholder
        hqx.push(0);
        hqx.push(0);

        // resource crc
        let rc = compute_crc(&hqx[resource_start..resource_start + resource_len as usize + 2]);
        hqx.pop();
        hqx.pop();
        rc.to_be_bytes().into_iter().for_each(|b| hqx.push(b));
    } else {
        hqx.push(0);
        hqx.push(0);
    }

    Ok(HQX { vec: hqx })
}

fn len_to_encoded_len(len: usize) -> usize {
    (len / 3) * 4
}

const BINHEX_FILE_MARKER: &[u8] = "(This file must be converted with BinHex 4.0)\n\n".as_bytes();

impl<'a> HQXRef<'a> {
    pub fn encode(&self) -> Vec<u8> {
        let data = self
            .data_fork
            .as_ref()
            .map(|fork| fork.data)
            .unwrap_or_default();
        let resource = self
            .data_fork
            .as_ref()
            .map(|fork| fork.data)
            .unwrap_or_default();

        let encoded_len = len_to_encoded_len(
            self.header_len as usize // header
                + data.len() // data
                + 2 // data crc
                + resource.len() // resource
                + 2, // resource crc
        );

        let bytes = &self.hqx.vec[..];

        let newline_count = (bytes.len() / 64) + 1;

        let mut encoded = Vec::with_capacity(
            BINHEX_FILE_MARKER.len() // file marker
            + 1 // colon
            + encoded_len // data len
            + newline_count * 2 // number of newlines (*2 for \r\n)
            + 1 // colon
            + 2, // \r\n
        );

        BINHEX_FILE_MARKER.iter().for_each(|b| encoded.push(*b));
        encoded.push(b':');

        let mut encode_state: u8 = 0;
        let mut save_bits: u32 = 0;
        let mut next_byte: u8;

        let mut bytes_in_line = 0;
        let encode_byte = |b: u8| BYTE_ENCODINGS[b as usize];
        let mut push_byte = |b: u8| {
            encoded.push(encode_byte(b));
            bytes_in_line += 1;
            if bytes_in_line == 64 {
                encoded.push(b'\r');
                encoded.push(b'\n');
                bytes_in_line = 0;
            }
        };

        for byte in bytes.iter().copied() {
            match encode_state {
                0 => {
                    next_byte = (byte >> 2) & 0x3F;
                    push_byte(next_byte);
                    save_bits = (byte & 0x03) as u32;
                }
                1 => {
                    next_byte = (((save_bits << 4) & 0x30) as u8) | (byte >> 4);
                    push_byte(next_byte);
                    save_bits = (byte & 0x0F) as u32;
                }
                2 => {
                    next_byte = (((save_bits << 2) & 0x3C) as u8) | (byte >> 6);
                    push_byte(next_byte);
                    next_byte = byte & 0x3F;
                    push_byte(next_byte);
                }
                _ => unreachable!(),
            }
            encode_state = (encode_state + 1) % 3;
        }

        encoded.push(b':');
        encoded.push(b'\r');
        encoded.push(b'\n');

        encoded
    }
}

lazy_static! {
    static ref BYTE_ENCODINGS: [u8; 64] = create_byte_encodings();
}

fn create_byte_encodings() -> [u8; 64] {
    "!\"#$%&'()*+,-012345689@ABCDEFGHIJKLMNPQRSTUVXYZ[`abcdefhijklmpqr"
        .as_bytes()
        .try_into()
        .unwrap()
}
