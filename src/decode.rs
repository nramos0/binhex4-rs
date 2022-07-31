use crate::error::DecodeError;
use crate::verify::verify;
use crate::{parse::parse, HQX};
use lazy_static::lazy_static;

pub fn hexbin(i: &[u8], should_verify: bool) -> Result<HQX, DecodeError> {
    match parse(i) {
        Ok((_, encoded_bin_with_newlines)) => {
            let hqx = decode(encoded_bin_with_newlines);
            if should_verify {
                verify(&hqx)?;
            }
            Ok(hqx)
        }
        Err(_) => Err(DecodeError::BadFormat),
    }
}

lazy_static! {
    static ref BYTE_DECODINGS: [u8; 82] = create_byte_decodings();
}

fn create_byte_decodings() -> [u8; 82] {
    let mut byte_decodings = [0; 82];

    let mut next = 0usize;

    macro_rules! fill_byte_encoding_range {
        ($items:expr) => {
            let items = $items;
            for (i, byte) in items.iter().enumerate() {
                byte_decodings[(*byte - b'!') as usize] = (i + next) as u8;
            }
            next += items.len();
        };
    }

    // [0-12] !"#$%&'()*+,-
    fill_byte_encoding_range!([
        b'!', b'"', b'#', b'$', b'%', b'&', b'\'', b'(', b')', b'*', b'+', b',', b'-'
    ]);

    // [13-14]
    // [15-21] 0123456
    fill_byte_encoding_range!([b'0', b'1', b'2', b'3', b'4', b'5', b'6']);

    // [22]
    // [23-24] 89
    fill_byte_encoding_range!([b'8', b'9']);

    // [25-30]
    // [31-45] @ABCDEFGHIJKLMN
    fill_byte_encoding_range!([
        b'@', b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N'
    ]);

    // [46]
    // [47-53] PQRSTUV
    fill_byte_encoding_range!([b'P', b'Q', b'R', b'S', b'T', b'U', b'V']);

    // [54]
    // [55-58] XYZ[
    fill_byte_encoding_range!([b'X', b'Y', b'Z', b'[']);

    // [59-62]
    // [63-69]`abcdef
    fill_byte_encoding_range!([b'`', b'a', b'b', b'c', b'd', b'e', b'f']);

    // [70]
    // [71-76] hijklm
    fill_byte_encoding_range!([b'h', b'i', b'j', b'k', b'l', b'm']);

    // [77-78]
    // [79-81] pqr
    fill_byte_encoding_range!([b'p', b'q', b'r']);

    debug_assert_eq!(next, 64);

    byte_decodings
}

const RLE_MARKER_BYTE: u8 = 0x90;
const EOF: u8 = 0xFF;

fn decode(encoded_bin_with_newlines: &[u8]) -> HQX {
    let mut out = Vec::<u8>::with_capacity(encoded_bin_with_newlines.len());

    let mut has_rle = false;
    let mut decode_state: u8 = 3;
    let mut partial_b8: u8 = 0;
    let mut last_byte: u8 = 0;

    let decode = |val: u8| BYTE_DECODINGS[(val - b'!') as usize] & 0x3F;

    for b6 in encoded_bin_with_newlines.iter() {
        let b6 = *b6;
        if b6 == b'\r' || b6 == b'\n' || b6 == EOF {
            continue;
        }

        decode_state = ((decode_state as u8) + 1) % 4;
        let b6_decoded = decode(b6);

        let mut data: u8;
        match decode_state {
            0 => {
                // cannot yet output a data byte
                partial_b8 = b6_decoded << 2;
                continue;
            }
            1 => {
                data = partial_b8 | (b6_decoded >> 4);
                partial_b8 = (b6_decoded & 0x0F) << 4;
            }
            2 => {
                data = partial_b8 | (b6_decoded >> 2);
                partial_b8 = (b6_decoded & 0x03) << 6;
            }
            3 => {
                data = partial_b8 | b6_decoded;
            }
            _ => unreachable!(),
        }

        if !has_rle {
            if data == RLE_MARKER_BYTE {
                has_rle = true;
            } else {
                last_byte = data;
                out.push(data);
            }
        } else {
            if data == 0x00 {
                last_byte = RLE_MARKER_BYTE;
                out.push(RLE_MARKER_BYTE);
            } else {
                loop {
                    data -= 1;
                    if data == 0 {
                        break;
                    }
                    out.push(last_byte);
                }
            }
            has_rle = false;
        }
    }

    HQX::new(out)
}
