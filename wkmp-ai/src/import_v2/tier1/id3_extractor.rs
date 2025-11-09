// PLAN023 Tier 1: ID3 Metadata Extractor (Placeholder)
// TODO: Complete lofty integration when API is clarified

use crate::import_v2::types::{
    ExtractionSource, ExtractorResult, ImportError, ImportResult, MetadataBundle,
};
use std::path::Path;

pub struct ID3Extractor {
    confidence: f64,
}

impl Default for ID3Extractor {
    fn default() -> Self {
        Self { confidence: 0.5 }
    }
}

impl ID3Extractor {
    pub fn extract(&self, file_path: &Path) -> ImportResult<ExtractorResult<MetadataBundle>> {
        if !file_path.exists() {
            return Err(ImportError::ExtractionFailed(format!(
                "File not found: {}",
                file_path.display()
            )));
        }
        Ok(ExtractorResult {
            data: MetadataBundle::default(),
            confidence: self.confidence,
            source: ExtractionSource::ID3Metadata,
        })
    }

    pub fn extract_genre(&self, file_path: &Path) -> ImportResult<Option<String>> {
        if !file_path.exists() {
            return Err(ImportError::ExtractionFailed(format!(
                "File not found: {}",
                file_path.display()
            )));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extractor_default_confidence() {
        let extractor = ID3Extractor::default();
        assert_eq!(extractor.confidence, 0.5);
    }

    #[test]
    fn test_extract_nonexistent_file() {
        let extractor = ID3Extractor::default();
        let result = extractor.extract(Path::new("nonexistent_file.mp3"));
        assert!(result.is_err());
    }
}
