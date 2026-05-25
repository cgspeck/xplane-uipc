#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::collections::HashMap;
use std::ffi::{CString, c_int};
use std::sync::{Arc, RwLock};

use ipc_host::value_table::{Table, get_value_table};
use uipc_mapping::Expr;
use uipc_mapping::FsuipcType;
pub use uipc_mapping::{DatarefMapping, MappingSource};
// use crate::shared_mem::SharedMem;
// use crate::xplane_sdk::{self, *};

// ─── Enumerations ───────────────────────────────────────────────────────────

pub const XPLM_TYPE_INT: XPLMDataTypeID = 1;
pub const XPLM_TYPE_FLOAT: XPLMDataTypeID = 2;
pub const XPLM_TYPE_DOUBLE: XPLMDataTypeID = 4;
pub const XPLM_TYPE_FLOAT_ARRAY: XPLMDataTypeID = 8;
pub const XPLM_TYPE_INT_ARRAY: XPLMDataTypeID = 16;
pub const XPLM_TYPE_DATA: XPLMDataTypeID = 32;

pub const XPLM_FLIGHT_LOOP_PHASE_BEFORE_FLIGHT_MODEL: XPLMFlightLoopPhaseType = 0;
pub const XPLM_FLIGHT_LOOP_PHASE_AFTER_FLIGHT_MODEL: XPLMFlightLoopPhaseType = 1;

// ─── Resolved dataref handle ──────────────────────────────────────────────────

#[derive(Clone)]
pub struct ResolvedRef {
    pub handle: XPLMDataRef,
    pub array_index: i32,
}

impl ResolvedRef {
    fn resolve(path: &str, array_index: i32) -> Self {
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
    pub fn read(&self) -> Option<f64> {
        if self.handle.is_null() {
            return None;
        }
        let ty = unsafe { XPLMGetDataRefTypes(self.handle) };
        let val = if ty & XPLM_TYPE_DOUBLE != 0 {
            unsafe { XPLMGetDatad(self.handle) }
        } else if ty & XPLM_TYPE_FLOAT != 0 {
            if self.array_index >= 0 {
                let mut v: f32 = 0.0;
                unsafe {
                    XPLMGetDatavf(self.handle, &mut v, self.array_index, 1);
                }
                v as f64
            } else {
                unsafe { XPLMGetDataf(self.handle) as f64 }
            }
        } else if ty & XPLM_TYPE_INT != 0 {
            if self.array_index >= 0 {
                let mut v: i32 = 0;
                unsafe {
                    XPLMGetDatavi(self.handle, &mut v, self.array_index, 1);
                }
                v as f64
            } else {
                unsafe { XPLMGetDatai(self.handle) as f64 }
            }
        } else {
            return None;
        };
        Some(val)
    }

    pub fn write(&self, xplane_value: f64) {
        if self.handle.is_null() {
            return;
        }
        let ty = unsafe { XPLMGetDataRefTypes(self.handle) };
        if ty & XPLM_TYPE_DOUBLE != 0 {
            unsafe {
                XPLMSetDatad(self.handle, xplane_value);
            }
        } else if ty & XPLM_TYPE_FLOAT != 0 {
            unsafe {
                XPLMSetDataf(self.handle, xplane_value as f32);
            }
        } else if ty & XPLM_TYPE_INT != 0 {
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
    Expr {
        /// name → resolved ref
        refs: HashMap<String, ResolvedRef>,
        expr: Expr,
    },
}

pub struct ResolvedMapping {
    pub offset: u16,
    pub fsuipc_type: FsuipcType,
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
            MappingSource::Expr { datarefs, expr } => {
                let refs = datarefs
                    .into_iter()
                    .map(|(name, (path, idx))| (name, ResolvedRef::resolve(&path, idx)))
                    .collect();
                ResolvedSource::Expr { refs, expr }
            }
            MappingSource::Static { static_value } => ResolvedSource::Static {
                static_value: Some(static_value),
            },
        };
        Self {
            offset: mapping.offset,
            fsuipc_type: mapping.fsuipc_type,
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
            ResolvedSource::Expr { refs, expr } => {
                let mut vars = HashMap::new();
                for (name, dr) in refs {
                    vars.insert(name.clone(), dr.read().unwrap_or(0.0));
                }
                Some(expr.eval(&vars))
            }
            ResolvedSource::Static { static_value } => *static_value,
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
    // pub shared_mem: SharedMem,
    pub config_path: String,
    pub update_rate: f64,
}

impl PluginState {
    pub fn new(
        mappings: Vec<ResolvedMapping>,
        // shared_mem: SharedMem,
        config_path: String,
        update_rate: f64,
    ) -> Self {
        let mut s = Self {
            mappings,
            // shared_mem,
            config_path,
            update_rate,
        };
        s.write_fsuipc_header();
        s
    }

    fn write_fsuipc_header(&mut self) {
        // let buf = self.shared_mem.as_buf_mut();
        // fsuipc_offsets::write_u16(buf, fsuipc_offsets::OFFSET_SIM_RUNNING, 1);
        // fsuipc_offsets::write_u32(buf, fsuipc_offsets::OFFSET_FSUIPC_VERSION, 0x0702);
        // fsuipc_offsets::write_u32(buf, fsuipc_offsets::OFFSET_FS_VERSION, 0x0B00);
    }

    pub fn populate_table(&mut self) {
        let table: Arc<RwLock<Table>> = get_value_table();
        if let Ok(mut table) = table.write() {
            for m in &self.mappings {
                if let Some(value) = m.read_xplane() {
                    let entry = match m.fsuipc_type {
                        FsuipcType::I8 | FsuipcType::U8 => {
                            ipc_host::value_table::Value::UnsignedInt8(value as u8)
                        }
                        FsuipcType::I16 | FsuipcType::U16 => {
                            ipc_host::value_table::Value::UnsignedInt16(value as u16)
                        }
                        FsuipcType::I32 | FsuipcType::U32 | FsuipcType::F32 => {
                            ipc_host::value_table::Value::UnsignedInteger32(value as u32)
                        }
                        FsuipcType::I64 | FsuipcType::U64 => {
                            ipc_host::value_table::Value::Integer64(value as i64)
                        }
                        FsuipcType::F64 => ipc_host::value_table::Value::Float64(value),
                    };
                    table.insert(
                        m.offset,
                        ipc_host::value_table::Entry {
                            value: entry,
                            source: 0,
                            destination: 0,
                            writable: m.writable,
                        },
                    );
                }
            }
            tracing::info!("Populated table with {} entries", table.active.len());
        }
    }

    pub fn update(&mut self) {
        let table: Arc<RwLock<Table>> = get_value_table();
        if let Ok(mut table) = table.write() {
            table.clear_active_and_writable();
            for m in &self.mappings {
                if let Some(value) = m.read_xplane() {
                    let entry = match m.fsuipc_type {
                        FsuipcType::I8 | FsuipcType::U8 => {
                            ipc_host::value_table::Value::UnsignedInt8(value as u8)
                        }
                        FsuipcType::I16 | FsuipcType::U16 => {
                            ipc_host::value_table::Value::UnsignedInt16(value as u16)
                        }
                        FsuipcType::I32 | FsuipcType::U32 | FsuipcType::F32 => {
                            ipc_host::value_table::Value::UnsignedInteger32(value as u32)
                        }
                        FsuipcType::I64 | FsuipcType::U64 => {
                            ipc_host::value_table::Value::Integer64(value as i64)
                        }
                        FsuipcType::F64 => ipc_host::value_table::Value::Float64(value),
                    };
                    table.insert(
                        m.offset,
                        ipc_host::value_table::Entry {
                            value: entry,
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
                tracing::debug!("Wrote value {} to offset {:#07x}", value, offset);
                return;
            }
        }
        tracing::warn!("No writable mapping found for offset {:#07x}", offset);
    }

    pub fn mark_stopped(&mut self) {
        // let buf = self.shared_mem.as_buf_mut();
        // fsuipc_offsets::write_u16(buf, fsuipc_offsets::OFFSET_SIM_RUNNING, 0);
    }
}
