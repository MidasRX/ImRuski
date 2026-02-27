//! Widget / window ID system using 64-bit FNV-1a hashing.

use std::hash::{Hash, Hasher};

/// A cheap, copyable widget identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Id(pub u64);

impl Id {
    /// Create from a string slice (FNV-1a).
    pub fn from_str(s: &str) -> Self {
        Self(fnv1a(s.as_bytes()))
    }

    /// Create from any `Hash`-able value.
    pub fn from_hash<T: Hash>(v: &T) -> Self {
        let mut h = FnvHasher::default();
        v.hash(&mut h);
        Self(h.finish())
    }

    /// Combine two IDs (e.g. window ID + label ID).
    #[inline]
    pub fn combine(self, child: Self) -> Self {
        // Boost-style hash combine
        let x = self.0 ^ child.0.wrapping_add(0x9e3779b9)
            .wrapping_add(self.0 << 6)
            .wrapping_add(self.0 >> 2);
        Self(x)
    }

    /// Returns 0 (null ID sentinel).
    pub const fn null() -> Self { Self(0) }
    pub fn is_null(self) -> bool { self.0 == 0 }
}

/// Parse an ImGui-style label: `"Label##hidden_id"`.
/// Returns `(display_text, full_id_source)`.
pub fn parse_label(label: &str) -> (&str, &str) {
    if let Some(pos) = label.find("##") {
        (&label[..pos], &label[pos + 2..])
    } else {
        (label, label)
    }
}

// ─── FNV-1a helpers ──────────────────────────────────────────────────────────

const FNV_OFFSET: u64 = 14695981039346656037;
const FNV_PRIME:  u64 = 1099511628211;

fn fnv1a(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &b in data { hash ^= b as u64; hash = hash.wrapping_mul(FNV_PRIME); }
    hash
}

#[derive(Default)]
struct FnvHasher(u64);

impl Hasher for FnvHasher {
    fn finish(&self) -> u64 { self.0 }
    fn write(&mut self, bytes: &[u8]) {
        let mut h = if self.0 == 0 { FNV_OFFSET } else { self.0 };
        for &b in bytes { h ^= b as u64; h = h.wrapping_mul(FNV_PRIME); }
        self.0 = h;
    }
}
