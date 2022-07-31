pub mod decode;
pub mod encode;
mod error;
mod parse;
mod verify;

use encode::binhex;
use error::EncodeError;

use std::io::{ErrorKind, Write};
use std::{
    ffi::{CStr, CString},
    fs::File,
    path::Path,
};

#[derive(Debug)]
pub struct HQX {
    pub vec: Vec<u8>,
}

impl HQX {
    pub fn new(vec: Vec<u8>) -> HQX {
        HQX { vec }
    }

    pub fn borrow(&self) -> HQXRef {
        let mut bytes = &self.vec[..];

        let name_len = &bytes[0];
        bytes = &bytes[1..];

        let name_len_usize = *name_len as usize;

        // add one to include null terminator
        let name = CStr::from_bytes_with_nul(&bytes[..name_len_usize + 1]).unwrap();
        bytes = &bytes[name_len_usize + 1..];

        let file_type = <&[u8; 4]>::try_from(&bytes[..4]).unwrap();
        bytes = &bytes[4..];

        let author = <&[u8; 4]>::try_from(&bytes[..4]).unwrap();
        bytes = &bytes[4..];

        let flags = <&[u8; 2]>::try_from(&bytes[..2]).unwrap();
        bytes = &bytes[2..];

        let data_len = u32::from_be_bytes(<[u8; 4]>::try_from(&bytes[..4]).unwrap());
        bytes = &bytes[4..];

        let resource_len = u32::from_be_bytes(<[u8; 4]>::try_from(&bytes[..4]).unwrap());
        bytes = &bytes[4..];

        let hc_bytes = <[u8; 2]>::try_from(&bytes[..2]).unwrap_or_default();
        bytes = &bytes[2..];

        let hc = u16::from_be_bytes(hc_bytes);

        let header_len = get_header_len(name_len_usize) as u16;

        let data = {
            if data_len > 0 {
                let data = &bytes[..data_len as usize];
                bytes = &bytes[data_len as usize..];

                Some(data)
            } else {
                None
            }
        };

        let dc_bytes = {
            if bytes.len() >= 2 {
                let dc_bytes = <[u8; 2]>::try_from(&bytes[..2]).unwrap_or_default();
                bytes = &bytes[2..];
                dc_bytes
            } else {
                [0, 0]
            }
        };

        let dc = u16::from_be_bytes(dc_bytes);
        let data_fork = data.map(|data| Fork { data, crc: dc });

        let resource = {
            if resource_len > 0 {
                let resource = &bytes[..resource_len as usize];
                bytes = &bytes[resource_len as usize..];

                Some(resource)
            } else {
                None
            }
        };

        let rc_bytes = {
            if bytes.len() >= 2 {
                let rc_bytes = <[u8; 2]>::try_from(&bytes[..2]).unwrap_or_default();
                bytes = &bytes[2..];
                rc_bytes
            } else {
                [0, 0]
            }
        };

        let rc = u16::from_be_bytes(rc_bytes);
        let resource_fork = resource.map(|resource| Fork {
            data: resource,
            crc: rc,
        });

        HQXRef {
            hqx: self,
            name_len,
            name,
            file_type,
            author,
            flags,
            data_len,
            resource_len,
            hc,
            header_len,
            data_fork,
            resource_fork,
        }
    }

    pub fn from_config(config: HQXConfig) -> Result<HQX, EncodeError> {
        binhex(config)
    }
}

pub struct HQXConfig<'a> {
    pub name: Option<CString>,
    pub file_type: Option<&'a [u8; 4]>,
    pub author: Option<&'a [u8; 4]>,
    pub flags: Option<&'a [u8; 2]>,
    pub data: Option<&'a [u8]>,
    pub resource: Option<&'a [u8]>,
}

impl<'a> HQXConfig<'a> {
    pub fn hqx_len(&self) -> usize {
        get_header_len(self.name.as_ref().map_or(0, |name| name.to_bytes().len()))
            + self.data.unwrap_or_default().len()
            + self.resource.unwrap_or_default().len()
    }
}

pub struct HQXRef<'a> {
    pub hqx: &'a HQX,
    pub name_len: &'a u8,
    pub name: &'a CStr,
    pub file_type: &'a [u8; 4],
    pub author: &'a [u8; 4],
    pub flags: &'a [u8; 2],
    pub data_len: u32,
    pub resource_len: u32,
    pub hc: u16,
    pub header_len: u16,
    pub data_fork: Option<Fork<'a>>,
    pub resource_fork: Option<Fork<'a>>,
}

#[derive(Debug, Default)]
pub struct Fork<'a> {
    pub data: &'a [u8],
    pub crc: u16,
}

impl<'a> HQXRef<'a> {
    pub fn decode_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<File> {
        let path = {
            let mut path = path.as_ref().to_path_buf();
            if path.is_dir() {
                match self.name.to_str() {
                    Ok(name) => {
                        path.push(name);
                    }
                    Err(err) => {
                        eprintln!("{:#?}", err);
                        return Err(std::io::Error::new(ErrorKind::Other, "Non UTF-8 file name"));
                    }
                }
            }
            path
        };
        let mut file = File::create(path)?;

        if let Some(Fork { data, .. }) = self.data_fork.as_ref() {
            file.write_all(data)?;
            Ok(file)
        } else {
            Err(ErrorKind::InvalidData.into())
        }
    }

    pub fn encode_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<File> {
        let path = {
            let mut path = path.as_ref().to_path_buf();
            if path.is_dir() {
                match self.name.to_str() {
                    Ok(name) => {
                        path.push(name);
                    }
                    Err(err) => {
                        eprintln!("{:#?}", err);
                        return Err(std::io::Error::new(ErrorKind::Other, "Non UTF-8 file name"));
                    }
                }
            }
            path
        };
        let mut file = File::create(path)?;

        let encoded = self.encode();

        file.write_all(&encoded)?;

        Ok(file)
    }
}

fn get_header_len(name_len: usize) -> usize {
    1 + (name_len + 1) + 4 + 4 + 2 + 4 + 4 + 2
}
