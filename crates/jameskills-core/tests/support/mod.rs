use jameskills_core::domain::{
    BundleEntry, EntryKind, PortablePath, ValidatedInventory, validate_bundle_inventory,
};
use std::collections::BTreeMap;

/// Builds a valid in-memory bundle: inventory entries plus the raw bytes the
/// canonical digest hashes. Fixtures are valid by construction; an invalid
/// fixture panics so the failing test points at the helper, not the unit.
pub fn fixture_bundle(
    files: &[(&str, &[u8])],
) -> (ValidatedInventory, BTreeMap<PortablePath, Vec<u8>>) {
    let mut entries = Vec::with_capacity(files.len());
    let mut bytes = BTreeMap::new();
    for (path, content) in files {
        let portable = PortablePath::new((*path).to_owned()).expect("fixture path is portable");
        entries.push(BundleEntry::new(
            portable.clone(),
            EntryKind::RegularFile,
            content.len() as u64,
            content.len() as u64,
        ));
        bytes.insert(portable, (*content).to_vec());
    }
    let inventory = validate_bundle_inventory(&entries).expect("fixture bundle is valid");
    (inventory, bytes)
}
