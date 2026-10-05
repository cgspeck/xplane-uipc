#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
use bindings::*;

use std::collections::{BTreeMap, HashMap};
use std::ffi::CString;
use std::sync::{Arc, RwLock};

use ipc_host::USER_AREA;
use ipc_host::value_table::{Table, Value, get_value_table};
use uipc_mapping::Expr;
use uipc_mapping::FsuipcType;
use uipc_mapping::eval_writes;
pub use uipc_mapping::{DatarefMapping, MappingSource};

/// Bound a raw string read to `max_len` bytes, always ending in a NUL.
/// Bytes after the first NUL are dropped; an empty read becomes an empty string.
fn terminate_string(mut buf: Vec<u8>, max_len: usize) -> Vec<u8> {
    if max_len == 0 {
        return Vec::new();
    }
    if let Some(nul) = buf.iter().position(|&b| b == 0) {
        buf.truncate(nul);
    }
    buf.truncate(max_len - 1);
    buf.push(0);
    buf
}

/// Outcome of evaluating a mapping for one update cycle.
pub enum Reading {
    /// Serve this value.
    Value(Value),
    /// Keep serving the previous value (`update_if_expr` was false).
    Retain,
    /// No value available; stop serving the offset.
    Missing,
}

// ─── Resolved dataref handle ──────────────────────────────────────────────────
#[derive(Clone)]
pub struct ResolvedRef {
    pub handle: XPLMDataRef,
    pub array_index: Option<i32>,
    /// Dataref path, without any `[N]` index, for log messages.
    pub path: String,
}

impl ResolvedRef {
    fn resolve(path: &str, array_index: Option<i32>) -> Self {
        let handle = match CString::new(path) {
            Ok(cs) => unsafe { XPLMFindDataRef(cs.as_ptr()) },
            Err(_) => std::ptr::null_mut(),
        };
        if handle.is_null() {
            tracing::warn!("dataref not found: '{}'", path);
        }
        Self {
            handle,
            array_index,
            path: path.to_string(),
        }
    }

    /// Read a NUL-terminated string of at most `max_len` bytes (None if the handle is invalid).
    pub fn read_bytes(&self, max_len: usize) -> Option<Vec<u8>> {
        if self.handle.is_null() {
            return None;
        }
        let mut buf = vec![0u8; max_len];
        let bytes_read = unsafe {
            XPLMGetDatab(
                self.handle,
                buf.as_mut_ptr() as *mut std::ffi::c_void,
                0,
                max_len as i32,
            )
        };
        buf.truncate(bytes_read.clamp(0, max_len as i32) as usize);
        Some(terminate_string(buf, max_len))
    }

    pub fn read(&self) -> Option<f64> {
        if self.handle.is_null() {
            return None;
        }
        let ty = unsafe { XPLMGetDataRefTypes(self.handle) };
        let memo: Option<f64>;

        if let Some(array_index) = self.array_index {
            memo = match ty {
                // Get*v returns the number of elements copied; 0 means the index is out of range.
                _ if (ty & xplmType_IntArray) != 0 => unsafe {
                    let mut v: i32 = 0;
                    let n = XPLMGetDatavi(self.handle, &mut v, array_index, 1);
                    tracing::trace!("retrieve array index: {}, i32 value: {}", array_index, v);
                    (n == 1).then_some(v as f64)
                },
                _ if (ty & xplmType_FloatArray) != 0 => unsafe {
                    let mut v: f32 = 0.0;
                    let n = XPLMGetDatavf(self.handle, &mut v, array_index, 1);
                    tracing::trace!("retrieve array index: {}, f32 value: {}", array_index, v);
                    (n == 1).then_some(v as f64)
                },
                _ => None,
            };
        } else {
            tracing::trace!("retrieve scalar value");
            memo = match ty {
                _ if (ty & xplmType_Int) != 0 => unsafe { Some(XPLMGetDatai(self.handle) as f64) },
                _ if (ty & xplmType_Float) != 0 => unsafe {
                    Some(XPLMGetDataf(self.handle) as f64)
                },
                _ if (ty & xplmType_Double) != 0 => unsafe { Some(XPLMGetDatad(self.handle)) },
                _ => None,
            };
        }
        if memo.is_none() {
            tracing::trace!("No value retrieved");
        }
        memo
    }

    /// Write a value to the dataref. Returns false if the handle is invalid or
    /// the dataref's type can't be written as a number.
    pub fn write(&self, xplane_value: f64) -> bool {
        if self.handle.is_null() {
            return false;
        }
        let ty = unsafe { XPLMGetDataRefTypes(self.handle) };
        if let Some(array_index) = self.array_index {
            if ty & xplmType_IntArray != 0 {
                let mut v = xplane_value.round() as i32;
                unsafe { XPLMSetDatavi(self.handle, &mut v, array_index, 1) };
            } else if ty & xplmType_FloatArray != 0 {
                let mut v = xplane_value as f32;
                unsafe { XPLMSetDatavf(self.handle, &mut v, array_index, 1) };
            } else {
                return false;
            }
        } else if ty & xplmType_Double != 0 {
            unsafe {
                XPLMSetDatad(self.handle, xplane_value);
            }
        } else if ty & xplmType_Float != 0 {
            unsafe {
                XPLMSetDataf(self.handle, xplane_value as f32);
            }
        } else if ty & xplmType_Int != 0 {
            unsafe {
                XPLMSetDatai(self.handle, xplane_value.round() as i32);
            }
        } else {
            return false;
        }
        true
    }
}

// ─── Resolved mapping ─────────────────────────────────────────────────────────

pub enum ResolvedSource {
    Simple {
        dr: ResolvedRef,
        scale: f64,
        offset_add: f64,
    },
    Static {
        static_value: Option<f64>,
    },
    StaticStr {
        static_str: String,
    },
    Expr {
        /// name → resolved ref
        refs: HashMap<String, ResolvedRef>,
        expr: Expr,
        update_if_expr: Option<Expr>,
        /// Target name (a key of `refs`) → expression for the value to write there.
        write_exprs: BTreeMap<String, Expr>,
    },
}

pub struct ResolvedMapping {
    pub offset: u16,
    pub fsuipc_type: FsuipcType,
    pub size: usize,
    pub source: ResolvedSource,
    pub writable: bool,
    /// Set once a write has skipped an unavailable dataref, so later skips log
    /// at debug instead of flooding the log. Reset by reloading mappings.
    unavailable_warned: bool,
}

impl ResolvedMapping {
    pub fn new(mapping: DatarefMapping) -> Self {
        let source = match mapping.source {
            MappingSource::Simple {
                dataref_path,
                array_index,
                scale,
                offset_add,
            } => {
                let dr = ResolvedRef::resolve(&dataref_path, array_index);
                // X-Plane silently ignores writes to read-only datarefs.
                if mapping.writable
                    && !dr.handle.is_null()
                    && unsafe { XPLMCanWriteDataRef(dr.handle) } == 0
                {
                    tracing::warn!(
                        "Offset {:#06x} is marked writable but dataref '{}' is read-only; writes to it will have no effect",
                        mapping.offset,
                        dataref_path
                    );
                }
                ResolvedSource::Simple {
                    dr,
                    scale,
                    offset_add,
                }
            }
            MappingSource::Expr {
                datarefs,
                expr,
                update_if_expr,
                write_exprs,
                ..
            } => {
                let refs: HashMap<String, ResolvedRef> = datarefs
                    .into_iter()
                    .map(|(name, (path, idx))| (name, ResolvedRef::resolve(&path, idx)))
                    .collect();
                for target in write_exprs.keys() {
                    if let Some(dr) = refs.get(target)
                        && !dr.handle.is_null()
                        && unsafe { XPLMCanWriteDataRef(dr.handle) } == 0
                    {
                        tracing::warn!(
                            "Offset {:#06x} writes '{}' but dataref '{}' is read-only; writes to it will have no effect",
                            mapping.offset,
                            target,
                            dr.path
                        );
                    }
                }
                ResolvedSource::Expr {
                    refs,
                    expr,
                    update_if_expr,
                    write_exprs,
                }
            }
            MappingSource::Static { static_value } => ResolvedSource::Static {
                static_value: Some(static_value),
            },
            MappingSource::StaticStr { static_str } => ResolvedSource::StaticStr { static_str },
        };
        if mapping.writable
            && matches!(
                source,
                ResolvedSource::Static { .. } | ResolvedSource::StaticStr { .. }
            )
        {
            tracing::warn!(
                "Offset {:#06x} is marked writable but static values can't be written; writes to it will have no effect",
                mapping.offset
            );
        }
        Self {
            offset: mapping.offset,
            fsuipc_type: mapping.fsuipc_type,
            size: mapping.size,
            source,
            writable: mapping.writable,
            unavailable_warned: false,
        }
    }

    /// Evaluate the mapping for this update cycle.
    ///
    /// Simple mappings are `Missing` when their dataref is unavailable. In
    /// expressions, an unavailable dataref evaluates as 0.0 (documented in
    /// README-EXPR.md; mappings rely on it to mix aircraft-specific datarefs).
    /// A false `update_if_expr` yields `Retain`.
    pub fn read_xplane_value(&self) -> Reading {
        if self.fsuipc_type == FsuipcType::String {
            let bytes = match &self.source {
                ResolvedSource::Simple { dr, .. } => dr.read_bytes(self.size),
                ResolvedSource::StaticStr { static_str } => {
                    Some(terminate_string(static_str.as_bytes().to_vec(), self.size))
                }
                _ => None,
            };
            return match bytes {
                Some(b) => Reading::Value(Value::String(b)),
                None => Reading::Missing,
            };
        }

        let value = match &self.source {
            ResolvedSource::Simple {
                dr,
                scale,
                offset_add,
            } => dr.read().map(|v| v * scale + offset_add),
            ResolvedSource::Expr {
                refs,
                expr,
                update_if_expr,
                ..
            } => {
                let vars = read_vars(refs);
                if update_if_expr
                    .as_ref()
                    .is_some_and(|c| c.eval(&vars) <= 0.0)
                {
                    return Reading::Retain;
                }
                Some(expr.eval(&vars))
            }
            ResolvedSource::Static { static_value } => *static_value,
            ResolvedSource::StaticStr { .. } => None,
        };
        match value.and_then(|v| Value::from_f64(v, self.fsuipc_type)) {
            Some(v) => Reading::Value(v),
            None => Reading::Missing,
        }
    }

    /// Write a client's value back to X-Plane. Returns whether any dataref
    /// was written.
    ///
    /// Simple mappings reverse `scale`/`offset_add`. Expression mappings
    /// evaluate every write expression against one snapshot of their datarefs
    /// (unavailable ones read as 0.0) and write nothing if any result isn't
    /// finite.
    pub fn write_xplane(&mut self, fsuipc_value: f64) -> bool {
        if !self.writable {
            return false;
        }
        let mut written = false;
        let mut skipped = Vec::new();
        match &self.source {
            ResolvedSource::Simple {
                dr,
                scale,
                offset_add,
            } => {
                debug_assert!(
                    *scale != 0.0,
                    "the loader rejects writable mappings with scale = 0"
                );
                if dr.write((fsuipc_value - offset_add) / scale) {
                    written = true;
                } else {
                    skipped.push(dr.path.clone());
                }
            }
            ResolvedSource::Expr {
                refs, write_exprs, ..
            } => {
                let mut vars = read_vars(refs);
                vars.insert("value".into(), fsuipc_value);
                let no_commands = BTreeMap::new();
                let results = match eval_writes(write_exprs, &no_commands, &vars) {
                    Ok(plan) => plan.datarefs,
                    Err(e) => {
                        tracing::warn!("Offset {:#06x}: {}; nothing written", self.offset, e);
                        return false;
                    }
                };
                // The loader checks every target names a dataref.
                for (target, v) in results {
                    let Some(dr) = refs.get(target) else { continue };
                    if dr.write(v) {
                        written = true;
                    } else {
                        skipped.push(dr.path.clone());
                    }
                }
            }
            ResolvedSource::Static { .. } | ResolvedSource::StaticStr { .. } => {}
        }
        for path in skipped {
            self.report_unavailable(&path);
        }
        written
    }

    /// Log a write skipped because its dataref is unavailable: a warning the
    /// first time for this mapping since mappings were loaded, debug after.
    fn report_unavailable(&mut self, path: &str) {
        if self.unavailable_warned {
            tracing::debug!(
                "Offset {:#06x}: dataref '{}' is unavailable; not written",
                self.offset,
                path
            );
        } else {
            tracing::warn!(
                "Offset {:#06x}: dataref '{}' is unavailable; not written (further skips for this offset are logged at debug level)",
                self.offset,
                path
            );
            self.unavailable_warned = true;
        }
    }
}

/// Current value of each named dataref; unavailable ones read as 0.0.
fn read_vars(refs: &HashMap<String, ResolvedRef>) -> HashMap<String, f64> {
    refs.iter()
        .map(|(name, dr)| (name.clone(), dr.read().unwrap_or(0.0)))
        .collect()
}

// ─── Built-in offsets ──────────────────────────────────────────────────────────

/// FSUIPC activity counter: incremented once per update cycle, wrapping at 0xFFFF.
pub const ACTIVITY_COUNTER_OFFSET: u16 = 0x337E;

/// Offsets generated by the plugin. They are read-only and take precedence over
/// any mapping for the same offset.
pub const BUILTIN_OFFSETS: &[u16] = &[ACTIVITY_COUNTER_OFFSET];

/// Remove mappings that target a built-in offset, warning about each one.
pub fn drop_builtin_offsets(mappings: Vec<DatarefMapping>) -> Vec<DatarefMapping> {
    mappings
        .into_iter()
        .filter(|m| {
            let builtin = BUILTIN_OFFSETS.contains(&m.offset);
            if builtin {
                tracing::warn!(
                    "Ignoring mapping for offset {:#06x}: it is a built-in offset",
                    m.offset
                );
            }
            !builtin
        })
        .collect()
}

/// Remove mappings that target the user area, which is served by the IPC
/// window as plain memory, warning about each one.
pub fn drop_user_area_offsets(mappings: Vec<DatarefMapping>) -> Vec<DatarefMapping> {
    mappings
        .into_iter()
        .filter(|m| {
            let in_area = USER_AREA.contains(&m.offset);
            if in_area {
                tracing::warn!(
                    "Ignoring mapping for offset {:#06x}: it is in the user area {:#06x}-{:#06x}",
                    m.offset,
                    USER_AREA.start(),
                    USER_AREA.end()
                );
            }
            !in_area
        })
        .collect()
}

/// Put one mapping's reading for this update cycle into the value table.
///
/// `Retain` keeps the previous value. If there is none yet, e.g. an
/// `update_if_expr` mapping that has been false since the plugin started, the
/// offset is served as zero, as FSUIPC does at startup.
fn apply_reading(table: &mut Table, mapping: &ResolvedMapping, reading: Reading) {
    let value = match reading {
        Reading::Value(value) => value,
        Reading::Retain if table.get(mapping.offset).is_some() => {
            table.keep(mapping.offset);
            return;
        }
        Reading::Retain => match Value::from_f64(0.0, mapping.fsuipc_type) {
            Some(zero) => zero,
            None => return,
        },
        Reading::Missing => return,
    };
    table.insert(
        mapping.offset,
        ipc_host::value_table::Entry {
            value,
            source: 0,
            destination: 0,
            writable: mapping.writable,
        },
    );
}

// ─── Plugin state ──────────────────────────────────────────────────────────────

pub struct PluginState {
    pub mappings: Vec<ResolvedMapping>,
    /// Not reset on mapping reload; only `mappings` is replaced.
    activity_counter: u16,
}

impl PluginState {
    pub fn new(mappings: Vec<ResolvedMapping>) -> Self {
        Self {
            mappings,
            activity_counter: 0,
        }
    }

    /// Advance and insert the plugin-generated offsets for this update cycle.
    fn update_builtins(&mut self, table: &mut Table) {
        self.activity_counter = self.activity_counter.wrapping_add(1);
        table.insert(
            ACTIVITY_COUNTER_OFFSET,
            ipc_host::value_table::Entry {
                value: Value::UnsignedInteger16(self.activity_counter),
                source: 0,
                destination: 0,
                writable: false,
            },
        );
    }

    pub fn update(&mut self) {
        let table: Arc<RwLock<Table>> = get_value_table();
        if let Ok(mut table) = table.write() {
            let previous = table.begin_update();
            for m in &self.mappings {
                apply_reading(&mut table, m, m.read_xplane_value());
            }
            self.update_builtins(&mut table);
            // Offsets that produced no value this cycle stop being served.
            table.end_update(previous);
        }
    }

    pub fn write_offset(&mut self, offset: u16, value: f64, _size: usize) {
        for m in &mut self.mappings {
            if m.offset == offset && m.writable {
                if m.write_xplane(value) {
                    tracing::debug!("Wrote value {} to offset {:#06x}", value, offset);
                }
                return;
            }
        }
        tracing::warn!("No writable mapping found for offset {:#06x}", offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn static_mapping(offset: u16) -> DatarefMapping {
        DatarefMapping {
            offset,
            fsuipc_type: FsuipcType::U16,
            size: 2,
            source: MappingSource::Static { static_value: 1.0 },
            writable: true,
        }
    }

    fn counter(table: &Table) -> Option<u16> {
        match table.get(ACTIVITY_COUNTER_OFFSET)?.value {
            Value::UnsignedInteger16(v) => Some(v),
            _ => None,
        }
    }

    #[test]
    fn activity_counter_increments_each_cycle() {
        let mut state = PluginState::new(vec![]);
        let mut table = Table::new();
        state.update_builtins(&mut table);
        assert_eq!(counter(&table), Some(1));
        state.update_builtins(&mut table);
        assert_eq!(counter(&table), Some(2));
        assert!(table.is_active(ACTIVITY_COUNTER_OFFSET));
        assert!(!table.is_writable(ACTIVITY_COUNTER_OFFSET));
    }

    #[test]
    fn activity_counter_wraps() {
        let mut state = PluginState::new(vec![]);
        state.activity_counter = 0xFFFF;
        let mut table = Table::new();
        state.update_builtins(&mut table);
        assert_eq!(counter(&table), Some(0));
    }

    #[test]
    fn activity_counter_survives_update_cycle_expiry() {
        let mut state = PluginState::new(vec![]);
        let mut table = Table::new();
        for _ in 0..3 {
            let previous = table.begin_update();
            state.update_builtins(&mut table);
            table.end_update(previous);
        }
        assert_eq!(counter(&table), Some(3));
    }

    #[test]
    fn mappings_on_builtin_offsets_are_dropped() {
        let kept = drop_builtin_offsets(vec![
            static_mapping(0x1000),
            static_mapping(ACTIVITY_COUNTER_OFFSET),
            static_mapping(0x337C),
        ]);
        let offsets: Vec<u16> = kept.iter().map(|m| m.offset).collect();
        assert_eq!(offsets, vec![0x1000, 0x337C]);
    }

    #[test]
    fn mappings_in_user_area_are_dropped() {
        let kept = drop_user_area_offsets(vec![
            static_mapping(0x66BF),
            static_mapping(0x66C0),
            static_mapping(0x66D0),
            static_mapping(0x66FF),
            static_mapping(0x6700),
        ]);
        let offsets: Vec<u16> = kept.iter().map(|m| m.offset).collect();
        assert_eq!(offsets, vec![0x66BF, 0x6700]);
    }

    #[test]
    fn terminate_string_fills_size_exactly() {
        assert_eq!(
            terminate_string(b"ABCDEFGH".to_vec(), 8),
            b"ABCDEFG\0".to_vec()
        );
    }

    #[test]
    fn terminate_string_keeps_existing_nul() {
        assert_eq!(terminate_string(b"AB\0junk".to_vec(), 8), b"AB\0".to_vec());
    }

    #[test]
    fn terminate_string_empty_read_is_empty_string() {
        assert_eq!(terminate_string(Vec::new(), 8), vec![0]);
        assert_eq!(terminate_string(b"x".to_vec(), 0), Vec::<u8>::new());
    }

    /// A mapping shaped like 0x030C (touchdown vertical speed, `i32`).
    fn touchdown_vs_mapping() -> ResolvedMapping {
        ResolvedMapping {
            offset: 0x030C,
            fsuipc_type: FsuipcType::I32,
            size: 4,
            source: ResolvedSource::Static { static_value: None },
            writable: false,
            unavailable_warned: false,
        }
    }

    /// Apply one reading inside an update cycle, as `PluginState::update` does.
    fn run_cycle(table: &mut Table, mapping: &ResolvedMapping, reading: Reading) {
        let previous = table.begin_update();
        apply_reading(table, mapping, reading);
        table.end_update(previous);
    }

    #[test]
    fn retain_without_previous_value_serves_zero() {
        let mapping = touchdown_vs_mapping();
        let mut table = Table::new();
        run_cycle(&mut table, &mapping, Reading::Retain);
        assert!(matches!(
            table.get(0x030C).map(|e| &e.value),
            Some(Value::Integer32(0))
        ));
        assert!(table.is_active(0x030C));
    }

    #[test]
    fn retain_keeps_previous_value() {
        let mapping = touchdown_vs_mapping();
        let mut table = Table::new();
        run_cycle(&mut table, &mapping, Reading::Value(Value::Integer32(-512)));
        run_cycle(&mut table, &mapping, Reading::Retain);
        run_cycle(&mut table, &mapping, Reading::Retain);
        assert!(matches!(
            table.get(0x030C).map(|e| &e.value),
            Some(Value::Integer32(-512))
        ));
    }

    /// Counts warning events, to check what a test logs.
    struct CountWarnings(Arc<std::sync::atomic::AtomicUsize>);

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for CountWarnings {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            if *event.metadata().level() == tracing::Level::WARN {
                self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }

    #[test]
    fn unavailable_dataref_warns_once_per_load() {
        use tracing_subscriber::layer::SubscriberExt;

        let warnings = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let subscriber = tracing_subscriber::registry().with(CountWarnings(warnings.clone()));
        let mut mapping = touchdown_vs_mapping();
        tracing::subscriber::with_default(subscriber, || {
            for _ in 0..3 {
                mapping.report_unavailable("addon/missing");
            }
        });
        assert_eq!(warnings.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn missing_is_not_served() {
        let mapping = touchdown_vs_mapping();
        let mut table = Table::new();
        run_cycle(&mut table, &mapping, Reading::Missing);
        assert!(table.get(0x030C).is_none());
    }
}
