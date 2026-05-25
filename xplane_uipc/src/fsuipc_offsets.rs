//! FSUIPC shared memory layout and offset constants.
//!
//! FSUIPC exposes a 65536-byte (64 KiB) memory-mapped region called the
//! "FSUIPC data area".  External tools (e.g. LINDA, SPAD.neXt, Axis & Ohs)
//! read/write offsets into that region.
//!
//! This module defines the raw layout, the most common well-known offsets,
//! and the types used to read/write individual fields.

use std::mem;

/// Size of the FSUIPC shared-memory region in bytes (64 KiB).
pub const FSUIPC_DATA_SIZE: usize = 0x10000;

/// Name of the Win32 shared-memory object (Windows only).
pub const FSUIPC_SHM_NAME: &str = "FSUIPC_Data";

/// Byte offset of the "sim running" flag written by this plugin.
pub const OFFSET_SIM_RUNNING: u16 = 0x0264; // 2 bytes, non-zero = running
/// Version of FSUIPC that we emulate.  Reported as BCD e.g. 0x0702 = 7.02.
pub const OFFSET_FSUIPC_VERSION: u16 = 0x3304; // 4 bytes
/// FS version: 0x0C00 = MSFS, we report 0x0B00 (FSX-style) for compatibility.
pub const OFFSET_FS_VERSION: u16 = 0x3308; // 4 bytes

// ─── Well-known FSUIPC offsets ──────────────────────────────────────────────
// All offsets are from the FSUIPC SDK documentation (Pete Dowson).

/// Zulu time in milliseconds since midnight.
pub const OFFSET_ZULU_TIME_MS: u16 = 0x0238; // 4 bytes, i32
/// Local time in seconds since midnight.
pub const OFFSET_LOCAL_TIME_S: u16 = 0x023C; // 4 bytes, i32

/// Indicated airspeed in knots * 128.
pub const OFFSET_IAS: u16 = 0x02BC; // 4 bytes, i32  (knots * 128)
/// True airspeed in knots * 128.
pub const OFFSET_TAS: u16 = 0x02B8; // 4 bytes, i32
/// Ground speed in knots * 128.
pub const OFFSET_GS: u16 = 0x02B4; // 4 bytes, i32

/// Pitch angle: +ve nose up, degrees * 65536.
pub const OFFSET_PITCH: u16 = 0x0578; // 4 bytes, i32
/// Bank angle: +ve right wing down, degrees * 65536.
pub const OFFSET_BANK: u16 = 0x057C; // 4 bytes, i32
/// True heading, degrees * 65536.
pub const OFFSET_HEADING_TRUE: u16 = 0x0580; // 4 bytes, u32

/// Aircraft latitude, high-precision (degrees * 10^7, i64).
pub const OFFSET_LATITUDE: u16 = 0x0560; // 8 bytes, i64
/// Aircraft longitude, high-precision (degrees * 10^7, i64).
pub const OFFSET_LONGITUDE: u16 = 0x0568; // 8 bytes, i64
/// Altitude above mean sea level in feet * 65536.
pub const OFFSET_ALTITUDE_MSL: u16 = 0x0574; // 4 bytes, i32

/// Vertical speed in feet/min * 256.
pub const OFFSET_VSI: u16 = 0x02C8; // 4 bytes, i32

/// Barometer setting in millibars * 16.
pub const OFFSET_BARO_MB: u16 = 0x0330; // 2 bytes, u16

/// Autopilot master switch (non-zero = engaged).
pub const OFFSET_AP_MASTER: u16 = 0x07D0; // 2 bytes

/// NAV1 frequency in BCD (e.g. 11800 → 118.00 MHz).
pub const OFFSET_NAV1_FREQ: u16 = 0x0350; // 2 bytes
/// NAV2 frequency in BCD.
pub const OFFSET_NAV2_FREQ: u16 = 0x0352; // 2 bytes
/// COM1 frequency in BCD (e.g. 12200 → 122.00 MHz).
pub const OFFSET_COM1_FREQ: u16 = 0x034E; // 2 bytes
/// COM2 frequency in BCD.
pub const OFFSET_COM2_FREQ: u16 = 0x3118; // 2 bytes

/// Transponder code in BCD (e.g. 0x2100 = squawk 2100).
pub const OFFSET_XPDR_CODE: u16 = 0x0354; // 2 bytes

/// Fuel quantity – total, in pounds * 128 (summed across all tanks).
pub const OFFSET_FUEL_TOTAL_LBS: u16 = 0x0AF4; // 4 bytes, i32

/// Gear position: 0 = up, 16383 = down.
pub const OFFSET_GEAR_POS: u16 = 0x0BEC; // 4 bytes

/// Flap position as % * 100.
pub const OFFSET_FLAP_PCT: u16 = 0x0BE0; // 4 bytes

/// On-ground flag (non-zero = on ground).
pub const OFFSET_ON_GROUND: u16 = 0x0366; // 2 bytes

/// Ambient temperature at aircraft in degrees Celsius * 256.
pub const OFFSET_AMBIENT_TEMP: u16 = 0x0E8C; // 2 bytes, i16

/// Wind direction at aircraft (true, degrees * 65536).
pub const OFFSET_WIND_DIR: u16 = 0x0E90; // 2 bytes
/// Wind speed at aircraft in knots.
pub const OFFSET_WIND_SPEED: u16 = 0x0E92; // 2 bytes

// ─── Data types that can be stored at an offset ──────────────────────────────

/// The type of value stored at an FSUIPC offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsuipcType {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
}

impl FsuipcType {
    /// Number of bytes this type occupies in the shared region.
    pub fn size(self) -> usize {
        match self {
            Self::I8 | Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::I32 | Self::U32 | Self::F32 => 4,
            Self::I64 | Self::U64 | Self::F64 => 8,
        }
    }
}

impl std::str::FromStr for FsuipcType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "i8" => Ok(Self::I8),
            "u8" => Ok(Self::U8),
            "i16" => Ok(Self::I16),
            "u16" => Ok(Self::U16),
            "i32" => Ok(Self::I32),
            "u32" => Ok(Self::U32),
            "i64" => Ok(Self::I64),
            "u64" => Ok(Self::U64),
            "f32" => Ok(Self::F32),
            "f64" => Ok(Self::F64),
            other => Err(format!("unknown FSUIPC type '{}'", other)),
        }
    }
}

// ─── Write helpers ────────────────────────────────────────────────────────────

/// Write a value into the raw 64 KiB buffer at `offset`.
/// Returns Err if the write would exceed the buffer.
pub fn write_value(buf: &mut [u8; FSUIPC_DATA_SIZE], offset: u16, value: f64, ty: FsuipcType) {
    let off = offset as usize;
    let end = off + ty.size();
    if end > FSUIPC_DATA_SIZE {
        return; // silently ignore out-of-range
    }
    let dst = &mut buf[off..end];
    match ty {
        FsuipcType::I8 => dst.copy_from_slice(&(value as i8).to_le_bytes()),
        FsuipcType::U8 => dst.copy_from_slice(&(value as u8).to_le_bytes()),
        FsuipcType::I16 => dst.copy_from_slice(&(value as i16).to_le_bytes()),
        FsuipcType::U16 => dst.copy_from_slice(&(value as u16).to_le_bytes()),
        FsuipcType::I32 => dst.copy_from_slice(&(value as i32).to_le_bytes()),
        FsuipcType::U32 => dst.copy_from_slice(&(value as u32).to_le_bytes()),
        FsuipcType::I64 => dst.copy_from_slice(&(value as i64).to_le_bytes()),
        FsuipcType::U64 => dst.copy_from_slice(&(value as u64).to_le_bytes()),
        FsuipcType::F32 => dst.copy_from_slice(&(value as f32).to_le_bytes()),
        FsuipcType::F64 => dst.copy_from_slice(&value.to_le_bytes()),
    }
}

/// Read a value from the raw buffer at `offset`.
pub fn read_value(buf: &[u8; FSUIPC_DATA_SIZE], offset: u16, ty: FsuipcType) -> f64 {
    let off = offset as usize;
    let end = off + ty.size();
    if end > FSUIPC_DATA_SIZE {
        return 0.0;
    }
    let src = &buf[off..end];
    match ty {
        FsuipcType::I8 => i8::from_le_bytes(src.try_into().unwrap()) as f64,
        FsuipcType::U8 => u8::from_le_bytes(src.try_into().unwrap()) as f64,
        FsuipcType::I16 => i16::from_le_bytes(src.try_into().unwrap()) as f64,
        FsuipcType::U16 => u16::from_le_bytes(src.try_into().unwrap()) as f64,
        FsuipcType::I32 => i32::from_le_bytes(src.try_into().unwrap()) as f64,
        FsuipcType::U32 => u32::from_le_bytes(src.try_into().unwrap()) as f64,
        FsuipcType::I64 => i64::from_le_bytes(src.try_into().unwrap()) as f64,
        FsuipcType::U64 => u64::from_le_bytes(src.try_into().unwrap()) as f64,
        FsuipcType::F32 => f32::from_le_bytes(src.try_into().unwrap()) as f64,
        FsuipcType::F64 => f64::from_le_bytes(src.try_into().unwrap()),
    }
}

/// Write a 4-byte little-endian u32 (convenience).
pub fn write_u32(buf: &mut [u8; FSUIPC_DATA_SIZE], offset: u16, value: u32) {
    write_value(buf, offset, value as f64, FsuipcType::U32);
}

/// Write a 4-byte little-endian i32 (convenience).
pub fn write_i32(buf: &mut [u8; FSUIPC_DATA_SIZE], offset: u16, value: i32) {
    write_value(buf, offset, value as f64, FsuipcType::I32);
}

/// Write a 2-byte little-endian u16 (convenience).
pub fn write_u16(buf: &mut [u8; FSUIPC_DATA_SIZE], offset: u16, value: u16) {
    write_value(buf, offset, value as f64, FsuipcType::U16);
}
