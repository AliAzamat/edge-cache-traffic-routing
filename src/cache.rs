use std::collections::HashMap;
use std::collections::VecDeque;

/// A stored object plus the metadata the edge needs to reason about it.
#[derive(Clone, Debug)]
pub struct Entry {
    pub body: Vec<u8>,
    pub etag: String,
    /// Unix seconds when this entry stops being fresh.
    pub expires_at: u64,
}

impl Entry {
    pub fn size(&self) -> usize {
        // The body dominates, but the key and metadata are not free. Counting
        // only bodies means the process uses meaningfully more memory than the
        // configured capacity, which is how edge nodes get OOM-killed.
        self.body.len() + self.etag.len() + std::mem::size_of::<Entry>()
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Stats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub revalidations: u64,
    pub bytes_stored: usize,
}

impl Stats {
    /// Hit ratio over hits and misses only.
    ///
    /// Revalidations are deliberately excluded: they are a third outcome, not
    /// a hit and not a miss. Folding them into hits inflates the number, which
    /// is the most common way a cache dashboard lies.
    pub fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }
}

pub struct LruCache {
    map: HashMap<String, Entry>,
    /// Most-recently-used at the back, least at the front.
    order: VecDeque<String>,
    capacity_bytes: usize,
    used_bytes: usize,
    pub stats: Stats,
}

impl LruCache {
    pub fn new(capacity_bytes: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            capacity_bytes,
            used_bytes: 0,
            stats: Stats::default(),
        }
    }

    pub fn get(&mut self, key: &str, now: u64) -> Option<Entry> {
        match self.map.get(key) {
            None => {
                self.stats.misses += 1;
                None
            }
            Some(entry) if entry.expires_at <= now => {
                // Present but stale. NOT a hit: we cannot serve it without
                // checking with the origin first.
                self.stats.misses += 1;
                Some(entry.clone())
            }
            Some(entry) => {
                let entry = entry.clone();
                self.touch(key);
                self.stats.hits += 1;
                Some(entry)
            }
        }
    }

    /// Move a key to the most-recently-used position.
    fn touch(&mut self, key: &str) {
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            let k = self.order.remove(pos).unwrap();
            self.order.push_back(k);
        }
    }

    pub fn put(&mut self, key: String, entry: Entry) {
        if let Some(old) = self.map.remove(&key) {
            self.used_bytes -= old.size();
            if let Some(pos) = self.order.iter().position(|k| *k == key) {
                self.order.remove(pos);
            }
        }

        let size = entry.size();
        // An object larger than the whole cache would evict everything and
        // still not fit, so refuse it outright.
        if size > self.capacity_bytes {
            return;
        }

        while self.used_bytes + size > self.capacity_bytes {
            match self.order.pop_front() {
                Some(victim) => {
                    if let Some(e) = self.map.remove(&victim) {
                        self.used_bytes -= e.size();
                        self.stats.evictions += 1;
                    }
                }
                None => break,
            }
        }

        self.used_bytes += size;
        self.stats.bytes_stored = self.used_bytes;
        self.map.insert(key.clone(), entry);
        self.order.push_back(key);
    }
}
