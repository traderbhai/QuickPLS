//! Resource-bounded integrity checks for QuickPLS ZIP project archives.
//!
//! This module deliberately has no dependency on the project document schema.
//! Callers can therefore preflight and checksum-verify a future archive before
//! attempting any best-effort decoding of `project.json`.

use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Seek},
};
use thiserror::Error;
use uuid::Uuid;
use zip::ZipArchive;

pub(crate) const MANIFEST_ENTRY_NAME: &str = "manifest.json";
pub(crate) const PROJECT_ENTRY_NAME: &str = "project.json";

/// Default archive limits applied before any entry body is read or allocated.
///
/// The limits intentionally leave ample room for large, multi-dataset research
/// projects while placing a finite ceiling on hostile ZIP metadata. A single
/// entry may be up to 512 MiB, the archive may declare up to 8 GiB in total,
/// and up to 16,384 entries are permitted.
pub(crate) const DEFAULT_ARCHIVE_LIMITS: ArchiveLimits = ArchiveLimits {
    max_entries: 16_384,
    max_entry_name_bytes: 1_024,
    max_entry_uncompressed_bytes: 512 * 1024 * 1024,
    max_total_uncompressed_bytes: 8 * 1024 * 1024 * 1024,
};

/// `manifest.json` is metadata rather than scientific data. Keeping its read
/// allocation below 32 MiB still supports thousands of checksum entries while
/// preventing a valid-looking central directory from requesting a huge buffer.
pub(crate) const MAX_MANIFEST_UNCOMPRESSED_BYTES: u64 = 32 * 1024 * 1024;
/// `project.json` contains typed metadata and result envelopes, not raw data.
/// A 256 MiB ceiling prevents hostile allocation while remaining far above
/// expected desktop projects. Arrow datasets use the separate 512 MiB cap.
pub(crate) const MAX_PROJECT_DOCUMENT_UNCOMPRESSED_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ArchiveLimits {
    pub(crate) max_entries: usize,
    pub(crate) max_entry_name_bytes: usize,
    pub(crate) max_entry_uncompressed_bytes: u64,
    pub(crate) max_total_uncompressed_bytes: u64,
}

#[derive(Debug, Error)]
pub(crate) enum ArchiveIntegrityError {
    #[error("archive contains {actual} entries; the limit is {limit}")]
    TooManyEntries { actual: usize, limit: usize },
    #[error("ZIP entry name at index {index} is {actual} bytes; the limit is {limit}")]
    EntryNameTooLong {
        index: usize,
        actual: usize,
        limit: usize,
    },
    #[error("archive contains duplicate ZIP entry name {0}")]
    DuplicateEntryName(String),
    #[error("project archive is missing required entry {0}")]
    MissingRequiredEntry(String),
    #[error("ZIP entry {name} declares {actual} uncompressed bytes; the limit is {limit}")]
    EntryTooLarge {
        name: String,
        actual: u64,
        limit: u64,
    },
    #[error("archive uncompressed size exceeds the {limit}-byte limit")]
    ArchiveTooLarge { limit: u64 },
    #[error("archive uncompressed-size total overflowed")]
    ArchiveSizeOverflow,
    #[error("manifest checksum for {entry} must be exactly 64 hexadecimal characters")]
    InvalidSha256 { entry: String },
    #[error("manifest must not contain a checksum for {0}")]
    ManifestChecksumDeclared(String),
    #[error("{scope} entry set mismatch (missing: {missing}; unexpected: {unexpected})")]
    EntrySetMismatch {
        scope: &'static str,
        missing: String,
        unexpected: String,
    },
    #[error("dataset ID {0} occurs more than once")]
    DuplicateDatasetId(Uuid),
    #[error("ZIP entry {name} expanded beyond its declared {declared} bytes")]
    EntryExpandedBeyondDeclared { name: String, declared: u64 },
    #[error("ZIP entry {name} ended after {actual} bytes; {declared} bytes were declared")]
    EntrySizeMismatch {
        name: String,
        declared: u64,
        actual: u64,
    },
    #[error("checksum mismatch for {0}")]
    ChecksumMismatch(String),
    #[error("ZIP failed: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("I/O failed while reading an archive entry: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PreflightedEntry {
    index: usize,
    uncompressed_bytes: u64,
}

/// Immutable central-directory facts collected before any entry body is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArchivePreflight {
    entries: BTreeMap<String, PreflightedEntry>,
}

impl ArchivePreflight {
    fn non_manifest_entry_names(&self) -> BTreeSet<String> {
        self.entries
            .keys()
            .filter(|name| name.as_str() != MANIFEST_ENTRY_NAME)
            .cloned()
            .collect()
    }

    fn entry(&self, name: &str) -> Result<PreflightedEntry, ArchiveIntegrityError> {
        self.entries
            .get(name)
            .copied()
            .ok_or_else(|| ArchiveIntegrityError::MissingRequiredEntry(name.to_owned()))
    }
}

/// A checksum map whose hashes are canonical lowercase SHA-256 values and whose
/// keys exactly match all non-manifest ZIP entries found during preflight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidatedArchiveChecksums(BTreeMap<String, String>);

impl ValidatedArchiveChecksums {
    pub(crate) fn get(&self, entry: &str) -> Option<&str> {
        self.0.get(entry).map(String::as_str)
    }

    pub(crate) fn entry_names(&self) -> BTreeSet<String> {
        self.0.keys().cloned().collect()
    }
}

/// Inspects ZIP metadata and enforces all limits before callers allocate a
/// buffer for an entry body. Duplicate names are rejected because lookup by
/// name would otherwise make checksum verification ambiguous.
pub(crate) fn preflight_archive<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    limits: ArchiveLimits,
) -> Result<ArchivePreflight, ArchiveIntegrityError> {
    let entry_count = archive.len();
    if entry_count > limits.max_entries {
        return Err(ArchiveIntegrityError::TooManyEntries {
            actual: entry_count,
            limit: limits.max_entries,
        });
    }

    let mut entries = BTreeMap::new();
    let mut total_uncompressed_bytes = 0_u64;
    for index in 0..entry_count {
        let entry = archive.by_index(index)?;
        let name = entry.name().to_owned();
        let name_bytes = name.len();
        if name_bytes > limits.max_entry_name_bytes {
            return Err(ArchiveIntegrityError::EntryNameTooLong {
                index,
                actual: name_bytes,
                limit: limits.max_entry_name_bytes,
            });
        }
        if entries.contains_key(&name) {
            return Err(ArchiveIntegrityError::DuplicateEntryName(name));
        }

        let uncompressed_bytes = entry.size();
        if uncompressed_bytes > limits.max_entry_uncompressed_bytes {
            return Err(ArchiveIntegrityError::EntryTooLarge {
                name,
                actual: uncompressed_bytes,
                limit: limits.max_entry_uncompressed_bytes,
            });
        }
        total_uncompressed_bytes = total_uncompressed_bytes
            .checked_add(uncompressed_bytes)
            .ok_or(ArchiveIntegrityError::ArchiveSizeOverflow)?;
        if total_uncompressed_bytes > limits.max_total_uncompressed_bytes {
            return Err(ArchiveIntegrityError::ArchiveTooLarge {
                limit: limits.max_total_uncompressed_bytes,
            });
        }

        entries.insert(
            name,
            PreflightedEntry {
                index,
                uncompressed_bytes,
            },
        );
    }

    if !entries.contains_key(MANIFEST_ENTRY_NAME) {
        return Err(ArchiveIntegrityError::MissingRequiredEntry(
            MANIFEST_ENTRY_NAME.to_owned(),
        ));
    }

    Ok(ArchivePreflight { entries })
}

/// Scans raw central-directory records before `ZipArchive` normalizes them
/// into its name-keyed index. This catches duplicate names that would otherwise
/// collapse to a single public entry.
pub(crate) fn validate_raw_central_directory<R: Read + Seek>(
    reader: &mut R,
    limits: ArchiveLimits,
) -> Result<(), ArchiveIntegrityError> {
    const CENTRAL_HEADER_SIGNATURE: u32 = 0x0201_4b50;
    const END_OF_DIRECTORY_SIGNATURE: u32 = 0x0605_4b50;
    const ZIP64_END_SIGNATURE: u32 = 0x0606_4b50;
    const ZIP64_LOCATOR_SIGNATURE: u32 = 0x0706_4b50;

    let original = reader.stream_position()?;
    let length = reader.seek(std::io::SeekFrom::End(0))?;
    let search_bytes = length.min(65_557);
    reader.seek(std::io::SeekFrom::End(-(search_bytes as i64)))?;
    let mut tail = vec![0_u8; search_bytes as usize];
    reader.read_exact(&mut tail)?;
    let eocd = tail
        .windows(4)
        .enumerate()
        .rev()
        .find_map(|(index, window)| {
            if window != END_OF_DIRECTORY_SIGNATURE.to_le_bytes() || index + 22 > tail.len() {
                return None;
            }
            let comment_length = u16::from_le_bytes([tail[index + 20], tail[index + 21]]) as usize;
            (index + 22 + comment_length == tail.len()).then_some(index)
        })
        .ok_or_else(|| zip::result::ZipError::InvalidArchive("Missing EOCD".into()))?;
    if eocd + 22 > tail.len() {
        return Err(zip::result::ZipError::InvalidArchive("Truncated EOCD".into()).into());
    }
    let disk = u16::from_le_bytes([tail[eocd + 4], tail[eocd + 5]]);
    let directory_disk = u16::from_le_bytes([tail[eocd + 6], tail[eocd + 7]]);
    let entries_on_disk = u16::from_le_bytes([tail[eocd + 8], tail[eocd + 9]]) as u64;
    let entries_16 = u16::from_le_bytes([tail[eocd + 10], tail[eocd + 11]]) as u64;
    let directory_size_32 = u32::from_le_bytes([
        tail[eocd + 12],
        tail[eocd + 13],
        tail[eocd + 14],
        tail[eocd + 15],
    ]) as u64;
    let directory_offset_32 = u32::from_le_bytes([
        tail[eocd + 16],
        tail[eocd + 17],
        tail[eocd + 18],
        tail[eocd + 19],
    ]) as u64;
    let comment_length = u16::from_le_bytes([tail[eocd + 20], tail[eocd + 21]]) as usize;
    if eocd + 22 + comment_length != tail.len() {
        return Err(zip::result::ZipError::InvalidArchive(
            "Invalid end-of-central-directory comment length".into(),
        )
        .into());
    }
    if disk != 0 || directory_disk != 0 {
        return Err(zip::result::ZipError::InvalidArchive(
            "Multi-disk ZIP archives are unsupported".into(),
        )
        .into());
    }

    let requires_zip64 = entries_16 == u16::MAX as u64
        || directory_size_32 == u32::MAX as u64
        || directory_offset_32 == u32::MAX as u64;
    let (entries, directory_size, directory_offset) = if requires_zip64 {
        let absolute_eocd = length - search_bytes + eocd as u64;
        if absolute_eocd < 20 {
            return Err(
                zip::result::ZipError::InvalidArchive("Missing ZIP64 locator".into()).into(),
            );
        }
        reader.seek(std::io::SeekFrom::Start(absolute_eocd - 20))?;
        let mut locator = [0_u8; 20];
        reader.read_exact(&mut locator)?;
        if u32::from_le_bytes(locator[0..4].try_into().unwrap()) != ZIP64_LOCATOR_SIGNATURE
            || u32::from_le_bytes(locator[4..8].try_into().unwrap()) != 0
            || u32::from_le_bytes(locator[16..20].try_into().unwrap()) != 1
        {
            return Err(
                zip::result::ZipError::InvalidArchive("Invalid ZIP64 locator".into()).into(),
            );
        }
        let zip64_offset = u64::from_le_bytes(locator[8..16].try_into().unwrap());
        reader.seek(std::io::SeekFrom::Start(zip64_offset))?;
        let mut record = [0_u8; 56];
        reader.read_exact(&mut record)?;
        if u32::from_le_bytes(record[0..4].try_into().unwrap()) != ZIP64_END_SIGNATURE
            || u64::from_le_bytes(record[4..12].try_into().unwrap()) < 44
            || u32::from_le_bytes(record[16..20].try_into().unwrap()) != 0
            || u32::from_le_bytes(record[20..24].try_into().unwrap()) != 0
        {
            return Err(zip::result::ZipError::InvalidArchive(
                "Invalid ZIP64 directory record".into(),
            )
            .into());
        }
        let entries_on_disk = u64::from_le_bytes(record[24..32].try_into().unwrap());
        let entries = u64::from_le_bytes(record[32..40].try_into().unwrap());
        if entries_on_disk != entries {
            return Err(zip::result::ZipError::InvalidArchive(
                "Multi-disk ZIP64 archives are unsupported".into(),
            )
            .into());
        }
        (
            entries,
            u64::from_le_bytes(record[40..48].try_into().unwrap()),
            u64::from_le_bytes(record[48..56].try_into().unwrap()),
        )
    } else {
        if entries_on_disk != entries_16 {
            return Err(zip::result::ZipError::InvalidArchive(
                "Central-directory entry count mismatch".into(),
            )
            .into());
        }
        (entries_16, directory_size_32, directory_offset_32)
    };
    let entries = usize::try_from(entries).map_err(|_| ArchiveIntegrityError::TooManyEntries {
        actual: usize::MAX,
        limit: limits.max_entries,
    })?;
    let directory_end = directory_offset
        .checked_add(directory_size)
        .ok_or_else(|| {
            zip::result::ZipError::InvalidArchive("Central directory offset overflow".into())
        })?;
    let absolute_eocd = length - search_bytes + eocd as u64;
    let expected_directory_end = if requires_zip64 {
        if absolute_eocd < 20 {
            return Err(
                zip::result::ZipError::InvalidArchive("Missing ZIP64 locator".into()).into(),
            );
        }
        reader.seek(std::io::SeekFrom::Start(absolute_eocd - 12))?;
        let mut locator_offset = [0_u8; 8];
        reader.read_exact(&mut locator_offset)?;
        u64::from_le_bytes(locator_offset)
    } else {
        absolute_eocd
    };
    if directory_end != expected_directory_end || directory_end > length {
        return Err(zip::result::ZipError::InvalidArchive(
            "Central directory does not end at its declared end record".into(),
        )
        .into());
    }
    if entries > limits.max_entries {
        return Err(ArchiveIntegrityError::TooManyEntries {
            actual: entries,
            limit: limits.max_entries,
        });
    }

    reader.seek(std::io::SeekFrom::Start(directory_offset))?;
    let mut names = BTreeSet::new();
    for index in 0..entries {
        let mut signature = [0_u8; 4];
        reader.read_exact(&mut signature)?;
        let signature = u32::from_le_bytes(signature);
        if signature == ZIP64_END_SIGNATURE || signature == ZIP64_LOCATOR_SIGNATURE {
            return Err(zip::result::ZipError::InvalidArchive(
                "Unexpected ZIP64 record in central directory".into(),
            )
            .into());
        }
        if signature != CENTRAL_HEADER_SIGNATURE {
            return Err(zip::result::ZipError::InvalidArchive(
                "Invalid central directory header".into(),
            )
            .into());
        }
        let mut fixed = [0_u8; 42];
        reader.read_exact(&mut fixed)?;
        let name_length = u16::from_le_bytes([fixed[24], fixed[25]]) as usize;
        let extra_length = u16::from_le_bytes([fixed[26], fixed[27]]) as i64;
        let comment_length = u16::from_le_bytes([fixed[28], fixed[29]]) as i64;
        if name_length > limits.max_entry_name_bytes {
            return Err(ArchiveIntegrityError::EntryNameTooLong {
                index,
                actual: name_length,
                limit: limits.max_entry_name_bytes,
            });
        }
        let mut name = vec![0_u8; name_length];
        reader.read_exact(&mut name)?;
        let name = String::from_utf8_lossy(&name).into_owned();
        if !names.insert(name.clone()) {
            return Err(ArchiveIntegrityError::DuplicateEntryName(name));
        }
        reader.seek(std::io::SeekFrom::Current(extra_length + comment_length))?;
    }
    if reader.stream_position()? != directory_end {
        return Err(zip::result::ZipError::InvalidArchive(
            "Central-directory entry records do not consume the declared directory size".into(),
        )
        .into());
    }
    reader.seek(std::io::SeekFrom::Start(original))?;
    Ok(())
}

/// Reads a preflighted entry with an additional purpose-specific allocation
/// ceiling. This is intended for small metadata entries such as the manifest;
/// scientific entries should be streamed or decoded from a bounded reader.
pub(crate) fn read_preflighted_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    preflight: &ArchivePreflight,
    name: &str,
    allocation_limit: u64,
) -> Result<Vec<u8>, ArchiveIntegrityError> {
    let expected = preflight.entry(name)?;
    if expected.uncompressed_bytes > allocation_limit {
        return Err(ArchiveIntegrityError::EntryTooLarge {
            name: name.to_owned(),
            actual: expected.uncompressed_bytes,
            limit: allocation_limit,
        });
    }
    let capacity = usize::try_from(expected.uncompressed_bytes).map_err(|_| {
        ArchiveIntegrityError::EntryTooLarge {
            name: name.to_owned(),
            actual: expected.uncompressed_bytes,
            limit: usize::MAX as u64,
        }
    })?;
    let mut body = Vec::with_capacity(capacity);
    let mut entry = archive.by_index(expected.index)?;
    let mut bounded = (&mut entry).take(expected.uncompressed_bytes.saturating_add(1));
    bounded.read_to_end(&mut body)?;
    validate_observed_size(name, expected.uncompressed_bytes, body.len() as u64)?;
    Ok(body)
}

/// Validates every checksum string and requires the checksum keys to match the
/// archive's exact non-manifest entry set. The manifest is intentionally not
/// self-checksummed because its checksum map is contained inside that entry.
pub(crate) fn validate_manifest_checksums(
    preflight: &ArchivePreflight,
    checksums: &BTreeMap<String, String>,
) -> Result<ValidatedArchiveChecksums, ArchiveIntegrityError> {
    if checksums.contains_key(MANIFEST_ENTRY_NAME) {
        return Err(ArchiveIntegrityError::ManifestChecksumDeclared(
            MANIFEST_ENTRY_NAME.to_owned(),
        ));
    }

    let mut normalized = BTreeMap::new();
    for (entry, checksum) in checksums {
        let checksum = normalize_sha256(entry, checksum)?;
        normalized.insert(entry.clone(), checksum);
    }

    let actual = preflight.non_manifest_entry_names();
    let declared = normalized.keys().cloned().collect();
    ensure_exact_entry_set("manifest checksum", &actual, &declared)?;
    Ok(ValidatedArchiveChecksums(normalized))
}

/// Streams every non-manifest entry through SHA-256 without allocating its
/// full uncompressed body. This can run before the project document is decoded,
/// including for archives written by a newer schema version.
pub(crate) fn verify_archive_checksums<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    preflight: &ArchivePreflight,
    checksums: &ValidatedArchiveChecksums,
) -> Result<(), ArchiveIntegrityError> {
    for (name, expected_checksum) in &checksums.0 {
        let expected_entry = preflight.entry(name)?;
        let mut entry = archive.by_index(expected_entry.index)?;
        let mut digest = Sha256::new();
        let mut observed = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let bytes_read = entry.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            observed = observed.checked_add(bytes_read as u64).ok_or(
                ArchiveIntegrityError::EntryExpandedBeyondDeclared {
                    name: name.clone(),
                    declared: expected_entry.uncompressed_bytes,
                },
            )?;
            if observed > expected_entry.uncompressed_bytes {
                return Err(ArchiveIntegrityError::EntryExpandedBeyondDeclared {
                    name: name.clone(),
                    declared: expected_entry.uncompressed_bytes,
                });
            }
            digest.update(&buffer[..bytes_read]);
        }
        validate_observed_size(name, expected_entry.uncompressed_bytes, observed)?;
        let actual_checksum = format!("{:x}", digest.finalize());
        if actual_checksum != *expected_checksum {
            return Err(ArchiveIntegrityError::ChecksumMismatch(name.clone()));
        }
    }
    Ok(())
}

/// Builds the exact current-schema non-manifest entry set. UUID formatting is
/// canonical and duplicate dataset IDs fail instead of silently collapsing to
/// the same `data/<uuid>.arrow` path.
pub(crate) fn expected_project_entries<I>(
    dataset_ids: I,
) -> Result<BTreeSet<String>, ArchiveIntegrityError>
where
    I: IntoIterator<Item = Uuid>,
{
    expected_project_entries_with_additional(dataset_ids, std::iter::empty::<String>())
}

/// Builds the exact non-manifest entry set for schema-6 archives that carry
/// additive scientific sidecars. Additional entries are already validated by
/// their owning typed contract; this seam still rejects collisions with
/// `project.json`, dataset entries, or another declared sidecar.
pub(crate) fn expected_project_entries_with_additional<I, J>(
    dataset_ids: I,
    additional_entries: J,
) -> Result<BTreeSet<String>, ArchiveIntegrityError>
where
    I: IntoIterator<Item = Uuid>,
    J: IntoIterator<Item = String>,
{
    let mut expected = BTreeSet::from([PROJECT_ENTRY_NAME.to_owned()]);
    let mut unique_ids = BTreeSet::new();
    for dataset_id in dataset_ids {
        if !unique_ids.insert(dataset_id) {
            return Err(ArchiveIntegrityError::DuplicateDatasetId(dataset_id));
        }
        expected.insert(format!("data/{dataset_id}.arrow"));
    }
    for entry_name in additional_entries {
        if entry_name == MANIFEST_ENTRY_NAME || !expected.insert(entry_name.clone()) {
            return Err(ArchiveIntegrityError::DuplicateEntryName(entry_name));
        }
    }
    Ok(expected)
}

/// Requires a decoded current-schema document's expected entry set to match
/// the already validated manifest checksum keys exactly.
pub(crate) fn validate_expected_project_entries(
    checksums: &ValidatedArchiveChecksums,
    expected: &BTreeSet<String>,
) -> Result<(), ArchiveIntegrityError> {
    ensure_exact_entry_set("project document", &checksums.entry_names(), expected)
}

fn normalize_sha256(entry: &str, checksum: &str) -> Result<String, ArchiveIntegrityError> {
    if checksum.len() != 64 || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ArchiveIntegrityError::InvalidSha256 {
            entry: entry.to_owned(),
        });
    }
    Ok(checksum.to_ascii_lowercase())
}

fn validate_observed_size(
    name: &str,
    declared: u64,
    actual: u64,
) -> Result<(), ArchiveIntegrityError> {
    if actual > declared {
        return Err(ArchiveIntegrityError::EntryExpandedBeyondDeclared {
            name: name.to_owned(),
            declared,
        });
    }
    if actual != declared {
        return Err(ArchiveIntegrityError::EntrySizeMismatch {
            name: name.to_owned(),
            declared,
            actual,
        });
    }
    Ok(())
}

fn ensure_exact_entry_set(
    scope: &'static str,
    actual: &BTreeSet<String>,
    expected: &BTreeSet<String>,
) -> Result<(), ArchiveIntegrityError> {
    let missing = expected.difference(actual).cloned().collect::<Vec<_>>();
    let unexpected = actual.difference(expected).cloned().collect::<Vec<_>>();
    if missing.is_empty() && unexpected.is_empty() {
        return Ok(());
    }
    Err(ArchiveIntegrityError::EntrySetMismatch {
        scope,
        missing: display_names(&missing),
        unexpected: display_names(&unexpected),
    })
}

fn display_names(names: &[String]) -> String {
    if names.is_empty() {
        "<none>".to_owned()
    } else {
        names.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::{ZipWriter, write::SimpleFileOptions};

    fn archive(entries: &[(&str, &[u8])]) -> ZipArchive<Cursor<Vec<u8>>> {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut bytes);
            for (name, body) in entries {
                writer
                    .start_file(*name, SimpleFileOptions::default())
                    .unwrap();
                writer.write_all(body).unwrap();
            }
            writer.finish().unwrap();
        }
        bytes.set_position(0);
        ZipArchive::new(bytes).unwrap()
    }

    fn digest(body: &[u8]) -> String {
        format!("{:x}", Sha256::digest(body))
    }

    #[test]
    fn preflight_rejects_duplicate_names() {
        // ZipWriter correctly rejects duplicate names, so construct a valid
        // archive with two same-length placeholder names and rewrite only the
        // filename bytes in its local and central directory records.
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut bytes);
            for (name, body) in [
                (MANIFEST_ENTRY_NAME, b"{}" as &[u8]),
                ("project0json", b"first" as &[u8]),
                ("project1json", b"second" as &[u8]),
            ] {
                writer
                    .start_file(name, SimpleFileOptions::default())
                    .unwrap();
                writer.write_all(body).unwrap();
            }
            writer.finish().unwrap();
        }
        let mut bytes = bytes.into_inner();
        for placeholder in [b"project0json", b"project1json"] {
            let mut offset = 0;
            while let Some(index) = bytes[offset..]
                .windows(placeholder.len())
                .position(|window| window == placeholder)
            {
                let start = offset + index;
                bytes[start..start + placeholder.len()]
                    .copy_from_slice(PROJECT_ENTRY_NAME.as_bytes());
                offset = start + placeholder.len();
            }
        }
        let archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut raw = archive.into_inner();
        let result = validate_raw_central_directory(&mut raw, DEFAULT_ARCHIVE_LIMITS);
        let mut archive = ZipArchive::new(raw).unwrap();
        // The dependency's public name index collapses duplicates, which is
        // why the raw check is a required first stage.
        assert_eq!(archive.len(), 2);
        let _ = preflight_archive(&mut archive, DEFAULT_ARCHIVE_LIMITS).unwrap();
        assert!(
            matches!(
                result,
                Err(ArchiveIntegrityError::DuplicateEntryName(ref name))
                    if name == PROJECT_ENTRY_NAME
            ),
            "unexpected duplicate-name preflight result: {result:?}"
        );
    }

    #[test]
    fn preflight_applies_count_and_size_limits() {
        let mut count_archive = archive(&[
            (MANIFEST_ENTRY_NAME, b"{}" as &[u8]),
            (PROJECT_ENTRY_NAME, b"data" as &[u8]),
        ]);
        let count_limits = ArchiveLimits {
            max_entries: 1,
            ..DEFAULT_ARCHIVE_LIMITS
        };
        assert!(matches!(
            preflight_archive(&mut count_archive, count_limits),
            Err(ArchiveIntegrityError::TooManyEntries {
                actual: 2,
                limit: 1
            })
        ));

        let mut size_archive = archive(&[
            (MANIFEST_ENTRY_NAME, b"{}" as &[u8]),
            (PROJECT_ENTRY_NAME, b"data" as &[u8]),
        ]);
        let size_limits = ArchiveLimits {
            max_entry_uncompressed_bytes: 3,
            ..DEFAULT_ARCHIVE_LIMITS
        };
        assert!(matches!(
            preflight_archive(&mut size_archive, size_limits),
            Err(ArchiveIntegrityError::EntryTooLarge { name, actual: 4, limit: 3 })
                if name == PROJECT_ENTRY_NAME
        ));
    }

    #[test]
    fn checksums_are_strictly_hex_and_case_insensitive() {
        let body = b"project";
        let mut archive = archive(&[(MANIFEST_ENTRY_NAME, b"{}"), (PROJECT_ENTRY_NAME, body)]);
        let preflight = preflight_archive(&mut archive, DEFAULT_ARCHIVE_LIMITS).unwrap();
        let uppercase = digest(body).to_ascii_uppercase();
        let checksums = BTreeMap::from([(PROJECT_ENTRY_NAME.to_owned(), uppercase)]);
        let validated = validate_manifest_checksums(&preflight, &checksums).unwrap();
        assert_eq!(
            validated.get(PROJECT_ENTRY_NAME),
            Some(digest(body).as_str())
        );
        verify_archive_checksums(&mut archive, &preflight, &validated).unwrap();

        for malformed in [
            "a".repeat(63),
            "a".repeat(65),
            format!("{}g", "a".repeat(63)),
        ] {
            let checksums = BTreeMap::from([(PROJECT_ENTRY_NAME.to_owned(), malformed)]);
            assert!(matches!(
                validate_manifest_checksums(&preflight, &checksums),
                Err(ArchiveIntegrityError::InvalidSha256 { entry })
                    if entry == PROJECT_ENTRY_NAME
            ));
        }
    }

    #[test]
    fn manifest_keys_must_exactly_match_non_manifest_zip_entries() {
        let mut archive = archive(&[
            (MANIFEST_ENTRY_NAME, b"{}"),
            (PROJECT_ENTRY_NAME, b"project"),
            ("data/unexpected.arrow", b"dataset"),
        ]);
        let preflight = preflight_archive(&mut archive, DEFAULT_ARCHIVE_LIMITS).unwrap();
        let checksums = BTreeMap::from([
            (PROJECT_ENTRY_NAME.to_owned(), digest(b"project")),
            ("data/missing.arrow".to_owned(), digest(b"missing")),
        ]);
        assert!(matches!(
            validate_manifest_checksums(&preflight, &checksums),
            Err(ArchiveIntegrityError::EntrySetMismatch {
                scope: "manifest checksum",
                missing,
                unexpected,
            }) if missing == "data/missing.arrow" && unexpected == "data/unexpected.arrow"
        ));
    }

    #[test]
    fn expected_entries_use_unique_ids_and_canonical_arrow_paths() {
        let first = Uuid::parse_str("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa").unwrap();
        let second = Uuid::parse_str("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb").unwrap();
        let expected = expected_project_entries([first, second]).unwrap();
        assert_eq!(
            expected,
            BTreeSet::from([
                PROJECT_ENTRY_NAME.to_owned(),
                format!("data/{first}.arrow"),
                format!("data/{second}.arrow"),
            ])
        );
        assert!(matches!(
            expected_project_entries([first, first]),
            Err(ArchiveIntegrityError::DuplicateDatasetId(duplicate)) if duplicate == first
        ));
    }

    #[test]
    fn decoded_project_entries_must_match_manifest_exactly() {
        let body = b"project";
        let mut archive = archive(&[(MANIFEST_ENTRY_NAME, b"{}"), (PROJECT_ENTRY_NAME, body)]);
        let preflight = preflight_archive(&mut archive, DEFAULT_ARCHIVE_LIMITS).unwrap();
        let checksums = BTreeMap::from([(PROJECT_ENTRY_NAME.to_owned(), digest(body))]);
        let validated = validate_manifest_checksums(&preflight, &checksums).unwrap();
        let dataset_id = Uuid::parse_str("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa").unwrap();
        let expected = expected_project_entries([dataset_id]).unwrap();
        assert!(matches!(
            validate_expected_project_entries(&validated, &expected),
            Err(ArchiveIntegrityError::EntrySetMismatch {
                scope: "project document",
                missing,
                unexpected,
            }) if missing == format!("data/{dataset_id}.arrow") && unexpected == "<none>"
        ));
    }
}
