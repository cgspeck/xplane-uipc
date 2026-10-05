//! Log levels chosen at runtime from `config.toml`, for messages that have
//! their own setting (such as application key writes). The global `log_level`
//! filter still applies on top.

use std::sync::atomic::{AtomicU8, Ordering};

use tracing::level_filters::LevelFilter;

/// Selectable levels, indexed by the value a `RuntimeLevel` stores.
const LEVELS: [LevelFilter; 6] = [
    LevelFilter::OFF,
    LevelFilter::ERROR,
    LevelFilter::WARN,
    LevelFilter::INFO,
    LevelFilter::DEBUG,
    LevelFilter::TRACE,
];

/// Index of `LevelFilter::INFO` in `LEVELS`.
const INFO: u8 = 3;

/// A log level that can be changed while the plugin runs. Log at it with
/// [`log_at!`](crate::log_at).
pub struct RuntimeLevel(AtomicU8);

impl RuntimeLevel {
    /// Starts at INFO.
    pub const fn new() -> Self {
        Self(AtomicU8::new(INFO))
    }

    pub fn set(&self, level: LevelFilter) {
        let index = LEVELS
            .iter()
            .position(|l| *l == level)
            .map_or(INFO, |i| i as u8);
        self.0.store(index, Ordering::Relaxed);
    }

    pub fn get(&self) -> LevelFilter {
        LEVELS[self.0.load(Ordering::Relaxed) as usize]
    }
}

impl Default for RuntimeLevel {
    fn default() -> Self {
        Self::new()
    }
}

/// Log a message at a [`RuntimeLevel`]'s current level; nothing if it's OFF.
/// A macro, so the event keeps the calling module as its target.
#[macro_export]
macro_rules! log_at {
    ($runtime_level:expr, $($arg:tt)+) => {
        match $runtime_level.get().into_level() {
            None => {}
            Some(::tracing::Level::ERROR) => ::tracing::error!($($arg)+),
            Some(::tracing::Level::WARN) => ::tracing::warn!($($arg)+),
            Some(::tracing::Level::INFO) => ::tracing::info!($($arg)+),
            Some(::tracing::Level::DEBUG) => ::tracing::debug!($($arg)+),
            Some(_) => ::tracing::trace!($($arg)+),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_info() {
        assert_eq!(RuntimeLevel::new().get(), LevelFilter::INFO);
    }

    #[test]
    fn round_trips_every_level() {
        let level = RuntimeLevel::new();
        for filter in LEVELS {
            level.set(filter);
            assert_eq!(level.get(), filter);
        }
    }
}
