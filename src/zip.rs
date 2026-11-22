//! Just enough of the ZIP format to take one small file (`modinfo.json`) out of a mod archive
//! held in memory: the end of central directory record, the central directory and the local
//! header of the wanted entry; stored and deflated entries. Like the game (SharpZipLib's
//! `GetEntry`), the name is matched without regard to case.
use std::io::Read;

const EOCD_SIGNATURE: u32 = 0x0605_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
const EOCD_SIZE: usize = 22;

#[derive(Debug, PartialEq, Eq)]
pub enum ZipError {
    NotAZip,
    Corrupt,
    /// ZIP64, encryption or a compression method other than stored and deflate.
    Unsupported,
    /// The entry inflates to more than the allowed size.
    TooLarge,
}

impl std::fmt::Display for ZipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotAZip => "not a zip archive",
            Self::Corrupt => "the zip archive is damaged",
            Self::Unsupported => "the zip archive uses a format this reader does not support",
            Self::TooLarge => "the file in the zip archive is too large",
        })
    }
}

fn u16_at(data: &[u8], at: usize) -> Result<u16, ZipError> {
    data.get(at..at + 2)
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        .ok_or(ZipError::Corrupt)
}

fn u32_at(data: &[u8], at: usize) -> Result<u32, ZipError> {
    data.get(at..at + 4)
        .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        .ok_or(ZipError::Corrupt)
}

fn find_eocd(data: &[u8]) -> Result<usize, ZipError> {
    if data.len() < EOCD_SIZE {
        return Err(ZipError::NotAZip);
    }
    // the record is followed by a comment of at most 65535 bytes
    let lowest = data.len().saturating_sub(EOCD_SIZE + 0xffff);
    (lowest..=data.len() - EOCD_SIZE)
        .rev()
        .find(|&at| u32_at(data, at) == Ok(EOCD_SIGNATURE))
        .ok_or(ZipError::NotAZip)
}

/// The contents of the root entry `name` (any case), at most `max_size` bytes once inflated;
/// `None` when the archive has no such entry.
pub fn read_entry(data: &[u8], name: &str, max_size: u64) -> Result<Option<Vec<u8>>, ZipError> {
    let eocd = find_eocd(data)?;
    let entries = u16_at(data, eocd + 10)?;
    let directory_offset = u32_at(data, eocd + 16)?;
    if entries == 0xffff || directory_offset == 0xffff_ffff {
        return Err(ZipError::Unsupported);
    }

    let mut at = directory_offset as usize;
    for _ in 0..entries {
        if u32_at(data, at)? != CENTRAL_SIGNATURE {
            return Err(ZipError::Corrupt);
        }
        let flags = u16_at(data, at + 8)?;
        let method = u16_at(data, at + 10)?;
        let compressed = u32_at(data, at + 20)?;
        let uncompressed = u32_at(data, at + 24)?;
        let name_len = u16_at(data, at + 28)? as usize;
        let extra_len = u16_at(data, at + 30)? as usize;
        let comment_len = u16_at(data, at + 32)? as usize;
        let local_offset = u32_at(data, at + 42)?;
        let entry_name = data
            .get(at + 46..at + 46 + name_len)
            .ok_or(ZipError::Corrupt)?;
        at += 46 + name_len + extra_len + comment_len;

        if !entry_name.eq_ignore_ascii_case(name.as_bytes()) {
            continue;
        }
        if flags & 1 != 0
            || compressed == 0xffff_ffff
            || uncompressed == 0xffff_ffff
            || local_offset == 0xffff_ffff
        {
            return Err(ZipError::Unsupported);
        }
        if u64::from(uncompressed) > max_size {
            return Err(ZipError::TooLarge);
        }

        let local = local_offset as usize;
        if u32_at(data, local)? != LOCAL_SIGNATURE {
            return Err(ZipError::Corrupt);
        }
        let start =
            local + 30 + u16_at(data, local + 26)? as usize + u16_at(data, local + 28)? as usize;
        let raw = data
            .get(start..start + compressed as usize)
            .ok_or(ZipError::Corrupt)?;

        return match method {
            0 => Ok(Some(raw.to_vec())),
            8 => {
                let mut out = Vec::with_capacity(uncompressed as usize);
                flate2::read::DeflateDecoder::new(raw)
                    .take(max_size + 1)
                    .read_to_end(&mut out)
                    .map_err(|_| ZipError::Corrupt)?;
                if out.len() as u64 > max_size {
                    return Err(ZipError::TooLarge);
                }
                Ok(Some(out))
            }
            _ => Err(ZipError::Unsupported),
        };
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOD_ZIP: &[u8] = include_bytes!("../tests/fixtures/zip/examplemod.zip");
    const STORED_ZIP: &[u8] = include_bytes!("../tests/fixtures/zip/stored-uppercase.zip");
    const NESTED_ZIP: &[u8] = include_bytes!("../tests/fixtures/zip/nested.zip");

    #[test]
    fn modinfo_is_read_from_a_deflated_archive() {
        let bytes = read_entry(MOD_ZIP, "modinfo.json", 1 << 20)
            .unwrap()
            .unwrap();
        let info = crate::modinfo::parse(&bytes).unwrap();
        assert_eq!(info.modid, "examplemod");
        assert_eq!(info.version, "1.2.0");
    }

    #[test]
    fn the_name_is_matched_in_any_case_and_stored_entries_work() {
        let bytes = read_entry(STORED_ZIP, "modinfo.json", 1 << 20)
            .unwrap()
            .unwrap();
        assert!(String::from_utf8(bytes).unwrap().contains("storedmod"));
    }

    #[test]
    fn only_the_archive_root_counts() {
        // the game only looks at the root, so a modinfo.json in a folder is no manifest
        assert_eq!(read_entry(NESTED_ZIP, "modinfo.json", 1 << 20), Ok(None));
    }

    #[test]
    fn limits_and_garbage() {
        assert_eq!(
            read_entry(MOD_ZIP, "modinfo.json", 10),
            Err(ZipError::TooLarge)
        );
        assert_eq!(
            read_entry(b"not a zip at all, sorry!", "a", 10),
            Err(ZipError::NotAZip)
        );
        let truncated = &MOD_ZIP[..MOD_ZIP.len() / 2];
        assert!(read_entry(truncated, "modinfo.json", 1 << 20).is_err());
    }
}
