//! Deduplication using SimHash for fuzzy matching of similar log entries

use crate::models::LogEntry;
use once_cell::sync::Lazy;
use std::collections::HashMap;

// Static compiled regex patterns for performance
static TIMESTAMP_RE: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?")
        .expect("Invalid timestamp regex")
});

static LINENO_RE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r":\d+\b").expect("Invalid line number regex"));

static UUID_RE: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
        .expect("Invalid UUID regex")
});

static HEXADDR_RE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"0x[0-9a-fA-F]+").expect("Invalid hex address regex"));

/// SimHash-based deduplicator for fuzzy matching of log entries
pub struct Deduplicator {
    /// Store hashes of seen entries (signature -> simhash)
    signatures: HashMap<String, u64>,
    /// Threshold for considering two entries similar (Hamming distance)
    similarity_threshold: u32,
}

impl Deduplicator {
    pub fn new() -> Self {
        Self {
            signatures: HashMap::new(),
            similarity_threshold: 3, // Allow up to 3 bits difference
        }
    }

    /// Compute SimHash for a string
    fn compute_simhash(text: &str) -> u64 {
        let mut hash_vector = [0i32; 64];

        // Simple word-based hashing
        let words: Vec<&str> = text.split_whitespace().collect();

        for word in words {
            // Compute hash for word
            let word_hash = Self::hash_word(word);

            // Update hash vector
            for (i, hv) in hash_vector.iter_mut().enumerate() {
                let bit = ((word_hash >> i) & 1) as i32;
                if bit == 1 {
                    *hv += 1;
                } else {
                    *hv -= 1;
                }
            }
        }

        // Build final hash from vector
        let mut simhash: u64 = 0;
        for (i, &val) in hash_vector.iter().enumerate() {
            if val > 0 {
                simhash |= 1 << i;
            }
        }

        simhash
    }

    /// Simple hash function for words
    fn hash_word(word: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        word.hash(&mut hasher);
        hasher.finish()
    }

    /// Compute Hamming distance between two hashes
    fn hamming_distance(a: u64, b: u64) -> u32 {
        (a ^ b).count_ones()
    }

    /// Normalize content for deduplication (remove variable parts like timestamps, line numbers)
    fn normalize_content(content: &str) -> String {
        let mut normalized = content.to_string();

        // Remove timestamps
        normalized = TIMESTAMP_RE
            .replace_all(&normalized, "[TIMESTAMP]")
            .to_string();

        // Remove line numbers from stack traces (e.g., ":123" or ":123)")
        normalized = LINENO_RE.replace_all(&normalized, ": [LINE]").to_string();

        // Remove UUIDs
        normalized = UUID_RE.replace_all(&normalized, "[UUID]").to_string();

        // Remove hex addresses
        normalized = HEXADDR_RE.replace_all(&normalized, "[ADDR]").to_string();

        normalized
    }

    /// Check if an entry is a duplicate of a previously seen entry
    /// Returns the signature of the matching entry if found
    pub fn find_duplicate(&self, entry: &LogEntry) -> Option<String> {
        let normalized = Self::normalize_content(&entry.raw_content);
        let hash = Self::compute_simhash(&normalized);

        // Check against all known signatures
        for (signature, known_hash) in &self.signatures {
            let distance = Self::hamming_distance(hash, *known_hash);
            if distance <= self.similarity_threshold {
                return Some(signature.clone());
            }
        }

        None
    }

    /// Add an entry to the deduplication index
    pub fn add_signature(&mut self, entry: &LogEntry, signature: String) {
        let normalized = Self::normalize_content(&entry.raw_content);
        let hash = Self::compute_simhash(&normalized);
        self.signatures.insert(signature, hash);
    }

    /// Get the number of unique signatures stored
    pub fn signature_count(&self) -> usize {
        self.signatures.len()
    }
}

impl Default for Deduplicator {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a signature from content (simple hash for now)
pub fn generate_signature(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let normalized = Deduplicator::normalize_content_static(content);

    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

impl Deduplicator {
    /// Static version of normalize_content for use in generate_signature
    fn normalize_content_static(content: &str) -> String {
        let mut normalized = content.to_string();

        // Use the same static regexes for consistency
        normalized = TIMESTAMP_RE
            .replace_all(&normalized, "[TIMESTAMP]")
            .to_string();

        normalized = LINENO_RE.replace_all(&normalized, ": [LINE]").to_string();

        normalized = UUID_RE.replace_all(&normalized, "[UUID]").to_string();

        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simhash_similarity() {
        let text1 = "ERROR: Connection refused at Database.java:45";
        let text2 = "ERROR: Connection refused at Database.java:47";
        let text3 = "INFO: Server started successfully";

        let hash1 = Deduplicator::compute_simhash(text1);
        let hash2 = Deduplicator::compute_simhash(text2);
        let hash3 = Deduplicator::compute_simhash(text3);

        // Similar texts should have small Hamming distance
        let dist12 = Deduplicator::hamming_distance(hash1, hash2);
        let dist13 = Deduplicator::hamming_distance(hash1, hash3);

        assert!(dist12 < dist13, "Similar texts should be closer");
    }

    #[test]
    fn test_normalize_content() {
        let content = "2024-01-15T10:30:00Z ERROR: Failed at File.java:123";
        let normalized = Deduplicator::normalize_content_static(content);

        assert!(!normalized.contains("2024-01-15"));
        assert!(!normalized.contains(":123"));
        assert!(normalized.contains("[TIMESTAMP]"));
        assert!(normalized.contains(": [LINE]"));
    }

    #[test]
    fn test_deduplicate_similar_entries() {
        let mut dedup = Deduplicator::new();

        let entry1 = LogEntry::new(
            uuid::Uuid::new_v4(),
            1,
            chrono::Utc::now(),
            "ERROR: NullPointer at Controller.java:45".to_string(),
        );

        let entry2 = LogEntry::new(
            uuid::Uuid::new_v4(),
            2,
            chrono::Utc::now(),
            "ERROR: NullPointer at Controller.java:48".to_string(),
        );

        // Add first entry
        let sig1 = generate_signature(&entry1.raw_content);
        dedup.add_signature(&entry1, sig1.clone());

        // Second entry should be detected as duplicate
        let duplicate = dedup.find_duplicate(&entry2);
        assert!(duplicate.is_some(), "Similar entries should match");
    }
}
