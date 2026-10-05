#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
use bindings::*;

use std::collections::HashMap;
use std::ffi::CString;
use std::sync::{Arc, RwLock};

use ipc_host::value_table::{Table, Value, get_value_table};
use uipc_mapping::Expr;
use uipc_mapping::FsuipcType;
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

    pub fn write(&self, xplane_value: f64) {
        if self.handle.is_null() {
            return;
        }
        let ty = unsafe { XPLMGetDataRefTypes(self.handle) };
        if let Some(array_index) = self.array_index {
            if ty & xplmType_IntArray != 0 {
                let mut v = xplane_value.round() as i32;
                unsafe { XPLMSetDatavi(self.handle, &mut v, array_index, 1) };
            } else if ty & xplmType_FloatArray != 0 {
                let mut v = xplane_value as f32;
                unsafe { XPLMSetDatavf(self.handle, &mut v, array_index, 1) };
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
        }
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
    },
}

pub struct ResolvedMapping {
    pub offset: u16,
    pub fsuipc_type: FsuipcType,
    pub size: usize,
    pub source: ResolvedSource,
    pub writable: bool,
}

impl ResolvedMapping {
    pub fn new(mapping: DatarefMapping) -> Self {
        let source = match mapping.source {
            MappingSource::Simple {
                dataref_path,
                array_index,
                scale,
                offset_add,
            } => ResolvedSource::Simple {
                dr: ResolvedRef::resolve(&dataref_path, array_index),
                scale,
                offset_add,
            },
            MappingSource::Expr {
                datarefs,
                expr,
                update_if_expr,
            } => {
                let refs = datarefs
                    .into_iter()
                    .map(|(name, (path, idx))| (name, ResolvedRef::resolve(&path, idx)))
                    .collect();
                ResolvedSource::Expr {
                    refs,
                    expr,
                    update_if_expr,
                }
            }
            MappingSource::Static { static_value } => ResolvedSource::Static {
                static_value: Some(static_value),
            },
            MappingSource::StaticStr { static_str } => ResolvedSource::StaticStr { static_str },
        };
        Self {
            offset: mapping.offset,
            fsuipc_type: mapping.fsuipc_type,
            size: mapping.size,
            source,
            writable: mapping.writable,
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
            } => {
                let mut vars = HashMap::new();
                for (name, dr) in refs {
                    vars.insert(name.clone(), dr.read().unwrap_or(0.0));
                }
                if update_if_expr.as_ref().is_some_and(|c| c.eval(&vars) <= 0.0) {
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

    /// Write a value back to X-Plane (simple mappings only; expr write-back
    /// requires knowledge of which dataref to write and the inverse expression,
    /// which is not yet supported).
    pub fn write_xplane(&self, fsuipc_value: f64) {
        if !self.writable {
            return;
        }
        if let ResolvedSource::Simple {
            dr,
            scale,
            offset_add,
        } = &self.source
        {
            let s = if scale.abs() < 1e-12 { 1.0 } else { *scale };
            dr.write((fsuipc_value - offset_add) / s);
        }
    }
}

// ─── Plugin state ──────────────────────────────────────────────────────────────

pub struct PluginState {
    pub mappings: Vec<ResolvedMapping>,
}

impl PluginState {
    pub fn new(mappings: Vec<ResolvedMapping>) -> Self {
        Self { mappings }
    }

    pub fn update(&mut self) {
        let table: Arc<RwLock<Table>> = get_value_table();
        if let Ok(mut table) = table.write() {
            let previous = table.begin_update();
            for m in &self.mappings {
                match m.read_xplane_value() {
                    Reading::Value(value) => table.insert(
                        m.offset,
                        ipc_host::value_table::Entry {
                            value,
                            source: 0,
                            destination: 0,
                            writable: m.writable,
                        },
                    ),
                    Reading::Retain => table.keep(m.offset),
                    Reading::Missing => {}
                }
            }
            // Offsets that produced no value this cycle stop being served.
            table.end_update(previous);
        }
    }

    pub fn write_offset(&mut self, offset: u16, value: f64, _size: usize) {
        for m in &self.mappings {
            if m.offset == offset && m.writable {
                m.write_xplane(value);
                tracing::debug!("Wrote value {} to offset {:#06x}", value, offset);
                return;
            }
        }
        tracing::warn!("No writable mapping found for offset {:#06x}", offset);
    }
}
