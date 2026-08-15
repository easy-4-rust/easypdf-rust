//! CMS 结构的 DER 编码与解码辅助工具。
use crate::crypto::CryptoError;

/// 将长度值编码为 DER 长度字段。
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

/// 构造 DER TLV（标签-长度-值）三元组。
pub(super) fn der_tlv(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + 4 + value.len());
    out.push(tag);
    out.extend(der_len(value.len()));
    out.extend(value);
    out
}

/// 构造 DER SEQUENCE。
pub(super) fn der_seq(value: &[u8]) -> Vec<u8> {
    der_tlv(0x30, value)
}
/// 构造 DER SET。
pub(super) fn der_set(value: &[u8]) -> Vec<u8> {
    der_tlv(0x31, value)
}

/// 构造 DER INTEGER，自动处理前导零。
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

/// 构造 DER OCTET STRING。
pub(super) fn der_octets(value: &[u8]) -> Vec<u8> {
    der_tlv(0x04, value)
}

/// 构造带上下文标签的 DER 字段。
pub(super) fn der_ctx(tag: u8, constructed: bool, value: &[u8]) -> Vec<u8> {
    let tag_byte = if constructed { 0xA0 | tag } else { 0x80 | tag };
    der_tlv(tag_byte, value)
}

/// 将多个字节切片拼接为一个 `Vec<u8>`。
pub(super) fn concat(parts: &[&[u8]]) -> Vec<u8> {
    let len: usize = parts.iter().map(|p| p.len()).sum();
    let mut out = Vec::with_capacity(len);
    for part in parts {
        out.extend_from_slice(part);
    }
    out
}

/// DER 流式读取器，逐字段解析 DER 编码数据。
pub(super) struct DerReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> DerReader<'a> {
    /// 创建新的 DER 读取器。
    pub(super) fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
    /// 是否已读取到末尾。
    #[allow(dead_code)]
    pub(super) fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }
    /// 查看当前标签字节，不移动读取位置。
    pub(super) fn peek_tag(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }
    /// 读取一个字节。
    pub(super) fn read_byte(&mut self) -> Result<u8, CryptoError> {
        self.data
            .get(self.pos)
            .copied()
            .inspect(|_| {
                self.pos += 1;
            })
            .ok_or_else(|| CryptoError::InvalidSignedPdf("unexpected end of DER data".into()))
    }
    /// 读取 DER 长度字段。
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
    /// 读取指定长度的字节切片。
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
    /// 读取一个完整的 TLV（标签-长度-值）三元组。
    pub(super) fn read_tlv(&mut self) -> Result<(u8, &'a [u8]), CryptoError> {
        let tag = self.read_byte()?;
        let len = self.read_length()?;
        let value = self.read_bytes(len)?;
        Ok((tag, value))
    }
    /// 读取 TLV 并验证标签是否匹配期望值。
    pub(super) fn expect_tag(&mut self, expected: u8) -> Result<&'a [u8], CryptoError> {
        let (tag, value) = self.read_tlv()?;
        if tag != expected {
            return Err(CryptoError::InvalidSignedPdf(format!(
                "expected DER tag 0x{expected:02X}, got 0x{tag:02X}"
            )));
        }
        Ok(value)
    }
    /// 读取 DER SEQUENCE。
    pub(super) fn read_sequence(&mut self) -> Result<&'a [u8], CryptoError> {
        self.expect_tag(0x30)
    }
    /// 读取 DER SET。
    pub(super) fn read_set(&mut self) -> Result<&'a [u8], CryptoError> {
        self.expect_tag(0x31)
    }
    /// 读取 DER INTEGER，去除多余的前导零。
    pub(super) fn read_integer(&mut self) -> Result<&'a [u8], CryptoError> {
        let value = self.expect_tag(0x02)?;
        if value.len() > 1 && value[0] == 0 && value[1] & 0x80 != 0 {
            Ok(&value[1..])
        } else {
            Ok(value)
        }
    }
    /// 读取 DER OCTET STRING。
    pub(super) fn read_octet_string(&mut self) -> Result<&'a [u8], CryptoError> {
        self.expect_tag(0x04)
    }
    /// 读取 DER OID。
    pub(super) fn read_oid(&mut self) -> Result<&'a [u8], CryptoError> {
        self.expect_tag(0x06)
    }
    /// 读取 DER BIT STRING，跳过首字节的未使用位数。
    #[allow(dead_code)]
    pub(super) fn read_bit_string(&mut self) -> Result<&'a [u8], CryptoError> {
        let value = self.expect_tag(0x03)?;
        if value.is_empty() {
            return Err(CryptoError::InvalidSignedPdf("empty BIT STRING".into()));
        }
        Ok(&value[1..])
    }
    /// 读取隐式上下文标签字段，不存在时返回 `None`。
    pub(super) fn read_ctx_implicit(&mut self, tag: u8) -> Result<Option<&'a [u8]>, CryptoError> {
        let expected = 0xA0 | tag;
        if self.peek_tag() != Some(expected) {
            return Ok(None);
        }
        let (t, value) = self.read_tlv()?;
        debug_assert_eq!(t, expected);
        Ok(Some(value))
    }
    /// 跳过当前 TLV 字段。
    pub(super) fn skip_field(&mut self) -> Result<(), CryptoError> {
        let _ = self.read_tlv()?;
        Ok(())
    }
}
