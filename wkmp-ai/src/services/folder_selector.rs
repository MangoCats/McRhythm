//! Folder Selection for Import Workflow
//!
//! **Traceability:** [REQ-SPEC032-005] Folder Selection (Step 2)
//!
//! Validates folder selection for Stage One import (root folder or subfolders only).

use std::path::{Path, PathBuf};
use wkmp_common::{Error, Result};

/// Folder selection validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionResult {
    /// Valid root folder selection
    ValidRoot(PathBuf),
    /// Valid subfolder within root
    ValidSubfolder(PathBuf),
    /// External folder (outside root) - Stage Two feature
    ExternalFolder(PathBuf),
    /// Folder does not exist
    NotFound(PathBuf),
    /// Folder exists but is not readable
    NotReadable(PathBuf),
    /// Symlink loop detected
    SymlinkLoop(PathBuf),
}

/// Folder Selector
///
/// **Traceability:** [REQ-SPEC032-005] (Step 2: Folder Selection)
pub struct FolderSelector {
    root_folder: PathBuf,
}

impl FolderSelector {
    /// Create new folder selector with root folder
    ///
    /// **Parameters:**
    /// - `root_folder`: The WKMP root folder path (e.g., ~/Music)
    pub fn new(root_folder: PathBuf) -> Self {
        Self { root_folder }
    }

    /// Validate folder selection
    ///
    /// **Algorithm:**
    /// 1. Canonicalize paths to resolve symlinks and relative paths
    /// 2. Check if folder exists
    /// 3. Check if folder is readable
    /// 4. Check if folder is root or subfolder of root
    /// 5. Return appropriate SelectionResult
    ///
    /// **Stage One Constraint:** Only root folder or subfolders allowed
    ///
    /// **Traceability:** [REQ-SPEC032-005]
    pub fn validate_selection(&self, selected_folder: &Path) -> Result<SelectionResult> {
        tracing::debug!(
            selected = %selected_folder.display(),
            root = %self.root_folder.display(),
            "Validating folder selection"
        );

        // Check if folder exists
        if !selected_folder.exists() {
            tracing::warn!(folder = %selected_folder.display(), "Selected folder does not exist");
            return Ok(SelectionResult::NotFound(selected_folder.to_path_buf()));
        }

        // Check if it's a directory
        if !selected_folder.is_dir() {
            tracing::warn!(path = %selected_folder.display(), "Selected path is not a directory");
            return Err(Error::InvalidInput(format!(
                "Selected path is not a directory: {}",
                selected_folder.display()
            )));
        }

        // Canonicalize paths to resolve symlinks and relative paths
        // This also detects symlink loops (will fail with error)
        let canonical_selected = selected_folder.canonicalize().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::NotFound(format!(
                    "Folder not found after canonicalization: {}",
                    selected_folder.display()
                ))
            } else {
                tracing::warn!(
                    folder = %selected_folder.display(),
                    error = %e,
                    "Symlink loop or canonicalization error detected"
                );
                Error::InvalidInput(format!(
                    "Symlink loop or invalid path: {} ({})",
                    selected_folder.display(),
                    e
                ))
            }
        })?;

        let canonical_root = self.root_folder.canonicalize().map_err(|e| {
            Error::Config(format!(
                "Root folder cannot be canonicalized: {} ({})",
                self.root_folder.display(),
                e
            ))
        })?;

        tracing::debug!(
            selected_canonical = %canonical_selected.display(),
            root_canonical = %canonical_root.display(),
            "Canonicalized paths"
        );

        // Check if folder is readable (try to read directory)
        if std::fs::read_dir(&canonical_selected).is_err() {
            tracing::warn!(folder = %canonical_selected.display(), "Selected folder is not readable");
            return Ok(SelectionResult::NotReadable(
                canonical_selected.to_path_buf(),
            ));
        }

        // Check if selected folder is root or subfolder of root
        if canonical_selected == canonical_root {
            tracing::info!("Selected folder is root folder");
            Ok(SelectionResult::ValidRoot(canonical_selected))
        } else if canonical_selected.starts_with(&canonical_root) {
            tracing::info!(
                subfolder = %canonical_selected.strip_prefix(&canonical_root).unwrap().display(),
                "Selected folder is valid subfolder of root"
            );
            Ok(SelectionResult::ValidSubfolder(canonical_selected))
        } else {
            tracing::warn!(
                selected = %canonical_selected.display(),
                root = %canonical_root.display(),
                "Selected folder is outside root (Stage Two feature)"
            );
            Ok(SelectionResult::ExternalFolder(canonical_selected))
        }
    }

    /// Get default folder (root folder)
    pub fn default_folder(&self) -> &Path {
        &self.root_folder
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_validate_root_folder() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        let selector = FolderSelector::new(root.clone());
        let result = selector.validate_selection(&root).unwrap();

        assert_eq!(result, SelectionResult::ValidRoot(root.canonicalize().unwrap()));
    }

    #[test]
    fn test_validate_subfolder() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();
        let subfolder = root.join("music");

        fs::create_dir(&subfolder).unwrap();

        let selector = FolderSelector::new(root);
        let result = selector.validate_selection(&subfolder).unwrap();

        match result {
            SelectionResult::ValidSubfolder(path) => {
                assert_eq!(path, subfolder.canonicalize().unwrap());
            }
            _ => panic!("Expected ValidSubfolder, got {:?}", result),
        }
    }

    #[test]
    fn test_validate_external_folder() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        let root = temp_dir1.path().to_path_buf();
        let external = temp_dir2.path().to_path_buf();

        let selector = FolderSelector::new(root);
        let result = selector.validate_selection(&external).unwrap();

        match result {
            SelectionResult::ExternalFolder(path) => {
                assert_eq!(path, external.canonicalize().unwrap());
            }
            _ => panic!("Expected ExternalFolder, got {:?}", result),
        }
    }

    #[test]
    fn test_validate_nonexistent_folder() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();
        let nonexistent = root.join("does_not_exist");

        let selector = FolderSelector::new(root);
        let result = selector.validate_selection(&nonexistent).unwrap();

        assert_eq!(result, SelectionResult::NotFound(nonexistent));
    }

    #[test]
    fn test_default_folder() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();

        let selector = FolderSelector::new(root.clone());

        assert_eq!(selector.default_folder(), root.as_path());
    }
}
