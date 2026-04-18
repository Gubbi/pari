//! Storage and executor I/O primitive errors.

use pari_macros::primitive_with_fields;

/// A storage existence check could not be completed for the requested asset path.
#[primitive_with_fields]
pub struct ExistenceCheckFailed {
    asset_path: String,
    operation: String,
}

/// A storage read failed while trying to fetch an asset.
#[primitive_with_fields]
pub struct AssetReadFailed {
    asset_path: String,
    operation: String,
}

/// A storage write failed while trying to persist an asset.
#[primitive_with_fields]
pub struct AssetWriteFailed {
    asset_path: String,
    operation: String,
}

/// A storage delete failed while trying to remove an asset.
#[primitive_with_fields]
pub struct AssetDeleteFailed {
    asset_path: String,
}

/// The substrate root directory could not be created or initialized.
#[primitive_with_fields]
pub struct RootDirectoryCreationFailed {
    root: String,
}

/// Traversal of stale substrate artifacts failed before cleanup could complete.
#[primitive_with_fields]
pub struct StaleCleanupTraversalFailed {
    path: String,
}

/// A stale substrate artifact was identified but could not be deleted.
#[primitive_with_fields]
pub struct StaleCleanupDeletionFailed {
    path: String,
}

/// A required directory could not be read during substrate or executor work.
#[primitive_with_fields]
pub struct DirectoryReadFailed {
    path: String,
}

/// A directory entry could not be enumerated or inspected during traversal.
#[primitive_with_fields]
pub struct DirectoryEntryReadFailed {
    path: String,
}

/// A file could not be read from the backing storage.
#[primitive_with_fields]
pub struct FileReadFailed {
    asset_path: String,
}

/// A parent directory required for a write could not be created.
#[primitive_with_fields]
pub struct ParentDirectoryCreationFailed {
    directory_path: String,
}

/// A file could not be written to the backing storage.
#[primitive_with_fields]
pub struct FileWriteFailed {
    asset_path: String,
}

/// A file could not be deleted from the backing storage.
#[primitive_with_fields]
pub struct FileDeleteFailed {
    asset_path: String,
}

/// The executor received an operation kind that it does not implement.
#[primitive_with_fields]
pub struct UnsupportedExecutorOperation {
    operation: String,
    asset_path: String,
}

/// The backing storage rejected access to the requested asset path.
#[primitive_with_fields]
pub struct PathPermissionDenied {
    asset_path: String,
    operation: String,
}

/// A requested asset was not present in the backing storage.
#[primitive_with_fields]
pub struct MissingAsset {
    asset_path: String,
}
