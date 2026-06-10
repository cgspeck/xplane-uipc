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

fn f64_to_value(value: f64, ty: FsuipcType) -> Value {
    match ty {
        FsuipcType::U8 => Value::UnsignedInteger8(value as u8),
        FsuipcType::I8 => Value::Integer8(value as i8),
        FsuipcType::U16 => Value::UnsignedInteger16(value as u16),
        FsuipcType::I16 => Value::Integer16(value as i16),
        FsuipcType::U32 => Value::UnsignedInteger32(value as u32),
        FsuipcType::I32 => Value::Integer32(value as i32),
        FsuipcType::U64 => Value::UnsignedInteger64(value as u64),
        FsuipcType::I64 => Value::Integer64(value as i64),
        FsuipcType::F32 => Value::Float32(value as f32),
        FsuipcType::F64 => Value::Float64(value),
        FsuipcType::String => Value::String(vec![0]),
    }
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

    /// Read the scalar value from this dataref (returns None if invalid).
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
        if bytes_read == 0 {
            return None;
        }
        buf.truncate(bytes_read as usize);
        if buf.last() != Some(&0) {
            buf.push(0);
        }
        Some(buf)
    }

    pub fn read(&self) -> Option<f64> {
        if self.handle.is_null() {
            return None;
        }
        let ty = unsafe { XPLMGetDataRefTypes(self.handle) };
        let memo: Option<f64>;

        if let Some(array_index) = self.array_index {
            memo = match ty {
                _ if (ty & xplmType_IntArray) != 0 => unsafe {
                    let mut v: i32 = 0;
                    XPLMGetDatavi(self.handle, &mut v, array_index, 1);
                    tracing::trace!("retrieve array index: {}, i32 value: {}", array_index, v);
                    Some(v as f64)
                },
                _ if (ty & xplmType_FloatArray) != 0 => unsafe {
                    let mut v: f32 = 0.0;
                    XPLMGetDatavf(self.handle, &mut v, array_index, 1);
                    tracing::trace!("retrieve array index: {}, f32 value: {}", array_index, v);
                    Some(v as f64)
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
        if ty & xplmType_Double != 0 {
            unsafe {
                XPLMSetDatad(self.handle, xplane_value);
            }
        } else if ty & xplmType_Float != 0 {
            unsafe {
                XPLMSetDataf(self.handle, xplane_value as f32);
            }
        } else if ty & xplmType_Int != 0 {
            unsafe {
                XPLMSetDatai(self.handle, xplane_value as i32);
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

    /// Evaluate the mapping and return the FSUIPC value, or None if any required
    /// dataref is missing.
    pub fn read_xplane(&self) -> Option<f64> {
        match &self.source {
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

                match update_if_expr {
                    Some(c) => {
                        if c.eval(&vars) > 0.0 {
                            Some(expr.eval(&vars))
                        } else {
                            None
                        }
                    }
                    None => Some(expr.eval(&vars)),
                }
            }
            ResolvedSource::Static { static_value } => *static_value,
            ResolvedSource::StaticStr { .. } => None,
        }
    }

    /// Write a value back to X-Plane (simple mappings only; expr write-back
    /// requires knowledge of which dataref to write and the inverse expression,
    /// which is not yet supported).
    pub fn read_xplane_value(&self) -> Option<Value> {
        match self.fsuipc_type {
            FsuipcType::String => {
                let bytes = match &self.source {
                    ResolvedSource::Simple { dr, .. } => dr.read_bytes(self.size)?,
                    ResolvedSource::StaticStr { static_str } => {
                        let mut b = static_str.as_bytes().to_vec();
                        b.push(0);
                        b
                    }
                    _ => return None,
                };
                Some(Value::String(bytes))
            }
            _ => self
                .read_xplane()
                .map(|v| f64_to_value(v, self.fsuipc_type)),
        }
    }

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
            table.clear_active_and_writable();
            for m in &self.mappings {
                if let Some(value) = m.read_xplane_value() {
                    table.insert(
                        m.offset,
                        ipc_host::value_table::Entry {
                            value,
                            source: 0,
                            destination: 0,
                            writable: m.writable,
                        },
                    );
                }
            }
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
