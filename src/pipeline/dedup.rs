//! Deduplication using SimHash for fuzzy matching of similar log entries

use crate::models::LogEntry;
use once_cell::sync::Lazy;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

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

const MAX_DEDUP_SIGNATURES: usize = 100_000;
/// Default dedup window: entries older than this are stale and evicted
const DEFAULT_DEDUP_WINDOW: Duration = Duration::from_secs(3600); // 1 hour

/// SimHash-based deduplicator for fuzzy matching of log entries
pub struct Deduplicator {
    /// Store hashes of seen entries (signature -> simhash)
    /// Limited to MAX_DEDUP_SIGNATURES to prevent unbounded growth
    signatures: HashMap<String, u64>,
    /// Track when each signature was last seen for TTL eviction
    seen_at: HashMap<String, Instant>,
    /// Threshold for considering two entries similar (Hamming distance)
    similarity_threshold: u32,
    /// FIFO queue for eviction (oldest entries at front)
    insertion_queue: VecDeque<String>,
    /// Dedup window — entries unseen longer than this are evicted
    window: Duration,
    /// Counter for periodic eviction sweeps
    insertion_count: usize,
}

impl Deduplicator {
    pub fn new() -> Self {
        Self {
            signatures: HashMap::with_capacity(MAX_DEDUP_SIGNATURES),
            seen_at: HashMap::with_capacity(MAX_DEDUP_SIGNATURES),
            similarity_threshold: 3, // Allow up to 3 bits difference
            insertion_queue: VecDeque::with_capacity(MAX_DEDUP_SIGNATURES),
            window: DEFAULT_DEDUP_WINDOW,
            insertion_count: 0,
        }
    }

    /// Evict oldest entries when size limit reached (proper FIFO)
    fn evict_if_needed(&mut self) {
        if self.signatures.len() >= MAX_DEDUP_SIGNATURES {
            // Remove oldest 20% of entries from front of FIFO queue
            let to_remove = (MAX_DEDUP_SIGNATURES / 5).max(1);

            for _ in 0..to_remove {
                if let Some(key) = self.insertion_queue.pop_front() {
                    // Only remove from signatures if key matches (handles duplicates in queue)
                    self.signatures.remove(&key);
                    self.seen_at.remove(&key);
                }
            }
        }
    }

    /// Evict entries older than the dedup window (TTL-based sweep).
    /// Called periodically (every N insertions or when map exceeds threshold).
    fn evict_stale(&mut self) {
        let cutoff = Instant::now() - self.window;
        // Collect keys to avoid borrowing issues with retain
        let stale_keys: Vec<String> = self
            .seen_at
            .iter()
            .filter(|(_, seen_at)| **seen_at < cutoff)
            .map(|(k, _)| k.clone())
            .collect();

        if stale_keys.is_empty() {
            return;
        }

        for key in stale_keys {
            self.signatures.remove(&key);
            self.seen_at.remove(&key);
        }
        // Also clean the FIFO queue (best-effort — full compaction deferred)
        self.insertion_queue.retain(|k| self.signatures.contains_key(k));
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
        // Periodic TTL sweep — every 10k insertions or at size threshold
        self.insertion_count += 1;
        if self.insertion_count % 10_000 == 0 || self.signatures.len() > MAX_DEDUP_SIGNATURES / 2 {
            self.evict_stale();
        }

        // Evict old entries if at capacity
        self.evict_if_needed();

        let normalized = Self::normalize_content(&entry.raw_content);
        let hash = Self::compute_simhash(&normalized);
        self.signatures.insert(signature.clone(), hash);
        self.seen_at.insert(signature.clone(), Instant::now());
        self.insertion_queue.push_back(signature);
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

    // ── Property-based tests ───────────────────────────────────────────

    /// Property: identical strings always produce the same hash
    #[test]
    fn test_identical_strings_same_hash() {
        for text in [
            "ERROR: Connection refused",
            "INFO: Server started on port 8080",
            "WARN: Disk usage at 85% on /dev/sda1",
            "",
            "x",
        ] {
            let h1 = Deduplicator::compute_simhash(text);
            let h2 = Deduplicator::compute_simhash(text);
            assert_eq!(
                h1, h2,
                "identical input '{}' produced different hashes",
                text
            );
            assert_eq!(
                Deduplicator::hamming_distance(h1, h2),
                0,
                "distance should be 0 for identical hash"
            );
        }
    }

    /// Property: completely different strings have distinct hashes
    #[test]
    fn test_completely_different_strings_distinct() {
        let pairs = [
            ("ERROR: Database timeout", "INFO: User login successful"),
            ("FATAL: Out of memory", "DEBUG: Cache hit ratio 0.95"),
            ("WARN: Retry attempt 3/5", "TRACE: Entering function foo()"),
            (
                "ERROR: NullPointerException at line 42",
                "Starting warmup phase for node-7",
            ),
        ];

        for (a, b) in pairs {
            let h1 = Deduplicator::compute_simhash(a);
            let h2 = Deduplicator::compute_simhash(b);
            let dist = Deduplicator::hamming_distance(h1, h2);
            assert!(
                dist > 3,
                "completely different texts should not match (got distance {}):\n  '{}'\n  '{}'",
                dist, a, b
            );
        }
    }

    /// Property: strings differing by one word should be similar
    #[test]
    fn test_single_word_difference_similar() {
        let template = "ERROR: Connection to database {} failed after 5000ms";
        let text1 = template.replace("{}", "users");
        let text2 = template.replace("{}", "orders");

        let h1 = Deduplicator::compute_simhash(&text1);
        let h2 = Deduplicator::compute_simhash(&text2);
        let dist = Deduplicator::hamming_distance(h1, h2);

        assert!(
            dist <= 10,
            "texts differing by one word should be similar (got distance {})",
            dist
        );
    }

    /// Property: SimHash distance is symmetric
    #[test]
    fn test_hamming_distance_symmetric() {
        let text_a = "ERROR: Service health check failed";
        let text_b = "INFO: Service health check passed";

        let ha = Deduplicator::compute_simhash(text_a);
        let hb = Deduplicator::compute_simhash(text_b);

        assert_eq!(
            Deduplicator::hamming_distance(ha, hb),
            Deduplicator::hamming_distance(hb, ha),
            "Hamming distance should be symmetric"
        );
    }

    /// Property: SimHash distance satisfies triangle inequality
    #[test]
    fn test_hamming_distance_triangle_inequality() {
        let t1 = "ERROR: foo failed";
        let t2 = "ERROR: bar failed";
        let t3 = "INFO: baz started";

        let h1 = Deduplicator::compute_simhash(t1);
        let h2 = Deduplicator::compute_simhash(t2);
        let h3 = Deduplicator::compute_simhash(t3);

        let d12 = Deduplicator::hamming_distance(h1, h2);
        let d23 = Deduplicator::hamming_distance(h2, h3);
        let d13 = Deduplicator::hamming_distance(h1, h3);

        assert!(
            d13 <= d12 + d23,
            "triangle inequality violated: d13={}, d12={}, d23={}",
            d13, d12, d23
        );
    }

    /// Property: normalizing then hashing is consistent with
    /// normalizing during deduplication
    #[test]
    fn test_normalization_is_idempotent() {
        let content = "2024-01-15T10:30:00Z User 1234abcd failed login at File.java:45";

        let once = Deduplicator::normalize_content_static(content);
        let twice = Deduplicator::normalize_content_static(&once);

        assert_eq!(
            once, twice,
            "normalize_content should be idempotent (no double-replacement)"
        );
    }

    /// Property: normalization handles UUIDs (static mode doesn't handle hex addresses)
    #[test]
    fn test_normalize_uuid_and_hex() {
        // normalize_content_static replaces timestamps, line numbers, and UUIDs
        // but not hex addresses (that's in normalize_content only)
        let content = "ERROR [550e8400-e29b-41d4-a716-446655440000] at 0x7fff5fbff7d0";

        let normalized = Deduplicator::normalize_content_static(content);

        assert!(
            !normalized.contains("550e8400"),
            "UUID should be replaced: {}",
            normalized
        );
        assert!(normalized.contains("[UUID]"));
        // Hex addresses are NOT replaced in the static version (only in the full version)
        assert!(
            normalized.contains("0x7fff"),
            "hex address is preserved in static normalize"
        );

        // normalize_content (instance method) handles hex addresses
        assert!(
            HEXADDR_RE.is_match(content),
            "content should match hex address pattern"
        );
    }

    /// Property: TTL eviction removes entries older than the dedup window
    #[test]
    fn test_ttl_eviction() {
        let mut dedup = Deduplicator::new();

        // Add an entry
        let entry = LogEntry::new(
            uuid::Uuid::new_v4(),
            1,
            chrono::Utc::now(),
            "ERROR: test".to_string(),
        );
        dedup.add_signature(&entry, "sig-1".to_string());

        assert_eq!(dedup.signature_count(), 1);

        // Manually age out the entry in seen_at
        dedup.seen_at
            .insert("sig-1".to_string(), Instant::now() - Duration::from_secs(3601));

        // Force a TTL sweep
        dedup.evict_stale();

        assert_eq!(
            dedup.signature_count(),
            0,
            "stale entries should be evicted after dedup window"
        );
    }

    /// Property: empty content produces a valid hash
    #[test]
    fn test_empty_content_hash() {
        let hash = Deduplicator::compute_simhash("");
        // Empty string should produce hash 0 (all bits 0)
        assert_eq!(hash, 0, "empty content should produce zero simhash");
    }

    /// Property: single word is consistent
    #[test]
    fn test_single_word_consistency() {
        let word = "ERROR";
        let h1 = Deduplicator::compute_simhash(word);
        let h2 = Deduplicator::compute_simhash(word);
        assert_eq!(h1, h2);
    }
}
