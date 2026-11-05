//! Xbox 360 XEX2 executables: recover the mapped base image (the bytes the
//! console loads at the image base) from a retail `default.xex`.
//! Layout follows the public XEX2 documentation used by Xenia and xextool.
//! Supports retail encryption and "normal" (LZX) or "basic" (raw) compression.
mod aes;
mod lzx;

/// The published Xbox 360 retail XEX key (it decrypts the per-file key).
const RETAIL_KEY: [u8; 16] = [
    0x20, 0xb1, 0x85, 0xa5, 0x9d, 0x28, 0xfd, 0xc3, 0x40, 0x58, 0x3f, 0xbb, 0x08, 0x96, 0xbf, 0x91,
];

const FILE_FORMAT_INFO: u32 = 0x0000_03ff;
const IMAGE_BASE_ADDRESS: u32 = 0x0001_0201;

#[derive(Debug)]
pub struct XexImage {
    /// Virtual address of `image[0]`.
    pub base_address: u32,
    pub image: Vec<u8>,
}

fn be16(d: &[u8], at: usize) -> Result<u16, String> {
    d.get(at..at + 2)
        .map(|b| u16::from_be_bytes([b[0], b[1]]))
        .ok_or_else(|| format!("Truncated XEX at {at}"))
}
fn be32(d: &[u8], at: usize) -> Result<u32, String> {
    d.get(at..at + 4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| format!("Truncated XEX at {at}"))
}
fn span(d: &[u8], at: usize, n: usize) -> Result<&[u8], String> {
    d.get(at..at.checked_add(n).ok_or("XEX offset overflow")?)
        .ok_or_else(|| format!("Truncated XEX at {at} (need {n})"))
}

impl XexImage {
    pub fn parse(xex: &[u8]) -> Result<Self, String> {
        if span(xex, 0, 4)? != b"XEX2" {
            return Err("Not an XEX2 executable".into());
        }
        let payload_offset = be32(xex, 8)? as usize;
        let security = be32(xex, 16)? as usize;
        let header_count = be32(xex, 20)? as usize;
        let mut format = None;
        let mut base_address = None;
        for i in 0..header_count {
            let key = be32(xex, 24 + i * 8)?;
            let value = be32(xex, 28 + i * 8)?;
            match key {
                FILE_FORMAT_INFO => format = Some(value as usize),
                IMAGE_BASE_ADDRESS => base_address = Some(value),
                _ => {}
            }
        }
        let format = format.ok_or("XEX has no file format header")?;
        let encryption = be16(xex, format + 4)?;
        let compression = be16(xex, format + 6)?;
        // Security info: image size at +4, the encrypted file key at +0x150,
        // the load address at +0x110.
        let image_size = be32(xex, security + 4)? as usize;
        let base_address = match base_address {
            Some(address) => address,
            None => be32(xex, security + 0x110)?,
        };
        let mut payload = span(
            xex,
            payload_offset,
            xex.len().saturating_sub(payload_offset),
        )?
        .to_vec();
        match encryption {
            0 => {}
            1 => {
                let mut key: [u8; 16] = span(xex, security + 0x150, 16)?.try_into().unwrap();
                aes::Aes128::new(RETAIL_KEY).decrypt_block(&mut key);
                aes::Aes128::new(key).decrypt_cbc_zero_iv(&mut payload);
            }
            other => return Err(format!("Unsupported XEX encryption type {other}")),
        }
        let image = match compression {
            1 => basic(xex, format, &payload, image_size)?,
            2 => normal(xex, format, &payload, image_size)?,
            other => return Err(format!("Unsupported XEX compression type {other}")),
        };
        if image.get(..2) != Some(b"MZ") {
            return Err("Unpacked XEX image does not start with a PE header (wrong key?)".into());
        }
        Ok(Self {
            base_address,
            image,
        })
    }

    /// Bytes at a virtual address.
    pub fn at(&self, address: u32, len: usize) -> Option<&[u8]> {
        let offset = address.checked_sub(self.base_address)? as usize;
        self.image.get(offset..offset.checked_add(len)?)
    }
}

/// Basic compression: (data size, zero size) pairs, zeros not stored.
fn basic(xex: &[u8], format: usize, payload: &[u8], image_size: usize) -> Result<Vec<u8>, String> {
    let info_size = be32(xex, format)? as usize;
    let blocks = info_size.saturating_sub(8) / 8;
    let mut image = Vec::with_capacity(image_size);
    let mut at = 0;
    for i in 0..blocks {
        let data = be32(xex, format + 8 + i * 8)? as usize;
        let zeros = be32(xex, format + 12 + i * 8)? as usize;
        image.extend_from_slice(span(payload, at, data)?);
        at += data;
        image.resize(image.len() + zeros, 0);
    }
    image.resize(image_size, 0);
    Ok(image)
}

/// Normal compression: a chain of blocks, each starting with the next
/// block's size and hash, then u16-length-prefixed LZX chunks.
fn normal(xex: &[u8], format: usize, payload: &[u8], image_size: usize) -> Result<Vec<u8>, String> {
    let window_size = be32(xex, format + 8)? as usize;
    let mut block_size = be32(xex, format + 12)? as usize;
    let mut at = 0;
    let mut stream = Vec::with_capacity(payload.len());
    while block_size != 0 {
        let block = span(payload, at, block_size)?;
        let next = be32(block, 0)? as usize;
        let mut p = 24;
        loop {
            let chunk = be16(block, p)? as usize;
            p += 2;
            if chunk == 0 {
                break;
            }
            stream.extend_from_slice(span(block, p, chunk)?);
            p += chunk;
        }
        at += block_size;
        block_size = next;
    }
    lzx::decompress(&stream, window_size, image_size)
}
