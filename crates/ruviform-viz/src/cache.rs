//! [`BoundedCache`] — a byte-budgeted LRU for viewport payloads.
//!
//! View queries produce bounded responses (view bins, tiles, encoded
//! bytes); caching them lets pans and repeated zooms avoid recomputing
//! or re-reading the source. The cache is keyed by whatever the caller
//! chooses (e.g. `(channel, start, end, bins)`), charges an explicit
//! byte cost per entry, and evicts least-recently-used entries to stay
//! under its budget — memory stays bounded regardless of dataset size.
//!
//! `viz` owns no invalidation policy: application data is immutable
//! per published source, so callers drop or rebuild caches when they
//! publish new sources.

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

/// LRU cache bounded by total byte cost.
///
/// `V` is the payload (typically `Vec<u8>` or a decoded tile); the
/// caller supplies its byte cost at [`insert`](Self::insert). Entries
/// larger than the whole budget are never stored. `get` refreshes
/// recency.
///
/// **RT contract:** non-RT only.
#[derive(Debug)]
pub struct BoundedCache<K, V> {
    budget: usize,
    used: usize,
    map: HashMap<K, (V, usize)>,
    /// Front = most recently used; back = eviction candidate.
    lru: VecDeque<K>,
}

impl<K: Eq + Hash + Clone, V> BoundedCache<K, V> {
    /// A cache holding at most `budget` bytes total.
    #[must_use]
    pub fn new(budget: usize) -> Self {
        Self {
            budget,
            used: 0,
            map: HashMap::new(),
            lru: VecDeque::new(),
        }
    }

    /// Total byte cost currently stored.
    #[must_use]
    pub fn used_bytes(&self) -> usize {
        self.used
    }

    /// Entry count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Looks up `key`, refreshing its recency.
    #[must_use]
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            self.touch(key);
            self.map.get(key).map(|(v, _)| v)
        } else {
            None
        }
    }

    /// Inserts `value` at `cost` bytes. If `key` already exists its old
    /// entry is replaced (cost refunded). Evicts LRU entries until the
    /// entry fits; values larger than the whole budget are dropped.
    /// Returns `true` when the value was stored.
    pub fn insert(&mut self, key: K, value: V, cost: usize) -> bool {
        if let Some((_, old_cost)) = self.map.remove(&key) {
            self.used -= old_cost;
        }
        if cost > self.budget {
            return false;
        }
        while self.used + cost > self.budget {
            let Some(evict) = self.lru.pop_back() else {
                break;
            };
            if let Some((_, c)) = self.map.remove(&evict) {
                self.used -= c;
            }
        }
        self.used += cost;
        self.touch(&key); // borrow first, then consume `key` into the map
        self.map.insert(key, (value, cost));
        true
    }

    /// Whether `key` is present (does not refresh recency).
    #[must_use]
    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    /// Drops all entries.
    pub fn clear(&mut self) {
        self.map.clear();
        self.lru.clear();
        self.used = 0;
    }

    /// Moves `key` to the most-recently-used position.
    fn touch(&mut self, key: &K) {
        // Rebuild-on-touch keeps the structure simple at viz payload
        // sizes (bounded by budget/entry); eviction order is exact.
        if let Some(pos) = self.lru.iter().position(|k| k == key) {
            self.lru.remove(pos);
        }
        self.lru.push_front(key.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_lru_within_budget() {
        let mut c = BoundedCache::new(10);
        c.insert("a", vec![0u8; 4], 4);
        c.insert("b", vec![0u8; 4], 4);
        assert_eq!(c.used_bytes(), 8);
        // Touch a → b is now the eviction candidate.
        assert!(c.get(&"a").is_some());
        c.insert("c", vec![0u8; 4], 4);
        assert!(!c.contains(&"b"));
        assert!(c.contains(&"a") && c.contains(&"c"));
        assert_eq!(c.used_bytes(), 8);
    }

    #[test]
    fn oversized_entries_are_never_stored() {
        let mut c = BoundedCache::new(4);
        assert!(!c.insert("big", vec![0u8; 9], 9));
        assert!(!c.contains(&"big"));
        assert_eq!(c.used_bytes(), 0);
    }

    #[test]
    fn replace_refunds_old_cost() {
        let mut c = BoundedCache::new(10);
        c.insert("a", vec![0u8; 4], 4);
        c.insert("a", vec![0u8; 6], 6);
        assert_eq!(c.used_bytes(), 6);
        assert_eq!(c.get(&"a").unwrap().len(), 6);
    }
}
