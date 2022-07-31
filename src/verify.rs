use crate::{
    error::{CRCVerificationError, DecodeError},
    HQX,
};

const CRC_POLY: u16 = 0x1021;

pub fn verify(hqx: &HQX) -> Result<(), DecodeError> {
    let hqx_ref = hqx.borrow();

    verify_crc(
        // remove crc from header
        &hqx.vec[..hqx_ref.header_len as usize - 2],
        hqx_ref.hc,
        CRCVerificationError::Header,
    )?;

    if let Some(data_fork) = hqx_ref.data_fork {
        verify_crc(data_fork.data, data_fork.crc, CRCVerificationError::Data)?;
    }
    if let Some(data_fork) = hqx_ref.resource_fork {
        verify_crc(
            data_fork.data,
            data_fork.crc,
            CRCVerificationError::Resource,
        )?;
    }

    Ok(())
}

fn verify_crc(
    bytes: &[u8],
    expected_crc: u16,
    err: CRCVerificationError,
) -> Result<(), DecodeError> {
    let mut crc = 0;
    for byte in bytes {
        crc = calc_crc(*byte, crc);
    }
    crc = calc_crc(0x00, crc);
    crc = calc_crc(0x00, crc);

    if crc != expected_crc {
        Err(DecodeError::CRCVerificationError(err))
    } else {
        Ok(())
    }
}

pub fn compute_crc(data: &[u8]) -> u16 {
    let mut crc = 0;

    for byte in data {
        crc = calc_crc(*byte, crc);
    }

    crc
}

fn calc_crc(mut byte: u8, mut crc: u16) -> u16 {
    for _ in 0..8 {
        // is 0xFFFF if crc has most significant bit == 1
        // is 0x0000 otherwise
        // converts to i16 to have msb carried downward
        let cond = (((crc & 0x8000) as i16) >> 15) as u16;
        crc = (crc << 1) | ((byte >> 7) as u16);
        crc ^= cond & CRC_POLY;
        byte <<= 1;
    }
    crc
}
