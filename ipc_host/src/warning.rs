use std::sync::atomic::{AtomicU64, Ordering};

const CATEGORY_COUNT: usize = 3;
const ENTRIES_PER_CATEGORY: usize = 1024;

#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub enum WarnCategory {
    ReadNotExist = 0,
    WriteNotExist = 1,
    WriteNotWritable = 2,
    COUNT = 3,
}

#[derive(Debug)]
pub struct WarnedSet {
    bits: Vec<Vec<AtomicU64>>,
}

impl Clone for WarnedSet {
    fn clone(&self) -> Self {
        let bits: Vec<Vec<AtomicU64>> = self
            .bits
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| AtomicU64::new(cell.load(Ordering::Relaxed)))
                    .collect()
            })
            .collect();
        Self { bits }
    }
}

impl WarnedSet {
    pub fn new() -> Self {
        let bits: Vec<Vec<AtomicU64>> = (0..CATEGORY_COUNT)
            .map(|_| {
                (0..ENTRIES_PER_CATEGORY)
                    .map(|_| AtomicU64::new(0))
                    .collect()
            })
            .collect();
        Self { bits }
    }

    pub fn check_and_set(&self, key: u16, category: WarnCategory) -> bool {
        let cat_idx = category as usize;
        if cat_idx >= CATEGORY_COUNT {
            tracing::debug!("check_and_set: cat_idx out of bounds");
            return false;
        }
        tracing::trace!("check_and_set: category={:#?}, key={:#07x}", category, key);
        let row = &self.bits[cat_idx];
        tracing::trace!("check_and_set: category row accessed");
        let (idx, bit) = Self::locate(key);
        tracing::trace!("check_and_set: idx={}, bit={:#016x}", idx, bit);
        if idx >= ENTRIES_PER_CATEGORY {
            tracing::debug!("check_and_set: idx out of bounds");
            return false;
        }
        tracing::trace!("check_and_set: about to fetch_or");
        let result = row[idx].fetch_or(bit, Ordering::Relaxed);
        tracing::trace!("check_and_set: fetch_or done, result={:#018x}", result);
        result & bit == 0
    }

    pub fn clear_key(&self, key: u16, category: WarnCategory) {
        let cat_idx = category as usize;
        if cat_idx >= CATEGORY_COUNT {
            return;
        }
        let row = &self.bits[cat_idx];
        let (idx, bit) = Self::locate(key);
        if idx >= ENTRIES_PER_CATEGORY {
            return;
        }
        row[idx].fetch_and(!bit, Ordering::Relaxed);
    }

    pub fn clear(&self, category: WarnCategory) {
        self.bits[category as usize]
            .iter()
            .for_each(|cell| cell.store(0, Ordering::Relaxed));
    }

    pub fn clear_all(&self) {
        for row in self.bits.iter() {
            for cell in row {
                cell.store(0, Ordering::Relaxed);
            }
        }
    }

    #[inline(always)]
    pub fn locate(key: u16) -> (usize, u64) {
        let idx = key as usize / 64;
        let bit_idx = key as usize % 64;
        let bit = 1u64 << bit_idx;
        (idx, bit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_and_set_first_time_returns_true() {
        let warned = WarnedSet::new();
        let result = warned.check_and_set(0, WarnCategory::ReadNotExist);
        assert!(result, "First call should return true (bit was not set)");
    }

    #[test]
    fn test_check_and_set_subsequent_calls_return_false() {
        let warned = WarnedSet::new();
        let _ = warned.check_and_set(0, WarnCategory::ReadNotExist);
        let result = warned.check_and_set(0, WarnCategory::ReadNotExist);
        assert!(
            !result,
            "Subsequent calls should return false (bit already set)"
        );
    }

    #[test]
    fn test_check_and_set_different_keys_independent() {
        let warned = WarnedSet::new();
        let r1 = warned.check_and_set(0, WarnCategory::ReadNotExist);
        let r2 = warned.check_and_set(1, WarnCategory::ReadNotExist);
        let r1_again = warned.check_and_set(0, WarnCategory::ReadNotExist);
        assert!(r1, "First key should return true");
        assert!(r2, "Different key should return true");
        assert!(!r1_again, "First key again should return false");
    }

    #[test]
    fn test_check_and_set_different_categories_independent() {
        let warned = WarnedSet::new();
        let _ = warned.check_and_set(0, WarnCategory::ReadNotExist);
        let r = warned.check_and_set(0, WarnCategory::WriteNotExist);
        assert!(
            r,
            "Different category should return true (independent bit set)"
        );
    }

    #[test]
    fn test_locate_key_0() {
        let (idx, bit) = WarnedSet::locate(0);
        assert_eq!(idx, 0, "Index should be 0 for key 0");
        assert_eq!(bit, 1, "Bit should be 1 for key 0");
    }

    #[test]
    fn test_locate_key_63() {
        let (idx, bit) = WarnedSet::locate(63);
        assert_eq!(idx, 0, "Index should be 0 for key 63");
        assert_eq!(
            bit, 0x8000000000000000u64,
            "Bit should be 0x8000000000000000 for key 63"
        );
    }

    #[test]
    fn test_locate_key_64() {
        let (idx, bit) = WarnedSet::locate(64);
        assert_eq!(idx, 1, "Index should be 1 for key 64");
        assert_eq!(bit, 1, "Bit should be 1 for key 64");
    }

    #[test]
    fn test_locate_key_0x0290c() {
        let (idx, bit) = WarnedSet::locate(0x0290c);
        assert_eq!(idx, 164, "Index should be 164 for key 0x0290c (10508)");
        assert_eq!(
            bit, 0x1000u64,
            "Bit should be 0x1000 for key 0x0290c (10508)"
        );
    }

    #[test]
    fn test_clear_clears_only_that_category() {
        let warned = WarnedSet::new();
        warned.check_and_set(0, WarnCategory::ReadNotExist);
        warned.check_and_set(0, WarnCategory::WriteNotExist);
        warned.check_and_set(0, WarnCategory::WriteNotWritable);
        warned.clear(WarnCategory::ReadNotExist);
        assert!(
            warned.check_and_set(0, WarnCategory::ReadNotExist),
            "ReadNotExist should be cleared - returns true (bit not set)"
        );
        assert!(
            !warned.check_and_set(0, WarnCategory::WriteNotExist),
            "WriteNotExist should still be set"
        );
        assert!(
            !warned.check_and_set(0, WarnCategory::WriteNotWritable),
            "WriteNotWritable should still be set"
        );
    }

    #[test]
    fn test_clear_all_clears_all() {
        let warned = WarnedSet::new();
        warned.check_and_set(0, WarnCategory::ReadNotExist);
        warned.check_and_set(1, WarnCategory::WriteNotExist);
        warned.check_and_set(2, WarnCategory::WriteNotWritable);
        warned.clear_all();
        assert!(
            warned.check_and_set(0, WarnCategory::ReadNotExist),
            "ReadNotExist should be cleared"
        );
        assert!(
            warned.check_and_set(1, WarnCategory::WriteNotExist),
            "WriteNotExist should be cleared"
        );
        assert!(
            warned.check_and_set(2, WarnCategory::WriteNotWritable),
            "WriteNotWritable should be cleared"
        );
    }

    #[test]
    fn test_clear_key_allows_re_warning() {
        let warned = WarnedSet::new();
        warned.check_and_set(0x3304, WarnCategory::ReadNotExist);
        warned.clear_key(0x3304, WarnCategory::ReadNotExist);
        assert!(
            warned.check_and_set(0x3304, WarnCategory::ReadNotExist),
            "Cleared key should return true (bit not set)"
        );
    }

    #[test]
    fn test_clear_key_does_not_affect_other_keys() {
        let warned = WarnedSet::new();
        warned.check_and_set(0x3304, WarnCategory::ReadNotExist);
        warned.check_and_set(0x3308, WarnCategory::ReadNotExist);
        warned.clear_key(0x3304, WarnCategory::ReadNotExist);
        assert!(
            warned.check_and_set(0x3304, WarnCategory::ReadNotExist),
            "Cleared key should return true"
        );
        assert!(
            !warned.check_and_set(0x3308, WarnCategory::ReadNotExist),
            "Other key should still be set"
        );
    }

    #[test]
    fn test_clear_key_does_not_affect_other_categories() {
        let warned = WarnedSet::new();
        warned.check_and_set(0x3304, WarnCategory::ReadNotExist);
        warned.check_and_set(0x3304, WarnCategory::WriteNotExist);
        warned.clear_key(0x3304, WarnCategory::ReadNotExist);
        assert!(
            warned.check_and_set(0x3304, WarnCategory::ReadNotExist),
            "ReadNotExist should be cleared"
        );
        assert!(
            !warned.check_and_set(0x3304, WarnCategory::WriteNotExist),
            "WriteNotExist should still be set"
        );
    }

    #[test]
    fn test_trace_output_does_not_panic() {
        let warned = WarnedSet::new();
        for key in 0u16..2000 {
            for cat in [
                WarnCategory::ReadNotExist,
                WarnCategory::WriteNotExist,
                WarnCategory::WriteNotWritable,
            ] {
                warned.check_and_set(key, cat);
                warned.check_and_set(key, cat);
            }
        }
    }
}
