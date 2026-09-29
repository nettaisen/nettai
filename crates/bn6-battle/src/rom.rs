//! Read-only access to game data in the ROM (chip tables, stats, animation
//! timing). The engine's rules live in Rust; the content stays in the ROM.

use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    /// MEGAMAN6_FXXBR6E
    Falzar,
    /// MEGAMAN6_GXXBR5E
    Gregar,
}

#[derive(Clone)]
pub struct Rom {
    bytes: Arc<[u8]>,
    pub version: Version,
}

#[derive(Debug)]
pub struct UnsupportedRom(pub String);

impl std::fmt::Display for UnsupportedRom {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "unsupported ROM: {}", self.0)
    }
}

impl std::error::Error for UnsupportedRom {}

impl Rom {
    pub fn new(bytes: impl Into<Arc<[u8]>>) -> Result<Rom, UnsupportedRom> {
        let bytes: Arc<[u8]> = bytes.into();
        let code = bytes.get(0xA0..0xB0).map(|b| String::from_utf8_lossy(b).into_owned()).unwrap_or_default();
        let version = match code.as_str() {
            "MEGAMAN6_FXXBR6E" => Version::Falzar,
            "MEGAMAN6_GXXBR5E" => Version::Gregar,
            _ => return Err(UnsupportedRom(code)),
        };
        Ok(Rom { bytes, version })
    }

    fn offset(addr: u32) -> usize {
        (addr & 0x01FF_FFFF) as usize
    }

    pub fn u8(&self, addr: u32) -> u8 {
        self.bytes[Self::offset(addr)]
    }

    pub fn u16(&self, addr: u32) -> u16 {
        let o = Self::offset(addr);
        u16::from_le_bytes([self.bytes[o], self.bytes[o + 1]])
    }

    pub fn u32(&self, addr: u32) -> u32 {
        let o = Self::offset(addr);
        u32::from_le_bytes(self.bytes[o..o + 4].try_into().unwrap())
    }

    pub fn slice(&self, addr: u32, len: usize) -> &[u8] {
        let o = Self::offset(addr);
        &self.bytes[o..o + len]
    }
}
