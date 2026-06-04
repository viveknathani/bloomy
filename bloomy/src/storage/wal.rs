use std::io::Read;
use std::io::Write;

use crate::error::Error;
use crate::error::Result;
use crate::types::Key;
use crate::types::Value;

pub const FILE_MAGIC: &[u8; 8] = b"BLOOMWAL";
pub const FORMAT_VERSION: u16 = 1;
pub const FILE_HEADER_BYTES: usize = FILE_MAGIC.len() + 2;
pub const RECORD_HEADER_BYTES: usize = 4;
pub const RECORD_CHECKSUM_BYTES: usize = 4;
pub const MAX_RECORD_PAYLOAD_BYTES: u32 = 64 * 1024 * 1024;

const PUT_KIND: u8 = 1;
const DELETE_KIND: u8 = 2;
const RECORD_METADATA_BYTES: usize = 1 + 4 + 4;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WalRecord {
    Put { key: Key, value: Value },
    Delete { key: Key },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ReadRecord {
    Record(WalRecord),
    CleanEof,
    PartialTail,
}

pub fn write_header(mut writer: impl Write) -> Result<()> {
    writer.write_all(FILE_MAGIC)?;
    writer.write_all(&FORMAT_VERSION.to_le_bytes())?;
    Ok(())
}

pub fn read_header(mut reader: impl Read) -> Result<()> {
    let mut header = [0; FILE_HEADER_BYTES];
    reader.read_exact(&mut header)?;

    if &header[..FILE_MAGIC.len()] != FILE_MAGIC {
        return Err(Error::Message("invalid WAL file magic".to_string()));
    }

    let version = u16::from_le_bytes([header[8], header[9]]);
    if version != FORMAT_VERSION {
        return Err(Error::Message(format!(
            "unsupported WAL format version: {version}"
        )));
    }

    Ok(())
}

pub fn encode_record(record: &WalRecord) -> Result<Vec<u8>> {
    let payload = encode_payload(record)?;
    let payload_len = u32::try_from(payload.len())
        .map_err(|_| Error::Message("WAL record payload is too large".to_string()))?;

    if payload_len > MAX_RECORD_PAYLOAD_BYTES {
        return Err(Error::Message(
            "WAL record payload is too large".to_string(),
        ));
    }

    let mut encoded =
        Vec::with_capacity(RECORD_HEADER_BYTES + payload.len() + RECORD_CHECKSUM_BYTES);
    encoded.extend_from_slice(&payload_len.to_le_bytes());
    encoded.extend_from_slice(&payload);

    let checksum = crc32(&encoded);
    encoded.extend_from_slice(&checksum.to_le_bytes());
    Ok(encoded)
}

pub fn write_record(mut writer: impl Write, record: &WalRecord) -> Result<usize> {
    let encoded = encode_record(record)?;
    writer.write_all(&encoded)?;
    Ok(encoded.len())
}

pub fn read_record(mut reader: impl Read) -> Result<ReadRecord> {
    let mut length_bytes = [0; RECORD_HEADER_BYTES];
    match read_exact_or_partial(&mut reader, &mut length_bytes)? {
        ReadExact::Complete => {}
        ReadExact::CleanEof => return Ok(ReadRecord::CleanEof),
        ReadExact::Partial => return Ok(ReadRecord::PartialTail),
    }

    let payload_len = u32::from_le_bytes(length_bytes);
    validate_payload_len(payload_len)?;

    let mut payload = vec![0; payload_len as usize];
    if read_exact_or_partial(&mut reader, &mut payload)? != ReadExact::Complete {
        return Ok(ReadRecord::PartialTail);
    }

    let mut checksum_bytes = [0; RECORD_CHECKSUM_BYTES];
    if read_exact_or_partial(&mut reader, &mut checksum_bytes)? != ReadExact::Complete {
        return Ok(ReadRecord::PartialTail);
    }

    let expected = u32::from_le_bytes(checksum_bytes);
    let mut checksummed = Vec::with_capacity(RECORD_HEADER_BYTES + payload.len());
    checksummed.extend_from_slice(&length_bytes);
    checksummed.extend_from_slice(&payload);

    let actual = crc32(&checksummed);
    if actual != expected {
        return Err(Error::Message("WAL record checksum mismatch".to_string()));
    }

    decode_payload(&payload).map(ReadRecord::Record)
}

fn encode_payload(record: &WalRecord) -> Result<Vec<u8>> {
    let (kind, key, value) = match record {
        WalRecord::Put { key, value } => (PUT_KIND, key.as_slice(), value.as_slice()),
        WalRecord::Delete { key } => (DELETE_KIND, key.as_slice(), &[][..]),
    };

    if key.is_empty() {
        return Err(Error::Message(
            "WAL record key must not be empty".to_string(),
        ));
    }

    let key_len = u32::try_from(key.len())
        .map_err(|_| Error::Message("WAL record key is too large".to_string()))?;
    let value_len = u32::try_from(value.len())
        .map_err(|_| Error::Message("WAL record value is too large".to_string()))?;

    let mut payload = Vec::with_capacity(RECORD_METADATA_BYTES + key.len() + value.len());
    payload.push(kind);
    payload.extend_from_slice(&key_len.to_le_bytes());
    payload.extend_from_slice(&value_len.to_le_bytes());
    payload.extend_from_slice(key);
    payload.extend_from_slice(value);
    Ok(payload)
}

fn decode_payload(payload: &[u8]) -> Result<WalRecord> {
    if payload.len() < RECORD_METADATA_BYTES {
        return Err(Error::Message(
            "WAL record payload is too short".to_string(),
        ));
    }

    let kind = payload[0];
    let key_len = u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]) as usize;
    let value_len = u32::from_le_bytes([payload[5], payload[6], payload[7], payload[8]]) as usize;

    if key_len == 0 {
        return Err(Error::Message(
            "WAL record key must not be empty".to_string(),
        ));
    }

    let expected_len = RECORD_METADATA_BYTES
        .checked_add(key_len)
        .and_then(|len| len.checked_add(value_len))
        .ok_or_else(|| Error::Message("WAL record payload length overflows".to_string()))?;

    if payload.len() != expected_len {
        return Err(Error::Message(
            "WAL record payload length mismatch".to_string(),
        ));
    }

    let key_start = RECORD_METADATA_BYTES;
    let value_start = key_start + key_len;
    let key = payload[key_start..value_start].to_vec();
    let value = payload[value_start..].to_vec();

    match kind {
        PUT_KIND => Ok(WalRecord::Put { key, value }),
        DELETE_KIND => {
            if value_len != 0 {
                return Err(Error::Message(
                    "WAL delete record must not include a value".to_string(),
                ));
            }

            Ok(WalRecord::Delete { key })
        }
        _ => Err(Error::Message(format!("unknown WAL record kind: {kind}"))),
    }
}

fn validate_payload_len(payload_len: u32) -> Result<()> {
    if payload_len > MAX_RECORD_PAYLOAD_BYTES {
        return Err(Error::Message(format!(
            "WAL record payload length exceeds maximum: {payload_len}"
        )));
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ReadExact {
    Complete,
    CleanEof,
    Partial,
}

fn read_exact_or_partial(mut reader: impl Read, buffer: &mut [u8]) -> Result<ReadExact> {
    let mut read = 0;

    while read < buffer.len() {
        let bytes = reader.read(&mut buffer[read..])?;
        if bytes == 0 {
            return if read == 0 {
                Ok(ReadExact::CleanEof)
            } else {
                Ok(ReadExact::Partial)
            };
        }

        read += bytes;
    }

    Ok(ReadExact::Complete)
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff;

    for byte in bytes {
        crc ^= u32::from(*byte);

        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }

    !crc
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn header_round_trip_succeeds() {
        let mut bytes = Vec::new();

        write_header(&mut bytes).unwrap();
        read_header(Cursor::new(bytes)).unwrap();
    }

    #[test]
    fn read_header_rejects_invalid_magic() {
        let bytes = b"NOTAWAL!\x01\x00";

        let error = read_header(Cursor::new(bytes)).unwrap_err();

        assert!(error.to_string().contains("invalid WAL file magic"));
    }

    #[test]
    fn put_record_round_trip_preserves_key_and_value() {
        let record = WalRecord::Put {
            key: b"alpha".to_vec(),
            value: b"one".to_vec(),
        };
        let encoded = encode_record(&record).unwrap();

        let decoded = read_record(Cursor::new(encoded)).unwrap();

        assert_eq!(decoded, ReadRecord::Record(record));
    }

    #[test]
    fn delete_record_round_trip_preserves_tombstone_key() {
        let record = WalRecord::Delete {
            key: b"alpha".to_vec(),
        };
        let encoded = encode_record(&record).unwrap();

        let decoded = read_record(Cursor::new(encoded)).unwrap();

        assert_eq!(decoded, ReadRecord::Record(record));
    }

    #[test]
    fn empty_value_put_is_valid() {
        let record = WalRecord::Put {
            key: b"alpha".to_vec(),
            value: Vec::new(),
        };
        let encoded = encode_record(&record).unwrap();

        let decoded = read_record(Cursor::new(encoded)).unwrap();

        assert_eq!(decoded, ReadRecord::Record(record));
    }

    #[test]
    fn empty_key_is_rejected() {
        let record = WalRecord::Put {
            key: Vec::new(),
            value: b"one".to_vec(),
        };

        let error = encode_record(&record).unwrap_err();

        assert!(error.to_string().contains("key must not be empty"));
    }

    #[test]
    fn clean_eof_between_records_is_reported() {
        let decoded = read_record(Cursor::new(Vec::new())).unwrap();

        assert_eq!(decoded, ReadRecord::CleanEof);
    }

    #[test]
    fn partial_tail_is_reported_without_corruption_error() {
        let mut encoded = encode_record(&WalRecord::Put {
            key: b"alpha".to_vec(),
            value: b"one".to_vec(),
        })
        .unwrap();
        encoded.pop();

        let decoded = read_record(Cursor::new(encoded)).unwrap();

        assert_eq!(decoded, ReadRecord::PartialTail);
    }

    #[test]
    fn checksum_mismatch_is_rejected() {
        let mut encoded = encode_record(&WalRecord::Put {
            key: b"alpha".to_vec(),
            value: b"one".to_vec(),
        })
        .unwrap();
        encoded[RECORD_HEADER_BYTES + RECORD_METADATA_BYTES] ^= 0xff;

        let error = read_record(Cursor::new(encoded)).unwrap_err();

        assert!(error.to_string().contains("checksum mismatch"));
    }

    #[test]
    fn delete_payload_with_value_is_rejected() {
        let mut payload = Vec::new();
        payload.push(DELETE_KIND);
        payload.extend_from_slice(&5u32.to_le_bytes());
        payload.extend_from_slice(&3u32.to_le_bytes());
        payload.extend_from_slice(b"alpha");
        payload.extend_from_slice(b"one");

        let mut encoded = Vec::new();
        encoded.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        encoded.extend_from_slice(&payload);
        let checksum = crc32(&encoded);
        encoded.extend_from_slice(&checksum.to_le_bytes());

        let error = read_record(Cursor::new(encoded)).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("delete record must not include a value")
        );
    }
}
