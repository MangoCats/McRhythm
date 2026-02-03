#!/usr/bin/env python3
"""
Update pipeline.rs to implement Improvement #1: Preserve samples on album match failure
"""

import re

pipeline_file = r"wkmp-ai\src\workflow\pipeline.rs"

with open(pipeline_file, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Update album_match_fallback signature and implementation
old_signature = r'''    async fn album_match_fallback\(
        &self,
        file_path: &Path,
        reason: &str,
    \) -> Result<Vec<ProcessedPassage>> \{
        warn!\("Falling back to single-song processing: \{\}", reason\);

        self\.emit_event\(WorkflowEvent::AlbumMatchingFallback \{
            file_path: file_path\.to_string_lossy\(\)\.to_string\(\),
            reason: reason\.to_string\(\),
            timestamp: chrono::Utc::now\(\)\.timestamp\(\),
        \}\)
        \.await;

        self\.process_as_single_song\(file_path\)\.await
    \}'''

new_signature = '''    /// **[IMPROVEMENT#1]** Added decoded_audio parameter to reuse samples and avoid double-decode
    async fn album_match_fallback(
        &self,
        file_path: &Path,
        reason: &str,
        decoded_audio: Option<(Vec<f32>, u32)>,
    ) -> Result<Vec<ProcessedPassage>> {
        if decoded_audio.is_some() {
            warn!("Falling back to single-song processing with preserved samples: {}", reason);
        } else {
            warn!("Falling back to single-song processing (will re-decode): {}", reason);
        }

        self.emit_event(WorkflowEvent::AlbumMatchingFallback {
            file_path: file_path.to_string_lossy().to_string(),
            reason: reason.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        })
        .await;

        // **[IMPROVEMENT#1]** Reuse decoded samples if available
        if let Some((samples, sample_rate)) = decoded_audio {
            self.process_with_cached_audio(file_path, samples, sample_rate).await
        } else {
            self.process_as_single_song(file_path).await
        }
    }'''

content = re.sub(old_signature, new_signature, content, flags=re.MULTILINE)

# 2. Update call sites to pass decoded_audio parameter
# Most calls pass None (no samples available yet)
# The key one that needs to pass result.decoded_audio is in process_album_file

# Replace calls that don't have samples yet with None
patterns_to_replace = [
    # Failed to create MusicBrainzClient (before decode)
    (r'\.album_match_fallback\(file_path, &format!\("Failed to create MusicBrainzClient: \{\}", e\)\)',
     '.album_match_fallback(file_path, &format!("Failed to create MusicBrainzClient: {}", e), None)'),

    # Failed to create AlbumMatcher (before decode)
    (r'\.album_match_fallback\(file_path, &format!\("Failed to create AlbumMatcher: \{\}", e\)\)',
     '.album_match_fallback(file_path, &format!("Failed to create AlbumMatcher: {}", e), None)'),

    # Retry client creation failed
    (r'return self\.album_match_fallback\(file_path, "Retry client creation failed"\)\.await;',
     'return self.album_match_fallback(file_path, "Retry client creation failed", None).await;'),

    # Retry matcher creation failed
    (r'return self\.album_match_fallback\(file_path, "Retry matcher creation failed"\)\.await;',
     'return self.album_match_fallback(file_path, "Retry matcher creation failed", None).await;'),
]

for old, new in patterns_to_replace:
    content = re.sub(old, new, content)

# 3. Special handling for calls that come after album matching (these WILL have samples)
# These are in the process_album_file function after match_result

# Find and replace the section with match results
# Match percentage too low
old_low_match = r'''if self\.config\.album_match_fallback \{
                        return self
                            \.album_match_fallback\(
                                file_path,
                                &format!\(
                                    "Match percentage too low: \{:.1\}%",
                                    result\.match_percentage
                                \),
                            \)
                            \.await;'''

new_low_match = '''if self.config.album_match_fallback {
                        return self
                            .album_match_fallback(
                                file_path,
                                &format!(
                                    "Match percentage too low: {:.1}%",
                                    result.match_percentage
                                ),
                                result.decoded_audio, // **[IMPROVEMENT#1]** Pass preserved samples
                            )
                            .await;'''

content = re.sub(old_low_match, new_low_match, content, flags=re.MULTILINE)

# Generic fallback calls after matching
content = re.sub(
    r'self\.album_match_fallback\(file_path, &reason\)\.await',
    'self.album_match_fallback(file_path, &reason, result.decoded_audio).await',
    content
)

content = re.sub(
    r'self\.album_match_fallback\(file_path, &e\.to_string\(\)\)\.await',
    'self.album_match_fallback(file_path, &e.to_string(), None).await',
    content
)

# Write back
with open(pipeline_file, 'w', encoding='utf-8', newline='\n') as f:
    f.write(content)

print("Successfully updated pipeline.rs")
