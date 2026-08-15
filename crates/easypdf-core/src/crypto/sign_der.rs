//! DER encoding and decoding helpers for CMS structures.
use crate::crypto::CryptoError;

#[allow(clippy::cast_possible_truncation)]
pub(super) fn der_len(len: usize) -> Vec<u8> {
    if len < 0x80 {
        vec![len as u8]
    } else if len < 0x100 {
        vec![0x81, len as u8]
    } else if len < 0x10_000 {
        vec![0x82, (len >> 8) as u8, len as u8]
    } else {
        vec![0x83, (len >> 16) as u8, (len >> 8) as u8, len as u8]
    }
}

pub(super) fn der_tlv(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + 4 + value.len());
    out.push(tag);
    out.extend(der_len(value.len()));
    out.extend(value);
    out
}

pub(super) fn der_seq(value: &[u8]) -> Vec<u8> {
    der_tlv(0x30, value)
}
pub(super) fn der_set(value: &[u8]) -> Vec<u8> {
    der_tlv(0x31, value)
}

pub(super) fn der_int(value: &[u8]) -> Vec<u8> {
    let mut content = Vec::with_capacity(value.len() + 1);
    if value.is_empty() {
        content.push(0);
    } else if value[0] & 0x80 != 0 {
        content.push(0);
        content.extend(value);
    } else {
        let mut start = 0;
        while start < value.len() - 1 && value[start] == 0 && value[start + 1] & 0x80 == 0 {
            start += 1;
        }
        content.extend(&value[start..]);
    }
    der_tlv(0x02, &content)
}

pub(super) fn der_octets(value: &[u8]) -> Vec<u8> {
    der_tlv(0x04, value)
}

pub(super) fn der_ctx(tag: u8, constructed: bool, value: &[u8]) -> Vec<u8> {
    let tag_byte = if constructed { 0xA0 | tag } else { 0x80 | tag };
    der_tlv(tag_byte, value)
}

pub(super) fn concat(parts: &[&[u8]]) -> Vec<u8> {
    let len: usize = parts.iter().map(|p| p.len()).sum();
    let mut out = Vec::with_capacity(len);
    for part in parts {
        out.extend_from_slice(part);
    }
    out
}

pub(super) struct DerReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> DerReader<'a> {
    pub(super) fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
    #[allow(dead_code)]
    pub(super) fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }
    pub(super) fn peek_tag(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }
    pub(super) fn read_byte(&mut self) -> Result<u8, CryptoError> {
        self.data
            .get(self.pos)
            .copied()
            .inspect(|_| {
                self.pos += 1;
            })
            .ok_or_else(|| CryptoError::InvalidSignedPdf("unexpected end of DER data".into()))
    }
    pub(super) fn read_length(&mut self) -> Result<usize, CryptoError> {
        let first = self.read_byte()?;
        if first & 0x80 == 0 {
            Ok(first as usize)
        } else {
            let num_bytes = (first & 0x7F) as usize;
            if num_bytes == 0 || self.pos + num_bytes > self.data.len() {
                return Err(CryptoError::InvalidSignedPdf(
                    "invalid DER length encoding".into(),
                ));
            }
            let mut len = 0usize;
            for _ in 0..num_bytes {
                len = (len << 8) | self.data[self.pos] as usize;
                self.pos += 1;
            }
            Ok(len)
        }
    }
    pub(super) fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], CryptoError> {
        if self.pos + len > self.data.len() {
            return Err(CryptoError::InvalidSignedPdf(
                "unexpected end of DER data".into(),
            ));
        }
        let bytes = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok(bytes)
    }
    pub(super) fn read_tlv(&mut self) -> Result<(u8, &'a [u8]), CryptoError> {
        let tag = self.read_byte()?;
        let len = self.read_length()?;
        let value = self.read_bytes(len)?;
        Ok((tag, value))
    }
    pub(super) fn expect_tag(&mut self, expected: u8) -> Result<&'a [u8], CryptoError> {
        let (tag, value) = self.read_tlv()?;
        if tag != expected {
            return Err(CryptoError::InvalidSignedPdf(format!(
                "expected DER tag 0x{expected:02X}, got 0x{tag:02X}"
            )));
        }
        Ok(value)
    }
    pub(super) fn read_sequence(&mut self) -> Result<&'a [u8], CryptoError> {
        self.expect_tag(0x30)
    }
    pub(super) fn read_set(&mut self) -> Result<&'a [u8], CryptoError> {
        self.expect_tag(0x31)
    }
    pub(super) fn read_integer(&mut self) -> Result<&'a [u8], CryptoError> {
        let value = self.expect_tag(0x02)?;
        if value.len() > 1 && value[0] == 0 && value[1] & 0x80 != 0 {
            Ok(&value[1..])
        } else {
            Ok(value)
        }
    }
    pub(super) fn read_octet_string(&mut self) -> Result<&'a [u8], CryptoError> {
        self.expect_tag(0x04)
    }
    pub(super) fn read_oid(&mut self) -> Result<&'a [u8], CryptoError> {
        self.expect_tag(0x06)
    }
    #[allow(dead_code)]
    pub(super) fn read_bit_string(&mut self) -> Result<&'a [u8], CryptoError> {
        let value = self.expect_tag(0x03)?;
        if value.is_empty() {
            return Err(CryptoError::InvalidSignedPdf("empty BIT STRING".into()));
        }
        Ok(&value[1..])
    }
    pub(super) fn read_ctx_implicit(&mut self, tag: u8) -> Result<Option<&'a [u8]>, CryptoError> {
        let expected = 0xA0 | tag;
        if self.peek_tag() != Some(expected) {
            return Ok(None);
        }
        let (t, value) = self.read_tlv()?;
        debug_assert_eq!(t, expected);
        Ok(Some(value))
    }
    pub(super) fn skip_field(&mut self) -> Result<(), CryptoError> {
        let _ = self.read_tlv()?;
        Ok(())
    }
}
