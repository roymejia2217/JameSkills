use super::super::domain::{
    BundleEntry, EntryKind, PathValidationError, PortablePath, ValidatedInventory,
    validate_bundle_inventory,
};
use crate::Diagnostic;
use std::collections::BTreeMap;

/// Raw bundle bytes keyed by canonical path: the single byte-map type shared
/// by hashing, archives and staging, so no layer redefines the container.
pub type BundleFiles = BTreeMap<PortablePath, Vec<u8>>;

const LOCAL_HEADER_SIG: u32 = 0x0403_4b50;
const LOCAL_HEADER_LEN: usize = 30;
const CENTRAL_HEADER_SIG: u32 = 0x0201_4b50;
const EOCD_SIG: u32 = 0x0605_4b50;
const EOCD_MIN_LEN: usize = 22;
const EOCD_MAX_COMMENT: usize = 65_535;
const CENTRAL_HEADER_LEN: usize = 46;
const MAX_FILES: usize = 2_000;
const METHOD_STORED: u16 = 0;
const METHOD_DEFLATED: u16 = 8;
const FLAG_ENCRYPTED: u16 = 1 << 0;
const FLAG_DATA_DESCRIPTOR: u16 = 1 << 3;
const FLAG_UTF8_NAMES: u16 = 1 << 11;
const UNIX_IFMT: u32 = 0o170_000;
const UNIX_IFLNK: u32 = 0o120_000;
const MADE_BY_UNIX_SPEC: u16 = (3 << 8) | 20;
const UNIX_REGULAR_ATTRS: u32 = 0o100_644 << 16;
/// Fixed DOS date 1980-01-01; exports carry no timestamps by design.
const DETERMINISTIC_DOS_DATE: u16 = 0x0021;

fn malformed(message: &'static str) -> Vec<Diagnostic> {
    vec![Diagnostic::error("bundle.archive.malformed", message)]
}

fn unsupported(message: &'static str) -> Vec<Diagnostic> {
    vec![Diagnostic::error("bundle.archive.unsupported", message)]
}

fn map_path_error(error: PathValidationError) -> Vec<Diagnostic> {
    let (code, message) = match error {
        PathValidationError::Length => {
            ("bundle.path.too_long", "Bundle path is empty or too long.")
        }
        PathValidationError::NonCanonicalUnicode => (
            "bundle.path.non_canonical",
            "Bundle path must use Unicode NFC.",
        ),
        PathValidationError::Absolute => ("bundle.path.absolute", "Bundle path must be relative."),
        PathValidationError::InvalidCharacter => (
            "bundle.path.invalid_character",
            "Bundle path contains a non-portable character.",
        ),
        PathValidationError::InvalidComponent => (
            "bundle.path.invalid_component",
            "Bundle path contains an invalid component.",
        ),
        PathValidationError::ReservedDeviceName => (
            "bundle.path.reserved_device",
            "Bundle path uses a reserved Windows device name.",
        ),
        PathValidationError::TrailingDotOrSpace => (
            "bundle.path.trailing_dot_space",
            "Bundle path component ends in a dot or space.",
        ),
    };
    vec![Diagnostic::error(code, message)]
}

/// Builds a validated inventory entry from a portable name. Shared by the
/// archive parser and the filesystem walk so both enforce identical names.
pub fn bundle_entry_from_path(
    name: &str,
    kind: EntryKind,
    compressed_bytes: u64,
    uncompressed_bytes: u64,
) -> Result<BundleEntry, Vec<Diagnostic>> {
    let path = PortablePath::new(name.to_string()).map_err(map_path_error)?;
    Ok(BundleEntry::new(
        path,
        kind,
        compressed_bytes,
        uncompressed_bytes,
    ))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, Vec<Diagnostic>> {
    bytes
        .get(offset..offset + 2)
        .and_then(|pair| pair.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or_else(|| malformed("Archive header ends before its fields."))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, Vec<Diagnostic>> {
    bytes
        .get(offset..offset + 4)
        .and_then(|word| word.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| malformed("Archive header ends before its fields."))
}

/// Validates a `.jskill` ZIP archive from its central directory without
/// extracting or decompressing anything. Compressed and uncompressed sizes
/// come from headers, so a bomb fails on accounting before any allocation
/// proportional to its claims. Multi-disk, zip64, encrypted and unknown
/// methods are rejected explicitly.
pub fn validate_archive_entries(bytes: &[u8]) -> Result<ValidatedInventory, Vec<Diagnostic>> {
    let mut entries = Vec::new();
    for record in central_records(bytes)? {
        entries.push(bundle_entry_from_path(
            &record.name,
            EntryKind::RegularFile,
            record.compressed_bytes,
            record.uncompressed_bytes,
        )?);
    }
    validate_bundle_inventory(&entries)
}

/// Raw central-directory record shared by validation and extraction so both
/// enforce identical names, methods, flags and sizes. Directory entries never
/// reach this type: they become implicit parent paths, never published files.
struct CentralEntry {
    name: String,
    method: u16,
    flags: u16,
    checksum: u32,
    compressed_bytes: u64,
    uncompressed_bytes: u64,
    local_offset: usize,
}

fn central_records(bytes: &[u8]) -> Result<Vec<CentralEntry>, Vec<Diagnostic>> {
    let end = find_eocd(bytes)?;
    let total = read_u16(bytes, end + 10)? as usize;
    if total > MAX_FILES {
        return Err(vec![Diagnostic::error(
            "bundle.file_count.limit",
            "Bundle contains too many files.",
        )]);
    }
    let central_size = read_u32(bytes, end + 12)? as usize;
    let central_offset = read_u32(bytes, end + 16)? as usize;
    let central_end = central_offset
        .checked_add(central_size)
        .ok_or_else(|| malformed("Archive central directory overflows."))?;
    if central_end > bytes.len() || central_end != end {
        return Err(malformed(
            "Archive central directory does not match its end record.",
        ));
    }
    let comment_len = read_u16(bytes, end + 20)? as usize;
    if end + EOCD_MIN_LEN + comment_len != bytes.len() {
        return Err(malformed(
            "Archive has trailing bytes after its end record.",
        ));
    }

    let mut records = Vec::with_capacity(total);
    let mut offset = central_offset;
    for _ in 0..total {
        if let Some(record) = parse_central_record(bytes, &mut offset)? {
            records.push(record);
        }
    }
    Ok(records)
}

fn find_eocd(bytes: &[u8]) -> Result<usize, Vec<Diagnostic>> {
    if bytes.len() < EOCD_MIN_LEN {
        return Err(malformed("Archive is shorter than its end record."));
    }
    let start = bytes.len().saturating_sub(EOCD_MIN_LEN + EOCD_MAX_COMMENT);
    let mut found = None;
    for position in start..=bytes.len() - EOCD_MIN_LEN {
        if read_u32(bytes, position).is_ok_and(|sig| sig == EOCD_SIG) {
            found = Some(position);
        }
    }
    found.ok_or_else(|| malformed("Archive end record not found."))
}

fn parse_central_record(
    bytes: &[u8],
    offset: &mut usize,
) -> Result<Option<CentralEntry>, Vec<Diagnostic>> {
    if read_u32(bytes, *offset).is_ok_and(|sig| sig != CENTRAL_HEADER_SIG) {
        return Err(malformed("Archive central entry has a bad signature."));
    }
    let flags = read_u16(bytes, *offset + 8)?;
    if flags & FLAG_ENCRYPTED != 0 {
        return Err(unsupported("Encrypted archive entries are not supported."));
    }
    let method = read_u16(bytes, *offset + 10)?;
    if method != METHOD_STORED && method != METHOD_DEFLATED {
        return Err(unsupported("Archive compression method is not supported."));
    }
    let checksum = read_u32(bytes, *offset + 16)?;
    let compressed = read_u32(bytes, *offset + 20)? as u64;
    let uncompressed = read_u32(bytes, *offset + 24)? as u64;
    let name_len = read_u16(bytes, *offset + 28)? as usize;
    let extra_len = read_u16(bytes, *offset + 30)? as usize;
    let comment_len = read_u16(bytes, *offset + 32)? as usize;
    let disk = read_u16(bytes, *offset + 34)?;
    if disk != 0 {
        return Err(unsupported("Multi-disk archives are not supported."));
    }
    let external_attrs = read_u32(bytes, *offset + 38)?;
    let local_offset = read_u32(bytes, *offset + 42)? as usize;
    let name_start = offset
        .checked_add(CENTRAL_HEADER_LEN)
        .ok_or_else(|| malformed("Archive central entry overflows."))?;
    let name_end = name_start
        .checked_add(name_len)
        .ok_or_else(|| malformed("Archive entry name overflows."))?;
    let record_end = name_end
        .checked_add(extra_len)
        .and_then(|end| end.checked_add(comment_len))
        .ok_or_else(|| malformed("Archive entry record overflows."))?;
    if record_end > bytes.len() {
        return Err(malformed("Archive central entry runs past its buffer."));
    }
    *offset = record_end;

    let raw_name = &bytes[name_start..name_end];
    let name = std::str::from_utf8(raw_name)
        .map_err(|_| {
            vec![Diagnostic::error(
                "bundle.path.invalid_utf8",
                "Bundle path is not valid UTF-8.",
            )]
        })?
        .to_owned();
    if name.ends_with('/') {
        return Ok(None);
    }
    if (external_attrs >> 16) & UNIX_IFMT == UNIX_IFLNK {
        return Err(vec![Diagnostic::error(
            "bundle.entry.kind.unsupported",
            "Bundle entries must be regular files.",
        )]);
    }
    Ok(Some(CentralEntry {
        name,
        method,
        flags,
        checksum,
        compressed_bytes: compressed,
        uncompressed_bytes: uncompressed,
        local_offset,
    }))
}

const fn build_crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut index = 0;
    while index < 256 {
        let mut crc = index as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
            bit += 1;
        }
        table[index] = crc;
        index += 1;
    }
    table
}

const CRC_TABLE: [u32; 256] = build_crc_table();

/// IEEE CRC-32 over raw bytes. Hand-rolled so deterministic exports need no
/// third-party codec; standard ZIP tools verify this checksum on extraction.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc = CRC_TABLE[(crc as u8 ^ byte) as usize] ^ (crc >> 8);
    }
    !crc
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn fit_u16(len: usize) -> Result<u16, Vec<Diagnostic>> {
    u16::try_from(len).map_err(|_| {
        vec![Diagnostic::error(
            "bundle.archive.too_large",
            "Bundle archive entry exceeds its size limit.",
        )]
    })
}

fn fit_u32(len: usize) -> Result<u32, Vec<Diagnostic>> {
    u32::try_from(len).map_err(|_| {
        vec![Diagnostic::error(
            "bundle.archive.too_large",
            "Bundle archive entry exceeds its size limit.",
        )]
    })
}

/// Writes a deterministic `.jskill` ZIP: stored entries in portable byte
/// order, fixed DOS date, no extra fields or comments. Byte-identical output
/// for identical input; readable by standard ZIP tools, which verify the
/// CRC-32 of every entry. Nested archives stay opaque payload bytes and are
/// never recursed into on read.
pub fn write_bundle_archive(files: &BundleFiles) -> Result<Vec<u8>, Vec<Diagnostic>> {
    if files.len() > MAX_FILES {
        return Err(vec![Diagnostic::error(
            "bundle.file_count.limit",
            "Bundle contains too many files.",
        )]);
    }
    let mut out = Vec::new();
    let mut offsets = Vec::with_capacity(files.len());
    for (path, content) in files {
        let name = path.as_str().as_bytes();
        let size = fit_u32(content.len())?;
        offsets.push((fit_u32(out.len())?, name, size, crc32(content)));
        push_u32(&mut out, LOCAL_HEADER_SIG);
        push_u16(&mut out, 20);
        push_u16(&mut out, FLAG_UTF8_NAMES);
        push_u16(&mut out, METHOD_STORED);
        push_u16(&mut out, 0);
        push_u16(&mut out, DETERMINISTIC_DOS_DATE);
        push_u32(&mut out, crc32(content));
        push_u32(&mut out, size);
        push_u32(&mut out, size);
        push_u16(&mut out, fit_u16(name.len())?);
        push_u16(&mut out, 0);
        out.extend_from_slice(name);
        out.extend_from_slice(content);
    }
    let central_start = fit_u32(out.len())?;
    for (offset, name, size, checksum) in &offsets {
        push_u32(&mut out, CENTRAL_HEADER_SIG);
        push_u16(&mut out, MADE_BY_UNIX_SPEC);
        push_u16(&mut out, 20);
        push_u16(&mut out, FLAG_UTF8_NAMES);
        push_u16(&mut out, METHOD_STORED);
        push_u16(&mut out, 0);
        push_u16(&mut out, DETERMINISTIC_DOS_DATE);
        push_u32(&mut out, *checksum);
        push_u32(&mut out, *size);
        push_u32(&mut out, *size);
        push_u16(&mut out, fit_u16(name.len())?);
        push_u16(&mut out, 0);
        push_u16(&mut out, 0);
        push_u16(&mut out, 0);
        push_u16(&mut out, 0);
        push_u32(&mut out, UNIX_REGULAR_ATTRS);
        push_u32(&mut out, *offset);
        out.extend_from_slice(name);
    }
    let central_end = fit_u32(out.len())?;
    push_u32(&mut out, EOCD_SIG);
    push_u16(&mut out, 0);
    push_u16(&mut out, 0);
    push_u16(&mut out, fit_u16(files.len())?);
    push_u16(&mut out, fit_u16(files.len())?);
    push_u32(&mut out, central_end - central_start);
    push_u32(&mut out, central_start);
    push_u16(&mut out, 0);
    Ok(out)
}

/// Extracts stored entries already validated by [`validate_archive_entries`]
/// on the same bytes. Local headers must agree with the central directory on
/// method, sizes and names; every checksum is verified and extraction stops
/// at the validated totals, so a bomb fails before its bytes are trusted.
/// Deflated entries validate as inventory but stay unextracted until a
/// verified inflate path exists.
pub fn extract_archive_files(
    bytes: &[u8],
    inventory: &ValidatedInventory,
) -> Result<BundleFiles, Vec<Diagnostic>> {
    let mut budget = inventory.total_uncompressed_bytes();
    let mut files = BTreeMap::new();
    for record in central_records(bytes)? {
        if record.method != METHOD_STORED {
            return Err(unsupported("Only stored archive entries can be extracted."));
        }
        if record.flags & FLAG_DATA_DESCRIPTOR != 0 {
            return Err(unsupported(
                "Archive entries with data descriptors are not supported.",
            ));
        }
        let data = local_data(bytes, &record)?;
        if data.len() as u64 > budget {
            return Err(vec![Diagnostic::error(
                "bundle.total_size.limit",
                "Bundle exceeds its size limit.",
            )]);
        }
        budget -= data.len() as u64;
        let path = PortablePath::new(record.name.clone()).map_err(map_path_error)?;
        if files.insert(path, data.to_vec()).is_some() {
            return Err(vec![Diagnostic::error(
                "bundle.path.duplicate",
                "Bundle contains a duplicate path.",
            )]);
        }
    }
    Ok(files)
}

fn local_data<'bytes>(
    bytes: &'bytes [u8],
    record: &CentralEntry,
) -> Result<&'bytes [u8], Vec<Diagnostic>> {
    let header_end = record
        .local_offset
        .checked_add(LOCAL_HEADER_LEN)
        .ok_or_else(|| malformed("Archive local header overflows."))?;
    if header_end > bytes.len() {
        return Err(malformed("Archive local header runs past its buffer."));
    }
    if read_u32(bytes, record.local_offset).is_ok_and(|sig| sig != LOCAL_HEADER_SIG) {
        return Err(malformed("Archive local header has a bad signature."));
    }
    if read_u16(bytes, record.local_offset + 6).is_ok_and(|flags| flags & FLAG_DATA_DESCRIPTOR != 0)
    {
        return Err(malformed(
            "Archive local header disagrees with its central directory.",
        ));
    }
    if read_u16(bytes, record.local_offset + 8).is_ok_and(|method| method != record.method) {
        return Err(malformed(
            "Archive local header disagrees with its central directory.",
        ));
    }
    if read_u32(bytes, record.local_offset + 18)
        .is_ok_and(|compressed| u64::from(compressed) != record.compressed_bytes)
        || read_u32(bytes, record.local_offset + 22)
            .is_ok_and(|uncompressed| u64::from(uncompressed) != record.uncompressed_bytes)
    {
        return Err(malformed(
            "Archive local sizes disagree with the central directory.",
        ));
    }
    let name_len = read_u16(bytes, record.local_offset + 26)? as usize;
    let extra_len = read_u16(bytes, record.local_offset + 28)? as usize;
    let data_start = header_end
        .checked_add(name_len)
        .and_then(|end| end.checked_add(extra_len))
        .ok_or_else(|| malformed("Archive entry name overflows."))?;
    if bytes.len() < data_start
        || &bytes[data_start - name_len - extra_len..data_start - extra_len]
            != record.name.as_bytes()
    {
        return Err(malformed(
            "Archive local name disagrees with the central directory.",
        ));
    }
    let data_end = data_start
        .checked_add(record.uncompressed_bytes as usize)
        .ok_or_else(|| malformed("Archive entry data overflows."))?;
    if data_end > bytes.len() {
        return Err(malformed("Archive entry data runs past its buffer."));
    }
    let data = &bytes[data_start..data_end];
    if crc32(data) != record.checksum {
        return Err(vec![Diagnostic::error(
            "bundle.archive.checksum_mismatch",
            "Archive entry checksum does not match its bytes.",
        )]);
    }
    Ok(data)
}
