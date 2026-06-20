//! Byte-level reader for XDV/DVI input.
//!
//! Provides convenient methods for reading signed/unsigned integers in big-endian
//! byte order, matching the DVI format specification.

use std::io::{Read, Seek, SeekFrom};

use crate::error::XdvError;

/// A byte reader that tracks position and provides DVI-format reads.
#[derive(Debug)]
pub struct ByteReader<R: Read> {
    inner: R,
    offset: usize,
}

impl<R: Read> ByteReader<R> {
    /// Wrap a reader.
    pub fn new(inner: R) -> Self {
        Self { inner, offset: 0 }
    }

    /// Current byte offset in the input.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Read exactly `n` bytes, returning an error on unexpected EOF.
    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), XdvError> {
        match self.inner.read_exact(buf) {
            Ok(()) => {
                self.offset += buf.len();
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                Err(XdvError::UnexpectedEof {
                    offset: self.offset,
                    needed: buf.len(),
                })
            }
            Err(e) => Err(XdvError::Io {
                offset: self.offset,
                message: e.to_string(),
            }),
        }
    }

    /// Read a single byte.
    pub fn read_u8(&mut self) -> Result<u8, XdvError> {
        let mut buf = [0u8; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    /// Read a 1-byte unsigned integer.
    pub fn read_u1(&mut self) -> Result<u8, XdvError> {
        self.read_u8()
    }

    /// Read a 2-byte unsigned integer (big-endian).
    pub fn read_u2(&mut self) -> Result<u16, XdvError> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    /// Read a 3-byte unsigned integer (big-endian).
    pub fn read_u3(&mut self) -> Result<u32, XdvError> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf[1..])?;
        Ok(u32::from_be_bytes(buf))
    }

    /// Read a 4-byte unsigned integer (big-endian).
    pub fn read_u4(&mut self) -> Result<u32, XdvError> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }

    /// Read a 1-byte signed integer (two's complement).
    pub fn read_i1(&mut self) -> Result<i8, XdvError> {
        self.read_u8().map(|v| v as i8)
    }

    /// Read a 2-byte signed integer (big-endian, two's complement).
    pub fn read_i2(&mut self) -> Result<i32, XdvError> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        let raw = u16::from_be_bytes(buf);
        Ok(if raw >= 0x8000 {
            (raw as i32) - 0x10000
        } else {
            raw as i32
        })
    }

    /// Read a 3-byte signed integer (big-endian, two's complement).
    pub fn read_i3(&mut self) -> Result<i32, XdvError> {
        self.read_u3().map(|v| {
            if v >= 0x800000 {
                (v as i32) - 0x1000000
            } else {
                v as i32
            }
        })
    }

    /// Read a 4-byte signed integer (big-endian, two's complement).
    pub fn read_i4(&mut self) -> Result<i32, XdvError> {
        self.read_u4().map(|v| v as i32)
    }

    /// Read a Pascal-style string (1-byte length followed by bytes).
    pub fn read_pascal_string(&mut self) -> Result<String, XdvError> {
        let len = self.read_u1()? as usize;
        let mut buf = vec![0u8; len];
        self.read_exact(&mut buf)?;
        String::from_utf8(buf).map_err(|_| XdvError::InvalidUtf8 {
            offset: self.offset - len,
        })
    }

    /// Read N raw bytes.
    pub fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>, XdvError> {
        let mut buf = vec![0u8; n];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }
}

impl<R: Read + Seek> ByteReader<R> {
    /// Seek to an absolute offset.
    pub fn seek(&mut self, pos: usize) -> Result<(), XdvError> {
        self.inner
            .seek(SeekFrom::Start(pos as u64))
            .map_err(|e| XdvError::Io {
                offset: self.offset,
                message: e.to_string(),
            })?;
        self.offset = pos;
        Ok(())
    }

    /// Return the current position.
    pub fn position(&mut self) -> Result<u64, XdvError> {
        self.inner
            .stream_position()
            .map_err(|e| XdvError::Io {
                offset: self.offset,
                message: e.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_u2() {
        let data = [0x12, 0x34];
        let mut r = ByteReader::new(&data[..]);
        assert_eq!(r.read_u2().unwrap(), 0x1234);
    }

    #[test]
    fn test_read_i2_negative() {
        let data = [0xFF, 0xFF]; // -1 as signed 16-bit big-endian
        let mut r = ByteReader::new(&data[..]);
        assert_eq!(r.read_i2().unwrap(), -1);
    }

    #[test]
    fn test_read_i3() {
        let data = [0x00, 0x01, 0x00]; // 256
        let mut r = ByteReader::new(&data[..]);
        assert_eq!(r.read_i3().unwrap(), 256);
    }

    #[test]
    fn test_read_pascal_string() {
        let data = [3, b'h', b'i', b'!'];
        let mut r = ByteReader::new(&data[..]);
        assert_eq!(r.read_pascal_string().unwrap(), "hi!");
    }
}
