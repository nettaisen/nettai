//! Byte codecs for what the handshake carries between peers
//! (`transport::Hello`): its versions, the game, the content's hash, the
//! nonce. (What a player brings to the match is the frontend's to encode: a
//! match file's side, as text.)
//!
//! Integers above a byte are LEB128 (rennet's varints).

use std::io;

use nettai_battle::ContentHash;

use crate::protocol::invalid;

/// Writes values to a byte buffer.
pub struct Writer<'a>(pub &'a mut Vec<u8>);

impl Writer<'_> {
    pub fn byte(&mut self, b: u8) {
        self.0.push(b);
    }

    pub fn uvarint(&mut self, v: u64) {
        rennet::write_uvarint(self.0, v).expect("writing to a Vec");
    }

    pub fn bytes(&mut self, b: &[u8]) {
        self.uvarint(b.len() as u64);
        self.0.extend_from_slice(b);
    }

    pub fn put<T: Wire>(&mut self, v: &T) {
        v.write(self);
    }
}

/// Reads values back from bytes.
pub struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Reader<'a> {
        Reader { rest: bytes }
    }

    pub fn byte(&mut self) -> io::Result<u8> {
        let (&b, rest) = self.rest.split_first().ok_or_else(|| io::Error::from(io::ErrorKind::UnexpectedEof))?;
        self.rest = rest;
        Ok(b)
    }

    pub fn uvarint(&mut self) -> io::Result<u64> {
        rennet::read_uvarint(&mut self.rest)
    }

    pub fn bytes(&mut self) -> io::Result<&'a [u8]> {
        let n = self.uvarint()?;
        if n > self.rest.len() as u64 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        let (b, rest) = self.rest.split_at(n as usize);
        self.rest = rest;
        Ok(b)
    }

    pub fn get<T: Wire>(&mut self) -> io::Result<T> {
        T::read(self)
    }

    /// The end: every byte was read.
    pub fn finish(self) -> io::Result<()> {
        if self.rest.is_empty() { Ok(()) } else { Err(invalid("bytes left over")) }
    }

    pub fn is_empty(&self) -> bool {
        self.rest.is_empty()
    }
}

/// A value with a byte form.
pub trait Wire: Sized {
    fn write(&self, w: &mut Writer);
    fn read(r: &mut Reader) -> io::Result<Self>;
}

/// The value encoded alone.
pub fn to_bytes<T: Wire>(v: &T) -> Vec<u8> {
    let mut out = Vec::new();
    v.write(&mut Writer(&mut out));
    out
}

/// The value decoded from all of `bytes`.
pub fn from_bytes<T: Wire>(bytes: &[u8]) -> io::Result<T> {
    let mut r = Reader::new(bytes);
    let v = T::read(&mut r)?;
    r.finish()?;
    Ok(v)
}

impl Wire for u8 {
    fn write(&self, w: &mut Writer) {
        w.byte(*self);
    }
    fn read(r: &mut Reader) -> io::Result<u8> {
        r.byte()
    }
}

impl Wire for bool {
    fn write(&self, w: &mut Writer) {
        w.byte(*self as u8);
    }
    fn read(r: &mut Reader) -> io::Result<bool> {
        match r.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            b => Err(invalid(&format!("bad bool {b}"))),
        }
    }
}

macro_rules! wire_uint {
    ($($t:ty),*) => {$(
        impl Wire for $t {
            fn write(&self, w: &mut Writer) {
                w.uvarint(*self as u64);
            }
            fn read(r: &mut Reader) -> io::Result<$t> {
                <$t>::try_from(r.uvarint()?).map_err(|_| invalid(concat!(stringify!($t), " out of range")))
            }
        }
    )*};
}

wire_uint!(u16, u32, u64);

impl Wire for String {
    fn write(&self, w: &mut Writer) {
        w.bytes(self.as_bytes());
    }
    fn read(r: &mut Reader) -> io::Result<String> {
        String::from_utf8(r.bytes()?.to_vec()).map_err(|_| invalid("a string that isn't UTF-8"))
    }
}

impl Wire for ContentHash {
    fn write(&self, w: &mut Writer) {
        w.0.extend_from_slice(&self.0.to_le_bytes());
    }
    fn read(r: &mut Reader) -> io::Result<ContentHash> {
        let mut b = [0u8; 8];
        for x in &mut b {
            *x = r.byte()?;
        }
        Ok(ContentHash(u64::from_le_bytes(b)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip<T: Wire + PartialEq + std::fmt::Debug>(v: &T) -> usize {
        let b = to_bytes(v);
        assert_eq!(&from_bytes::<T>(&b).unwrap(), v);
        // Cut short, it is an error.
        if !b.is_empty() {
            assert!(from_bytes::<T>(&b[..b.len() - 1]).is_err());
        }
        b.len()
    }

    #[test]
    fn a_hellos_parts_roundtrip() {
        assert_eq!(roundtrip(&7u16), 1);
        assert_eq!(roundtrip(&300u16), 2);
        roundtrip(&0xDEAD_BEEFu32);
        roundtrip(&true);
        assert_eq!(roundtrip(&ContentHash(0x0123_4567_89AB_CDEF)), 8);
        roundtrip(&"nettai".to_string());
        assert!(from_bytes::<bool>(&[2]).is_err());
        assert!(from_bytes::<u16>(&[0x80, 0x80, 0x04]).is_err(), "out of range");
        assert!(from_bytes::<u8>(&[1, 2]).is_err(), "bytes left over");
    }
}
