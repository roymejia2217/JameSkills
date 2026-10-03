use super::super::domain::{
    BundleEntry, EntryKind, PathValidationError, PortablePath, ValidatedInventory,
    validate_bundle_inventory,
};
use crate::Diagnostic;

const CENTRAL_HEADER_SIG: u32 = 0x0201_4b50;
const EOCD_SIG: u32 = 0x0605_4b50;
const EOCD_MIN_LEN: usize = 22;
const EOCD_MAX_COMMENT: usize = 65_535;
const CENTRAL_HEADER_LEN: usize = 46;
const MAX_FILES: usize = 2_000;
const METHOD_STORED: u16 = 0;
const METHOD_DEFLATED: u16 = 8;
const FLAG_ENCRYPTED: u16 = 1 << 0;
const UNIX_IFMT: u32 = 0o170_000;
const UNIX_IFLNK: u32 = 0o120_000;

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

    let mut entries = Vec::with_capacity(total);
    let mut offset = central_offset;
    for _ in 0..total {
        let entry = parse_central_entry(bytes, &mut offset)?;
        if let Some(entry) = entry {
            entries.push(entry);
        }
    }
    validate_bundle_inventory(&entries)
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

fn parse_central_entry(
    bytes: &[u8],
    offset: &mut usize,
) -> Result<Option<BundleEntry>, Vec<Diagnostic>> {
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
    let name = std::str::from_utf8(raw_name).map_err(|_| {
        vec![Diagnostic::error(
            "bundle.path.invalid_utf8",
            "Bundle path is not valid UTF-8.",
        )]
    })?;
    if name.ends_with('/') {
        return Ok(None);
    }
    if (external_attrs >> 16) & UNIX_IFMT == UNIX_IFLNK {
        return Err(vec![Diagnostic::error(
            "bundle.entry.kind.unsupported",
            "Bundle entries must be regular files.",
        )]);
    }
    bundle_entry_from_path(name, EntryKind::RegularFile, compressed, uncompressed).map(Some)
}
