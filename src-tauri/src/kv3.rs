//! Valve's binary KeyValues3 (KV3) as used inside compiled Source 2 resources.
//!
//! Reading covers versions 2 to 5, uncompressed or LZ4 block-compressed.
//! Writing produces version 5 without compression, which is how Valve's own
//! compiler stores small blocks such as a Panorama layout's `LaCo` tree.
//! Binary blobs and zstd are not supported and are reported as such.

const MAGIC_BASE: u32 = 0x4B56_3300;
const TRAILER: u32 = 0xFFEE_DD00;
const MAX_DECOMPRESSED_BYTES: usize = 32 * 1024 * 1024;
const MAX_DEPTH: usize = 256;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    Null,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
    /// A value carrying a KV3 flag (resource, panorama, soundevent, ...).
    Flagged(u8, Box<Value>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Document {
    pub format: [u8; 16],
    pub root: Value,
}

fn invalid() -> String {
    "kv3_invalid".to_string()
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(
        bytes
            .get(at..at + 4)
            .ok_or_else(invalid)?
            .try_into()
            .unwrap(),
    ))
}

fn i32_at(bytes: &[u8], at: usize) -> Result<i32, String> {
    Ok(u32_at(bytes, at)? as i32)
}

/// LZ4 block format (no frame).
pub(crate) fn lz4_decompress(source: &[u8], size: usize) -> Result<Vec<u8>, String> {
    if size > MAX_DECOMPRESSED_BYTES {
        return Err(invalid());
    }
    let mut out = Vec::with_capacity(size);
    let mut i = 0usize;
    let byte = |i: usize| source.get(i).copied().ok_or_else(invalid);
    while i < source.len() {
        let token = byte(i)?;
        i += 1;
        let mut literal = (token >> 4) as usize;
        if literal == 15 {
            loop {
                let extra = byte(i)?;
                i += 1;
                literal += extra as usize;
                if extra != 255 {
                    break;
                }
            }
        }
        out.extend_from_slice(source.get(i..i + literal).ok_or_else(invalid)?);
        i += literal;
        if i >= source.len() {
            break;
        }
        let offset = byte(i)? as usize | (byte(i + 1)? as usize) << 8;
        i += 2;
        let mut length = (token & 15) as usize;
        if length == 15 {
            loop {
                let extra = byte(i)?;
                i += 1;
                length += extra as usize;
                if extra != 255 {
                    break;
                }
            }
        }
        length += 4;
        if offset == 0 || offset > out.len() || out.len() + length > size {
            return Err(invalid());
        }
        let start = out.len() - offset;
        for k in 0..length {
            let value = out[start + k];
            out.push(value);
        }
    }
    if out.len() != size {
        return Err(invalid());
    }
    Ok(out)
}

/// Reads typed little-endian values from the 1/2/4/8-byte segments.
struct Segments<'a> {
    parts: [&'a [u8]; 4],
    positions: [usize; 4],
}

impl<'a> Segments<'a> {
    fn new(b1: &'a [u8], b2: &'a [u8], b4: &'a [u8], b8: &'a [u8]) -> Self {
        Self {
            parts: [b1, b2, b4, b8],
            positions: [0; 4],
        }
    }

    fn take(&mut self, width: usize) -> Result<&'a [u8], String> {
        let index = match width {
            1 => 0,
            2 => 1,
            4 => 2,
            _ => 3,
        };
        let at = self.positions[index];
        let slice = self.parts[index].get(at..at + width).ok_or_else(invalid)?;
        self.positions[index] += width;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn i16(&mut self) -> Result<i16, String> {
        Ok(i16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i64(&mut self) -> Result<i64, String> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn f64(&mut self) -> Result<f64, String> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn finished(&self) -> bool {
        self.positions
            .iter()
            .zip(self.parts.iter())
            .all(|(position, part)| *position == part.len())
    }
}

fn segment<'a>(
    buffer: &'a [u8],
    offset: &mut usize,
    count: usize,
    width: usize,
) -> Result<&'a [u8], String> {
    if count == 0 {
        return Ok(&[]);
    }
    *offset = offset.next_multiple_of(width);
    let end = offset
        .checked_add(count.checked_mul(width).ok_or_else(invalid)?)
        .ok_or_else(invalid)?;
    let slice = buffer.get(*offset..end).ok_or_else(invalid)?;
    *offset = end;
    Ok(slice)
}

struct Reader<'a> {
    version: u8,
    main: Segments<'a>,
    aux: Segments<'a>,
    types: &'a [u8],
    type_position: usize,
    object_lengths: Vec<i32>,
    object_position: usize,
    strings: Vec<String>,
}

impl Reader<'_> {
    fn read_type(&mut self) -> Result<(u8, u8), String> {
        let mut kind = *self.types.get(self.type_position).ok_or_else(invalid)?;
        self.type_position += 1;
        let mut flag = 0;
        if kind & 0x80 != 0 {
            kind &= if self.version >= 3 { 0x3f } else { 0x7f };
            flag = *self.types.get(self.type_position).ok_or_else(invalid)?;
            self.type_position += 1;
        }
        Ok((kind, flag))
    }

    fn string(&self, index: i32) -> Result<String, String> {
        if index == -1 {
            return Ok(String::new());
        }
        self.strings
            .get(index as usize)
            .cloned()
            .ok_or_else(invalid)
    }

    fn read_value(&mut self, kind: u8, flag: u8, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return Err(invalid());
        }
        let value = match kind {
            1 => Value::Null,
            2 => Value::Bool(self.main.u8()? != 0),
            13 => Value::Bool(true),
            14 => Value::Bool(false),
            15 => Value::Int(0),
            16 => Value::Int(1),
            17 => Value::Float(0.0),
            18 => Value::Float(1.0),
            23 => Value::Int(self.main.u8()? as i64),
            20 => Value::Int(self.main.i16()? as i64),
            21 => Value::Int(self.main.u16()? as i64),
            11 => Value::Int(self.main.i32()? as i64),
            12 => Value::UInt(self.main.u32()? as u64),
            19 => Value::Float(self.main.f32()? as f64),
            3 => Value::Int(self.main.i64()?),
            4 => Value::UInt(self.main.u64()?),
            5 => Value::Float(self.main.f64()?),
            6 => {
                let index = self.main.i32()?;
                Value::String(self.string(index)?)
            }
            8 => {
                let count = self.main.i32()?;
                if count < 0 {
                    return Err(invalid());
                }
                let mut items = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    let (kind, flag) = self.read_type()?;
                    items.push(self.read_value(kind, flag, depth + 1)?);
                }
                Value::Array(items)
            }
            10 | 24 => {
                let count = if kind == 24 {
                    self.main.u8()? as i32
                } else {
                    self.main.i32()?
                };
                if count < 0 {
                    return Err(invalid());
                }
                let (item_kind, item_flag) = self.read_type()?;
                let mut items = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    items.push(self.read_value(item_kind, item_flag, depth + 1)?);
                }
                Value::Array(items)
            }
            25 => {
                let count = self.main.u8()?;
                let (item_kind, item_flag) = self.read_type()?;
                std::mem::swap(&mut self.main, &mut self.aux);
                let mut items = Vec::with_capacity(count as usize);
                let mut result = Ok(());
                for _ in 0..count {
                    match self.read_value(item_kind, item_flag, depth + 1) {
                        Ok(item) => items.push(item),
                        Err(error) => {
                            result = Err(error);
                            break;
                        }
                    }
                }
                std::mem::swap(&mut self.main, &mut self.aux);
                result?;
                Value::Array(items)
            }
            9 => {
                let count = if self.version >= 5 {
                    let count = *self
                        .object_lengths
                        .get(self.object_position)
                        .ok_or_else(invalid)?;
                    self.object_position += 1;
                    count
                } else {
                    self.main.i32()?
                };
                if count < 0 {
                    return Err(invalid());
                }
                let mut entries = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    let (kind, flag) = self.read_type()?;
                    let key_index = self.main.i32()?;
                    let key = self.string(key_index)?;
                    entries.push((key, self.read_value(kind, flag, depth + 1)?));
                }
                Value::Object(entries)
            }
            _ => return Err("kv3_type_unsupported".to_string()),
        };
        Ok(if flag != 0 {
            Value::Flagged(flag, Box::new(value))
        } else {
            value
        })
    }
}

fn decompress(method: u32, source: &[u8], size: usize) -> Result<Vec<u8>, String> {
    match method {
        0 => source.get(..size).map(<[u8]>::to_vec).ok_or_else(invalid),
        1 => lz4_decompress(source, size),
        _ => Err("kv3_compression_unsupported".to_string()),
    }
}

fn read_strings(bytes: &[u8], count: usize) -> Result<(Vec<String>, usize), String> {
    let mut strings = Vec::with_capacity(count);
    let mut at = 0usize;
    for _ in 0..count {
        let end = bytes
            .get(at..)
            .and_then(|rest| rest.iter().position(|byte| *byte == 0))
            .ok_or_else(invalid)?;
        strings.push(String::from_utf8(bytes[at..at + end].to_vec()).map_err(|_| invalid())?);
        at += end + 1;
    }
    Ok((strings, at))
}

pub(crate) fn parse(raw: &[u8]) -> Result<Document, String> {
    let magic = u32_at(raw, 0)?;
    let version = (magic & 0xff) as u8;
    if magic & 0xffff_ff00 != MAGIC_BASE || !(2..=5).contains(&version) {
        return Err("kv3_version_unsupported".to_string());
    }
    let format: [u8; 16] = raw.get(4..20).ok_or_else(invalid)?.try_into().unwrap();
    let method = u32_at(raw, 20)?;
    let count1 = i32_at(raw, 28)? as usize;
    let count4 = i32_at(raw, 32)? as usize;
    let count8 = i32_at(raw, 36)? as usize;
    let type_count = i32_at(raw, 40)? as usize;
    let uncompressed = i32_at(raw, 48)? as usize;
    let compressed = i32_at(raw, 52)? as usize;
    let blocks = i32_at(raw, 56)?;
    if blocks != 0 {
        return Err("kv3_blobs_unsupported".to_string());
    }
    let mut position = 64usize;
    let (count2, _) = if version >= 4 {
        position += 8;
        (i32_at(raw, 64)? as usize, i32_at(raw, 68)?)
    } else {
        (0, 0)
    };
    if version < 5 {
        let input = raw.get(position..).ok_or_else(invalid)?;
        let buffer = decompress(
            method,
            if method == 0 {
                input
            } else {
                input.get(..compressed).ok_or_else(invalid)?
            },
            uncompressed,
        )?;
        let mut offset = 0usize;
        let b1 = segment(&buffer, &mut offset, count1, 1)?;
        let b2 = segment(&buffer, &mut offset, count2, 2)?;
        let b4 = segment(&buffer, &mut offset, count4, 4)?;
        let b8 = if count8 > 0 {
            segment(&buffer, &mut offset, count8, 8)?
        } else {
            offset = offset.next_multiple_of(8);
            &[]
        };
        let string_count = i32_at(b4, 0)? as usize;
        let strings_start = offset;
        let (strings, used) =
            read_strings(buffer.get(offset..).ok_or_else(invalid)?, string_count)?;
        offset += used;
        let types_len = type_count
            .checked_sub(offset - strings_start)
            .ok_or_else(invalid)?;
        let types = buffer.get(offset..offset + types_len).ok_or_else(invalid)?;
        offset += types_len;
        if u32_at(&buffer, offset)? != TRAILER {
            return Err(invalid());
        }
        let mut reader = Reader {
            version,
            main: Segments::new(b1, b2, b4.get(4..).ok_or_else(invalid)?, b8),
            aux: Segments::new(&[], &[], &[], &[]),
            types,
            type_position: 0,
            object_lengths: Vec::new(),
            object_position: 0,
            strings,
        };
        let (kind, flag) = reader.read_type()?;
        let root = reader.read_value(kind, flag, 0)?;
        if reader.type_position != types.len() {
            return Err(invalid());
        }
        return Ok(Document { format, root });
    }
    let fields = (0..12)
        .map(|index| i32_at(raw, position + 4 * index))
        .collect::<Result<Vec<_>, _>>()?;
    position += 48;
    let [u1, cs1, u2, cs2, m1, m2, m4, m8, _, objects, _, _] = fields[..] else {
        return Err(invalid());
    };
    let (buffer1, buffer2) = if method == 0 {
        let first = raw
            .get(position..position + u1 as usize)
            .ok_or_else(invalid)?
            .to_vec();
        position += u1 as usize;
        let second = raw
            .get(position..position + u2 as usize)
            .ok_or_else(invalid)?
            .to_vec();
        (first, second)
    } else {
        let first = decompress(
            method,
            raw.get(position..position + cs1 as usize)
                .ok_or_else(invalid)?,
            u1 as usize,
        )?;
        position += cs1 as usize;
        let second = decompress(
            method,
            raw.get(position..position + cs2 as usize)
                .ok_or_else(invalid)?,
            u2 as usize,
        )?;
        (first, second)
    };
    let mut offset = 0usize;
    let a1 = segment(&buffer1, &mut offset, count1, 1)?;
    let a2 = segment(&buffer1, &mut offset, count2, 2)?;
    let a4 = segment(&buffer1, &mut offset, count4, 4)?;
    let a8 = segment(&buffer1, &mut offset, count8, 8)?;
    if offset != buffer1.len() {
        return Err(invalid());
    }
    let string_count = i32_at(a4, 0)? as usize;
    let (strings, used) = read_strings(a1, string_count)?;
    let objects = objects.max(0) as usize;
    let object_lengths = (0..objects)
        .map(|index| i32_at(&buffer2, 4 * index))
        .collect::<Result<Vec<_>, _>>()?;
    let mut offset = objects * 4;
    let b1 = segment(&buffer2, &mut offset, m1 as usize, 1)?;
    let b2 = segment(&buffer2, &mut offset, m2 as usize, 2)?;
    let b4 = segment(&buffer2, &mut offset, m4 as usize, 4)?;
    let b8 = segment(&buffer2, &mut offset, m8 as usize, 8)?;
    let types = buffer2
        .get(offset..offset + type_count)
        .ok_or_else(invalid)?;
    offset += type_count;
    if u32_at(&buffer2, offset)? != TRAILER || offset + 4 != buffer2.len() {
        return Err(invalid());
    }
    let mut reader = Reader {
        version,
        main: Segments::new(b1, b2, b4, b8),
        aux: Segments::new(&a1[used..], a2, a4.get(4..).ok_or_else(invalid)?, a8),
        types,
        type_position: 0,
        object_lengths,
        object_position: 0,
        strings,
    };
    let (kind, flag) = reader.read_type()?;
    let root = reader.read_value(kind, flag, 0)?;
    if reader.type_position != types.len()
        || reader.object_position != reader.object_lengths.len()
        || !reader.main.finished()
        || !reader.aux.finished()
    {
        return Err(invalid());
    }
    Ok(Document { format, root })
}

#[derive(Default)]
struct Writer {
    strings: Vec<String>,
    string_ids: std::collections::HashMap<String, i32>,
    b4: Vec<u8>,
    b8: Vec<u8>,
    types: Vec<u8>,
    object_lengths: Vec<i32>,
    arrays: usize,
}

impl Writer {
    fn string_id(&mut self, value: &str) -> i32 {
        if let Some(id) = self.string_ids.get(value) {
            return *id;
        }
        let id = self.strings.len() as i32;
        self.strings.push(value.to_string());
        self.string_ids.insert(value.to_string(), id);
        id
    }

    fn write_type(&mut self, kind: u8, flag: u8) {
        if flag != 0 {
            self.types.push(kind | 0x80);
            self.types.push(flag);
        } else {
            self.types.push(kind);
        }
    }

    fn kind(value: &Value) -> u8 {
        match value {
            Value::Null => 1,
            Value::Bool(true) => 13,
            Value::Bool(false) => 14,
            Value::Int(0) => 15,
            Value::Int(1) => 16,
            Value::Int(value) if i32::try_from(*value).is_ok() => 11,
            Value::Int(_) => 3,
            Value::UInt(_) => 4,
            Value::Float(value) if *value == 0.0 => 17,
            Value::Float(value) if *value == 1.0 => 18,
            Value::Float(_) => 5,
            Value::String(_) => 6,
            Value::Array(_) => 8,
            Value::Object(_) => 9,
            Value::Flagged(_, inner) => Self::kind(inner),
        }
    }

    fn payload(&mut self, value: &Value) {
        match (Self::kind(value), value) {
            (11, Value::Int(value)) => self.b4.extend_from_slice(&(*value as i32).to_le_bytes()),
            (3, Value::Int(value)) => self.b8.extend_from_slice(&value.to_le_bytes()),
            (_, Value::UInt(value)) => self.b8.extend_from_slice(&value.to_le_bytes()),
            (5, Value::Float(value)) => self.b8.extend_from_slice(&value.to_le_bytes()),
            (_, Value::String(value)) => {
                let id = self.string_id(value);
                self.b4.extend_from_slice(&id.to_le_bytes());
            }
            (_, Value::Array(items)) => {
                self.arrays += 1;
                self.b4
                    .extend_from_slice(&(items.len() as i32).to_le_bytes());
                for item in items {
                    self.value(item);
                }
            }
            (_, Value::Object(entries)) => {
                self.object_lengths.push(entries.len() as i32);
                for (key, item) in entries {
                    let (flag, inner) = match item {
                        Value::Flagged(flag, inner) => (*flag, inner.as_ref()),
                        other => (0, other),
                    };
                    self.write_type(Self::kind(inner), flag);
                    let id = self.string_id(key);
                    self.b4.extend_from_slice(&id.to_le_bytes());
                    self.payload(inner);
                }
            }
            (_, Value::Flagged(_, inner)) => self.payload(inner),
            _ => {}
        }
    }

    fn value(&mut self, value: &Value) {
        let (flag, inner) = match value {
            Value::Flagged(flag, inner) => (*flag, inner.as_ref()),
            other => (0, other),
        };
        self.write_type(Self::kind(inner), flag);
        self.payload(inner);
    }
}

fn pad(buffer: &mut Vec<u8>, width: usize) {
    buffer.resize(buffer.len().next_multiple_of(width), 0);
}

/// Serializes as uncompressed KV3 version 5.
pub(crate) fn write(document: &Document) -> Vec<u8> {
    let mut writer = Writer::default();
    writer.value(&document.root);
    let string_bytes = writer
        .strings
        .iter()
        .flat_map(|value| value.bytes().chain(std::iter::once(0)))
        .collect::<Vec<u8>>();
    let mut buffer1 = string_bytes.clone();
    pad(&mut buffer1, 4);
    buffer1.extend_from_slice(&(writer.strings.len() as i32).to_le_bytes());
    let mut buffer2 = writer
        .object_lengths
        .iter()
        .flat_map(|length| length.to_le_bytes())
        .collect::<Vec<u8>>();
    for (part, width) in [(&writer.b4, 4usize), (&writer.b8, 8)] {
        if !part.is_empty() {
            pad(&mut buffer2, width);
            buffer2.extend_from_slice(part);
        }
    }
    buffer2.extend_from_slice(&writer.types);
    buffer2.extend_from_slice(&TRAILER.to_le_bytes());
    let mut out = (MAGIC_BASE | 5).to_le_bytes().to_vec();
    out.extend_from_slice(&document.format);
    out.extend_from_slice(&0u32.to_le_bytes()); // no compression
    out.extend_from_slice(&0u16.to_le_bytes()); // dictionary
    out.extend_from_slice(&0u16.to_le_bytes()); // frame size
    for value in [string_bytes.len() as i32, 1, 0, writer.types.len() as i32] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&(writer.object_lengths.len() as u16).to_le_bytes());
    out.extend_from_slice(&(writer.arrays as u16).to_le_bytes());
    for value in [(buffer1.len() + buffer2.len()) as i32, 0, 0, 0, 0, 0] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    for value in [
        buffer1.len() as i32,
        0,
        buffer2.len() as i32,
        0,
        0,
        0,
        (writer.b4.len() / 4) as i32,
        (writer.b8.len() / 8) as i32,
        0,
        writer.object_lengths.len() as i32,
        writer.arrays as i32,
        0,
    ] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&buffer1);
    out.extend_from_slice(&buffer2);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Document {
        Document {
            format: [7; 16],
            root: Value::Object(vec![
                (
                    "m_AST".to_string(),
                    Value::Object(vec![(
                        "m_pRoot".to_string(),
                        Value::Object(vec![
                            ("eType".to_string(), Value::String("ROOT".to_string())),
                            (
                                "sourceLineColumn".to_string(),
                                Value::Array(vec![Value::Int(3), Value::Int(17)]),
                            ),
                            ("big".to_string(), Value::Int(1 << 40)),
                            ("ratio".to_string(), Value::Float(0.5)),
                            ("none".to_string(), Value::Null),
                            ("yes".to_string(), Value::Bool(true)),
                            (
                                "flagged".to_string(),
                                Value::Flagged(1, Box::new(Value::String("s2r://x".to_string()))),
                            ),
                        ]),
                    )]),
                ),
                ("empty".to_string(), Value::Array(Vec::new())),
            ]),
        }
    }

    #[test]
    fn written_documents_read_back_identically() {
        let document = sample();
        let bytes = write(&document);
        assert_eq!(parse(&bytes).expect("parses"), document);
    }

    #[test]
    fn lz4_round_trip_of_a_known_block() {
        // "abcabcabcabc": 3 literals then a match of 9 at offset 3.
        let block = [0x35, b'a', b'b', b'c', 3, 0];
        assert_eq!(lz4_decompress(&block, 12).expect("lz4"), b"abcabcabcabc");
        assert!(lz4_decompress(&block, 11).is_err());
        assert!(lz4_decompress(&[0x10, b'a', 9, 0], 10).is_err());
    }

    #[test]
    fn rejects_unknown_headers() {
        assert!(parse(b"nope").is_err());
        let mut bytes = write(&sample());
        bytes[0] = 9;
        assert!(parse(&bytes).is_err());
    }
}
