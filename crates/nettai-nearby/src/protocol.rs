//! What two devices say over their TCP connection: the asker's hello, then the offerer's reply, which, where its
//! player chose to send, carries the files. Every number big-endian, every string UTF-8 after its length.
//!
//! ```text
//! hello  (asker -> offerer):  MAGIC, version u16, name (u16 length, bytes)
//! reply  (offerer -> asker):  MAGIC, version u16, status u8
//!   status 0, accepted:        count u16, total u64, then each file: name (u16 length, bytes), size u64, bytes
//!   status 1, declined         (its player chose not to)
//!   status 2, busy             (it is sending already, or asking its player of another)
//!   status 3, another version  (the hello's version isn't the offerer's, which the reply's header says)
//! ```
//!
//! Nothing is encrypted: the devices are on one local network, and what goes is the player's own ROMs, which the
//! receiver checks (each its game's header, by the app) and whose zips carry each entry's CRC-32, checked as it is
//! read. Limits keep a stray connection from asking for a lot of memory: a name at most [`MOST_NAME`] bytes, at most
//! [`MOST_FILES`] files, [`MOST_TOTAL`] bytes in all.

use crate::File;
use std::io::{self, Read, Write};

/// The first bytes of every hello and reply.
pub const MAGIC: [u8; 8] = *b"NETTAINB";
/// This protocol's version: a device of another refuses (status 3) rather than misread.
pub const VERSION: u16 = 1;
pub const MOST_NAME: usize = 255;
pub const MOST_FILES: usize = 64;
pub const MOST_TOTAL: u64 = 512 << 20;

/// An asker's hello.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hello {
    pub version: u16,
    pub name: String,
}

/// An offerer's reply, but for the files an accepted one goes on with ([`read_files`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reply {
    Accepted { count: usize, total: u64 },
    Declined,
    Busy,
    OtherVersion(u16),
}

fn bad(why: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why.into())
}

fn write_str(out: &mut impl Write, s: &str) -> io::Result<()> {
    let bytes = &s.as_bytes()[..s.len().min(MOST_NAME)];
    out.write_all(&(bytes.len() as u16).to_be_bytes())?;
    out.write_all(bytes)
}

fn read_u16(input: &mut impl Read) -> io::Result<u16> {
    let mut b = [0; 2];
    input.read_exact(&mut b)?;
    Ok(u16::from_be_bytes(b))
}

fn read_u64(input: &mut impl Read) -> io::Result<u64> {
    let mut b = [0; 8];
    input.read_exact(&mut b)?;
    Ok(u64::from_be_bytes(b))
}

fn read_str(input: &mut impl Read) -> io::Result<String> {
    let len = read_u16(input)? as usize;
    if len > MOST_NAME {
        return Err(bad(format!("a name of {len} bytes")));
    }
    let mut bytes = vec![0; len];
    input.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(|_| bad("a name that isn't UTF-8"))
}

/// The header both say first: MAGIC and the version.
fn read_header(input: &mut impl Read) -> io::Result<u16> {
    let mut magic = [0; 8];
    input.read_exact(&mut magic)?;
    if magic != MAGIC {
        return Err(bad("not nettai's"));
    }
    read_u16(input)
}

pub fn write_hello(out: &mut impl Write, name: &str) -> io::Result<()> {
    out.write_all(&MAGIC)?;
    out.write_all(&VERSION.to_be_bytes())?;
    write_str(out, name)?;
    out.flush()
}

pub fn read_hello(input: &mut impl Read) -> io::Result<Hello> {
    let version = read_header(input)?;
    Ok(Hello { version, name: read_str(input)? })
}

/// A reply that carries no files: declined, busy, or another version.
pub fn write_refusal(out: &mut impl Write, reply: Reply) -> io::Result<()> {
    let status = match reply {
        Reply::Declined => 1,
        Reply::Busy => 2,
        Reply::OtherVersion(_) => 3,
        Reply::Accepted { .. } => return Err(bad("an acceptance carries its files: write_files")),
    };
    out.write_all(&MAGIC)?;
    out.write_all(&VERSION.to_be_bytes())?;
    out.write_all(&[status])?;
    out.flush()
}

/// An acceptance, and the files after it; `progress` hears the bytes written of the files', of their total.
pub fn write_files(out: &mut impl Write, files: &[File], progress: &mut dyn FnMut(u64, u64)) -> io::Result<()> {
    let total: u64 = files.iter().map(|f| f.bytes.len() as u64).sum();
    if files.len() > MOST_FILES || total > MOST_TOTAL {
        return Err(bad(format!("{} files of {total} bytes: more than a transfer holds", files.len())));
    }
    out.write_all(&MAGIC)?;
    out.write_all(&VERSION.to_be_bytes())?;
    out.write_all(&[0])?;
    out.write_all(&(files.len() as u16).to_be_bytes())?;
    out.write_all(&total.to_be_bytes())?;
    let mut done = 0;
    for file in files {
        write_str(out, &file.name)?;
        out.write_all(&(file.bytes.len() as u64).to_be_bytes())?;
        for chunk in file.bytes.chunks(64 << 10) {
            out.write_all(chunk)?;
            done += chunk.len() as u64;
            progress(done, total);
        }
    }
    out.flush()
}

pub fn read_reply(input: &mut impl Read) -> io::Result<Reply> {
    let version = read_header(input)?;
    let mut status = [0];
    input.read_exact(&mut status)?;
    Ok(match status[0] {
        0 if version == VERSION => {
            let count = read_u16(input)? as usize;
            let total = read_u64(input)?;
            if count > MOST_FILES || total > MOST_TOTAL {
                return Err(bad(format!("{count} files of {total} bytes: more than a transfer holds")));
            }
            Reply::Accepted { count, total }
        }
        0 => return Err(bad(format!("an acceptance of version {version}"))),
        1 => Reply::Declined,
        2 => Reply::Busy,
        3 => Reply::OtherVersion(version),
        other => return Err(bad(format!("status {other}"))),
    })
}

/// An accepted reply's files ([`Reply::Accepted`]); `progress` hears the bytes read, of the total said.
pub fn read_files(input: &mut impl Read, count: usize, total: u64, progress: &mut dyn FnMut(u64, u64)) -> io::Result<Vec<File>> {
    let mut files = Vec::with_capacity(count);
    let mut done = 0u64;
    for _ in 0..count {
        let name = read_str(input)?;
        if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') {
            return Err(bad(format!("a file named {name:?}")));
        }
        let size = read_u64(input)?;
        if done + size > total {
            return Err(bad("more bytes than the total said"));
        }
        let mut bytes = Vec::with_capacity(size as usize);
        let mut left = size;
        let mut chunk = vec![0; 64 << 10];
        while left > 0 {
            let n = (left as usize).min(chunk.len());
            input.read_exact(&mut chunk[..n])?;
            bytes.extend_from_slice(&chunk[..n]);
            left -= n as u64;
            done += n as u64;
            progress(done, total);
        }
        files.push(File { name, bytes });
    }
    if done != total {
        return Err(bad("fewer bytes than the total said"));
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> Vec<File> {
        vec![File { name: "BR6E.zip".into(), bytes: vec![1; 100_000] }, File { name: "BR5E.zip".into(), bytes: (0..=255).collect() }]
    }

    /// A hello and an acceptance with its files read back as they were written, the progress reaching the total.
    #[test]
    fn a_transfer_reads_back_whole() {
        let mut wire = Vec::new();
        write_hello(&mut wire, "iPhone").unwrap();
        assert_eq!(read_hello(&mut wire.as_slice()).unwrap(), Hello { version: VERSION, name: "iPhone".into() });

        let mut wire = Vec::new();
        let mut written = 0;
        write_files(&mut wire, &files(), &mut |done, _| written = done).unwrap();
        let mut input = wire.as_slice();
        let Reply::Accepted { count, total } = read_reply(&mut input).unwrap() else { panic!("not accepted") };
        assert_eq!((count, total, written), (2, 100_256, 100_256));
        let mut read = 0;
        assert_eq!(read_files(&mut input, count, total, &mut |done, _| read = done).unwrap(), files());
        assert_eq!(read, total);
    }

    /// Each refusal reads back as itself; another version's is said with its version.
    #[test]
    fn refusals_read_back() {
        for reply in [Reply::Declined, Reply::Busy, Reply::OtherVersion(VERSION)] {
            let mut wire = Vec::new();
            write_refusal(&mut wire, reply).unwrap();
            assert_eq!(read_reply(&mut wire.as_slice()).unwrap(), reply);
        }
    }

    /// What isn't nettai's, a file named as a path, and more bytes than said are refused.
    #[test]
    fn bad_bytes_are_refused() {
        assert!(read_hello(&mut &b"GET / HTTP/1.1\r\n"[..]).is_err());
        let mut wire = Vec::new();
        write_files(&mut wire, &[File { name: "../evil".into(), bytes: vec![0] }], &mut |_, _| {}).unwrap();
        let mut input = wire.as_slice();
        let Reply::Accepted { count, total } = read_reply(&mut input).unwrap() else { panic!() };
        assert!(read_files(&mut input, count, total, &mut |_, _| {}).is_err());
        let mut wire = Vec::new();
        write_files(&mut wire, &files(), &mut |_, _| {}).unwrap();
        let mut input = wire.as_slice();
        let Reply::Accepted { count, total } = read_reply(&mut input).unwrap() else { panic!() };
        assert!(read_files(&mut input, count, total - 1, &mut |_, _| {}).is_err());
    }
}
