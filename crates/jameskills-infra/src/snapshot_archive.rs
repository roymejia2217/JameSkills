use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read, Write},
};

use jameskills_core::{
    AppError, AppResult,
    domain::{
        BACKUP_AEAD_TAG_LEN, CANONICAL_HASH_VERSION, DeviceId, MAX_ENCRYPTED_PAYLOAD_BYTES,
        MAX_SNAPSHOT_ARCHIVES_BYTES, MAX_SNAPSHOT_BUNDLE_BYTES, MAX_SNAPSHOT_METADATA_ENTRIES,
        SNAPSHOT_SCHEMA_VERSION, SnapshotBundle, SnapshotHead, SnapshotId, SnapshotPayload,
        SnapshotRevision, VaultId,
    },
};
use serde::{Deserialize, Serialize};
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter, write::SimpleFileOptions};

const MAX_SNAPSHOT_MANIFEST_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SNAPSHOT_ZIP_BYTES: u64 = MAX_ENCRYPTED_PAYLOAD_BYTES - BACKUP_AEAD_TAG_LEN;
const MAX_SNAPSHOT_ZIP_ENTRIES: usize = MAX_SNAPSHOT_METADATA_ENTRIES + 1;
const MAX_ZIP_NAME_BYTES: u64 = 96;
const MAX_CENTRAL_RECORD_BYTES: u64 = 46 + MAX_ZIP_NAME_BYTES;
const ZIP_LOCAL_HEADER_BYTES: u64 = 30;
const ZIP_CENTRAL_HEADER_BYTES: u64 = 46;
const ZIP_END_RECORD_BYTES: u64 = 22;
const ZIP64_END_RECORD_BYTES: u64 = 76;
const ZIP_LOCAL_FILE_HEADER_SIGNATURE: u32 = 0x0403_4b50;
const ZIP_END_OF_CENTRAL_DIRECTORY_SIGNATURE: u32 = 0x0605_4b50;
const ZIP64_END_OF_CENTRAL_DIRECTORY_SIGNATURE: u32 = 0x0606_4b50;
const ZIP64_END_OF_CENTRAL_DIRECTORY_LOCATOR_SIGNATURE: u32 = 0x0706_4b50;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotManifestV1 {
    schema_version: u16,
    canonical_hash_version: u16,
    vault_id: VaultId,
    snapshot_id: SnapshotId,
    parents: Vec<SnapshotId>,
    device_id: DeviceId,
    library_generation: u64,
    created_at: String,
    revisions: Vec<SnapshotRevision>,
    heads: Vec<SnapshotHead>,
    bundle_hashes: Vec<jameskills_core::domain::ContentHash>,
}

/// ZIP stored codec for the plaintext payload. `decode` returns only a bounded,
/// unverified DTO; the caller must authenticate its containing AEAD first and
/// then invoke `SnapshotPayload::verify` before exposing it to restore.
pub fn encode(payload: &SnapshotPayload) -> AppResult<Vec<u8>> {
    payload.validate()?;
    let manifest = SnapshotManifestV1 {
        schema_version: payload.schema_version(),
        canonical_hash_version: payload.canonical_hash_version(),
        vault_id: payload.vault_id(),
        snapshot_id: payload.snapshot_id(),
        parents: payload.parents().to_vec(),
        device_id: payload.device_id(),
        library_generation: payload.library_generation(),
        created_at: payload.created_at().to_owned(),
        revisions: payload.revisions().to_vec(),
        heads: payload.heads().to_vec(),
        bundle_hashes: payload
            .bundles()
            .iter()
            .map(|bundle| bundle.content_hash().clone())
            .collect(),
    };
    let manifest_bytes = serde_json::to_vec(&manifest).map_err(|_| archive_error())?;
    if manifest_bytes.len() as u64 > MAX_SNAPSHOT_MANIFEST_BYTES {
        return Err(archive_error());
    }
    let entries_count = payload.bundles().len() + 1;
    if entries_count > MAX_SNAPSHOT_ZIP_ENTRIES {
        return Err(archive_error());
    }
    let upper_bound = zip_size_upper_bound(manifest_bytes.len(), payload)?;
    let upper_bound = usize::try_from(upper_bound).map_err(|_| archive_error())?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(upper_bound)
        .map_err(|_| archive_error())?;
    let mut writer = ZipWriter::new(Cursor::new(bytes));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(DateTime::DEFAULT)
        .unix_permissions(0o644);
    writer
        .start_file("snapshot.json", options)
        .map_err(|_| archive_error())?;
    writer
        .write_all(&manifest_bytes)
        .map_err(|_| archive_error())?;
    for bundle in payload.bundles() {
        let name = format!("bundles/{}.jskill", bundle.content_hash().as_str());
        writer
            .start_file(name, options)
            .map_err(|_| archive_error())?;
        writer
            .write_all(bundle.archive_bytes())
            .map_err(|_| archive_error())?;
    }
    let bytes = writer.finish().map_err(|_| archive_error())?.into_inner();
    if bytes.len() as u64 > MAX_SNAPSHOT_ZIP_BYTES {
        return Err(archive_error());
    }
    Ok(bytes)
}

pub fn decode(bytes: &[u8]) -> AppResult<SnapshotPayload> {
    let expected_entries = preflight_zip(bytes)?;
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| archive_error())?;
    if archive.len() != expected_entries || !archive.comment().is_empty() {
        return Err(archive_error());
    }

    let mut manifest_bytes = None;
    let mut bundles = BTreeMap::new();
    let mut seen_paths = BTreeSet::new();
    let mut total_size = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|_| archive_error())?;
        let name = entry.name().to_owned();
        if name.len() as u64 > MAX_ZIP_NAME_BYTES
            || entry.encrypted()
            || entry.is_dir()
            || entry.compression() != CompressionMethod::Stored
            || entry.compressed_size() != entry.size()
            || !entry.comment().is_empty()
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 != 0 && mode & 0o170000 != 0o100000)
        {
            return Err(archive_error());
        }
        let portable = jameskills_core::domain::PortablePath::new(name.clone())
            .map_err(|_| archive_error())?;
        if portable.as_str() != name || !seen_paths.insert(name.clone()) {
            return Err(archive_error());
        }
        let size = entry.size();
        total_size = total_size.checked_add(size).ok_or_else(archive_error)?;
        if total_size > MAX_SNAPSHOT_ARCHIVES_BYTES as u64 {
            return Err(archive_error());
        }
        let entry_limit = if name == "snapshot.json" {
            MAX_SNAPSHOT_MANIFEST_BYTES
        } else {
            u64::try_from(MAX_SNAPSHOT_BUNDLE_BYTES).map_err(|_| archive_error())?
        };
        if size > entry_limit {
            return Err(archive_error());
        }
        let capacity = usize::try_from(size).map_err(|_| archive_error())?;
        let mut contents = Vec::new();
        contents
            .try_reserve_exact(capacity)
            .map_err(|_| archive_error())?;
        (&mut entry)
            .take(size.saturating_add(1))
            .read_to_end(&mut contents)
            .map_err(|_| archive_error())?;
        if contents.len() != capacity {
            return Err(archive_error());
        }
        if name == "snapshot.json" {
            if manifest_bytes.replace(contents).is_some() {
                return Err(archive_error());
            }
        } else {
            let hash = bundle_hash_from_path(&name)?;
            let bundle = SnapshotBundle::new(hash.clone(), contents)?;
            if bundles.insert(hash, bundle).is_some() {
                return Err(archive_error());
            }
        }
    }

    let manifest_bytes = manifest_bytes.ok_or_else(archive_error)?;
    let manifest: SnapshotManifestV1 =
        serde_json::from_slice(&manifest_bytes).map_err(|_| archive_error())?;
    if manifest.schema_version != SNAPSHOT_SCHEMA_VERSION
        || manifest.canonical_hash_version != CANONICAL_HASH_VERSION
        || manifest.bundle_hashes.len() != bundles.len()
    {
        return Err(archive_error());
    }
    let actual_bundle_hashes = bundles.keys().cloned().collect::<Vec<_>>();
    if manifest.bundle_hashes != actual_bundle_hashes {
        return Err(archive_error());
    }
    SnapshotPayload::new(
        manifest.vault_id,
        manifest.snapshot_id,
        manifest.parents,
        manifest.device_id,
        manifest.library_generation,
        manifest.created_at,
        manifest.schema_version,
        manifest.canonical_hash_version,
        manifest.revisions,
        manifest.heads,
        bundles.into_values().collect(),
    )
}

fn zip_size_upper_bound(manifest_len: usize, payload: &SnapshotPayload) -> AppResult<u64> {
    let mut data_bytes = u64::try_from(manifest_len).map_err(|_| archive_error())?;
    let manifest_name_len = "snapshot.json".len() as u64;
    let mut names_bytes = manifest_name_len;
    for bundle in payload.bundles() {
        data_bytes = data_bytes
            .checked_add(u64::try_from(bundle.archive_bytes().len()).map_err(|_| archive_error())?)
            .ok_or_else(archive_error)?;
        names_bytes = names_bytes
            .checked_add(8 + 64 + 7)
            .ok_or_else(archive_error)?;
    }
    let entry_count = u64::try_from(payload.bundles().len() + 1).map_err(|_| archive_error())?;
    let per_entry = entry_count
        .checked_mul(ZIP_LOCAL_HEADER_BYTES + ZIP_CENTRAL_HEADER_BYTES)
        .and_then(|overhead| overhead.checked_add(names_bytes.checked_mul(2)?))
        .ok_or_else(archive_error)?;
    let zip64 = if entry_count >= u16::MAX as u64 {
        ZIP64_END_RECORD_BYTES
    } else {
        0
    };
    data_bytes
        .checked_add(per_entry)
        .and_then(|size| size.checked_add(ZIP_END_RECORD_BYTES + zip64))
        .filter(|size| *size <= MAX_SNAPSHOT_ZIP_BYTES)
        .ok_or_else(archive_error)
}

fn preflight_zip(bytes: &[u8]) -> AppResult<usize> {
    if bytes.len() as u64 > MAX_SNAPSHOT_ZIP_BYTES || bytes.len() < 22 {
        return Err(archive_error());
    }
    let eocd = bytes.len() - 22;
    if read_u32_le(bytes, eocd)? != ZIP_END_OF_CENTRAL_DIRECTORY_SIGNATURE
        || read_u16_le(bytes, eocd + 4)? != 0
        || read_u16_le(bytes, eocd + 6)? != 0
        || read_u16_le(bytes, eocd + 20)? != 0
        || eocd + 22 != bytes.len()
    {
        return Err(archive_error());
    }
    let disk_entries = read_u16_le(bytes, eocd + 8)?;
    let total_entries = read_u16_le(bytes, eocd + 10)?;
    let central_size32 = read_u32_le(bytes, eocd + 12)?;
    let central_offset32 = read_u32_le(bytes, eocd + 16)?;
    let has_zip64 = disk_entries == u16::MAX
        || total_entries == u16::MAX
        || central_size32 == u32::MAX
        || central_offset32 == u32::MAX;
    let (entry_count, central_size, central_offset, central_end_boundary) = if has_zip64 {
        parse_zip64_directory(bytes, eocd)?
    } else {
        if disk_entries != total_entries {
            return Err(archive_error());
        }
        (
            u64::from(total_entries),
            u64::from(central_size32),
            u64::from(central_offset32),
            eocd as u64,
        )
    };
    let maximum_records = entry_count
        .checked_mul(MAX_CENTRAL_RECORD_BYTES)
        .ok_or_else(archive_error)?;
    if entry_count == 0
        || entry_count > MAX_SNAPSHOT_ZIP_ENTRIES as u64
        || central_size > maximum_records
        || central_offset == 0
        || central_offset
            .checked_add(central_size)
            .is_none_or(|end| end != central_end_boundary)
        || read_u32_le(bytes, 0)? != ZIP_LOCAL_FILE_HEADER_SIGNATURE
        || usize::try_from(central_end_boundary).is_err()
    {
        return Err(archive_error());
    }
    usize::try_from(entry_count).map_err(|_| archive_error())
}

fn parse_zip64_directory(bytes: &[u8], eocd: usize) -> AppResult<(u64, u64, u64, u64)> {
    let locator_offset = eocd.checked_sub(20).ok_or_else(archive_error)?;
    if read_u32_le(bytes, locator_offset)? != ZIP64_END_OF_CENTRAL_DIRECTORY_LOCATOR_SIGNATURE
        || read_u32_le(bytes, locator_offset + 4)? != 0
        || read_u32_le(bytes, locator_offset + 16)? != 1
    {
        return Err(archive_error());
    }
    let zip64_offset =
        usize::try_from(read_u64_le(bytes, locator_offset + 8)?).map_err(|_| archive_error())?;
    if zip64_offset > bytes.len().saturating_sub(56) {
        return Err(archive_error());
    }
    if read_u32_le(bytes, zip64_offset)? != ZIP64_END_OF_CENTRAL_DIRECTORY_SIGNATURE {
        return Err(archive_error());
    }
    let record_size = read_u64_le(bytes, zip64_offset + 4)?;
    if record_size < 44
        || (zip64_offset as u64)
            .checked_add(12)
            .and_then(|offset| offset.checked_add(record_size))
            .is_none_or(|end| end != locator_offset as u64)
        || read_u32_le(bytes, zip64_offset + 16)? != 0
        || read_u32_le(bytes, zip64_offset + 20)? != 0
    {
        return Err(archive_error());
    }
    let entries_on_disk = read_u64_le(bytes, zip64_offset + 24)?;
    let total_entries = read_u64_le(bytes, zip64_offset + 32)?;
    if entries_on_disk != total_entries {
        return Err(archive_error());
    }
    Ok((
        total_entries,
        read_u64_le(bytes, zip64_offset + 40)?,
        read_u64_le(bytes, zip64_offset + 48)?,
        zip64_offset as u64,
    ))
}

fn bundle_hash_from_path(name: &str) -> AppResult<jameskills_core::domain::ContentHash> {
    let hash = name
        .strip_prefix("bundles/")
        .and_then(|name| name.strip_suffix(".jskill"))
        .ok_or_else(archive_error)?;
    if hash.len() != 64 {
        return Err(archive_error());
    }
    jameskills_core::domain::ContentHash::parse_hex(hash).map_err(|_| archive_error())
}

fn read_u16_le(bytes: &[u8], offset: usize) -> AppResult<u16> {
    let end = offset.checked_add(2).ok_or_else(archive_error)?;
    let raw: [u8; 2] = bytes
        .get(offset..end)
        .ok_or_else(archive_error)?
        .try_into()
        .map_err(|_| archive_error())?;
    Ok(u16::from_le_bytes(raw))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> AppResult<u32> {
    let end = offset.checked_add(4).ok_or_else(archive_error)?;
    let raw: [u8; 4] = bytes
        .get(offset..end)
        .ok_or_else(archive_error)?
        .try_into()
        .map_err(|_| archive_error())?;
    Ok(u32::from_le_bytes(raw))
}

fn read_u64_le(bytes: &[u8], offset: usize) -> AppResult<u64> {
    let end = offset.checked_add(8).ok_or_else(archive_error)?;
    let raw: [u8; 8] = bytes
        .get(offset..end)
        .ok_or_else(archive_error)?
        .try_into()
        .map_err(|_| archive_error())?;
    Ok(u64::from_le_bytes(raw))
}

fn archive_error() -> AppError {
    AppError::Validation(vec![jameskills_core::Diagnostic::error(
        "sync.snapshot.archive.invalid",
        "Snapshot archive is invalid or exceeds bounded resource limits.",
    )])
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

    use super::preflight_zip;

    #[test]
    fn preflight_reads_zip64_entry_count_before_constructing_the_archive_reader() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(DateTime::DEFAULT);
        for index in 0..=u16::MAX {
            writer
                .start_file(format!("entry-{index:05}"), options)
                .unwrap();
        }
        let bytes = writer.finish().unwrap().into_inner();
        assert_eq!(preflight_zip(&bytes).unwrap(), usize::from(u16::MAX) + 1);
    }
}
