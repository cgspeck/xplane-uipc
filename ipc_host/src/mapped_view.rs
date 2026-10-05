//! Parsing and answering the FSUIPC request stream in a client's file mapping.
//!
//! The wire format follows the FSUIPC SDK client libraries (`IPCuser.c` for
//! 32-bit clients, `FSUIPCuser64.c` for 64-bit ones). Records sit back to back,
//! each a header followed by `nBytes` of payload, and the first DWORD of each
//! header (`dwId`) gives its layout:
//!
//! | dwId | Kind   | Header                                | Payload at |
//! |------|--------|---------------------------------------|------------|
//! | 1    | Read32 | `[dwId][dwOffset][nBytes][pDest:4]`   | +16        |
//! | 4    | Read64 | `[dwId][dwOffset][nBytes][pDest:8]`   | +20        |
//! | 2    | Write  | `[dwId][dwOffset][nBytes]`            | +12        |
//! | 0    | end of stream                                             |
//!
//! `pDest` is the client's own destination pointer: the client copies each
//! read's payload there after the call, so it means nothing to the server.

use byteorder::{ByteOrder, LittleEndian};
use std::collections::HashSet;
use std::fmt;
use std::slice;
use tracing::level_filters::LevelFilter;

use crate::{
    log_at,
    runtime_level::RuntimeLevel,
    try_send_write,
    value_table::{Table, Value},
    warning::{WarnCategory, WarnedSet},
};

/// `dwId` of a read from a 32-bit client.
pub const FS6IPC_READSTATEDATA_ID: u32 = 1;
/// `dwId` of a write, from either client.
pub const FS6IPC_WRITESTATEDATA_ID: u32 = 2;
/// `dwId` of a read from a 64-bit client.
pub const F64IPC_READSTATEDATA_ID: u32 = 4;

/// Offset clients such as FSInterrogate write their application key to when
/// connecting (FSUIPC's access registration). It is accepted and logged, never
/// forwarded to the flight loop.
pub const APP_KEY_OFFSET: u16 = 0x8001;

/// Offset clients write the parameter for the next macro or Lua request to.
pub const LUA_PARAM_OFFSET: u16 = 0x0D6C;

/// Offset clients write a macro or Lua request to, for FSUIPC to run. We can't
/// run them, so requests are logged, never forwarded.
pub const LUA_REQUEST_OFFSET: u16 = 0x0D70;

/// Size of the request text area at `LUA_REQUEST_OFFSET`.
const LUA_REQUEST_LEN: usize = 40;

/// Level application key writes to `APP_KEY_OFFSET` are logged at.
static KEY_WRITE_LOG_LEVEL: RuntimeLevel = RuntimeLevel::new();

/// Set the level application key writes to `APP_KEY_OFFSET` are logged at.
/// `LevelFilter::OFF` silences them.
pub fn set_key_write_log_level(level: LevelFilter) {
    KEY_WRITE_LOG_LEVEL.set(level);
}

/// Level macro and Lua requests written to `LUA_REQUEST_OFFSET` are logged at.
static LUA_REQUEST_LOG_LEVEL: RuntimeLevel = RuntimeLevel::new();

/// Set the level macro and Lua requests written to `LUA_REQUEST_OFFSET` are
/// logged at. `LevelFilter::OFF` silences them.
pub fn set_lua_request_log_level(level: LevelFilter) {
    LUA_REQUEST_LOG_LEVEL.set(level);
}

/// Text written by a client: bytes up to the first NUL, with anything
/// unprintable escaped.
fn client_text(payload: &[u8]) -> String {
    let end = payload
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(payload.len());
    payload[..end].escape_ascii().to_string()
}

fn log_key_write(payload: &[u8]) {
    log_at!(
        KEY_WRITE_LOG_LEVEL,
        "Application key write: \"{}\"",
        client_text(payload)
    );
}

/// The parameter written to `LUA_PARAM_OFFSET`: up to 4 bytes, little-endian,
/// zero-extended.
fn lua_param(payload: &[u8]) -> u32 {
    let mut bytes = [0u8; 4];
    let len = payload.len().min(bytes.len());
    bytes[..len].copy_from_slice(&payload[..len]);
    u32::from_le_bytes(bytes)
}

/// The request text written to `LUA_REQUEST_OFFSET`, limited to its 40 bytes.
fn lua_request_text(payload: &[u8]) -> String {
    client_text(&payload[..payload.len().min(LUA_REQUEST_LEN)])
}

/// Log a request at `LUA_REQUEST_LOG_LEVEL` the first time its text and
/// parameter are seen, so clients that repeat it every poll don't flood the
/// log. Repeats are logged at trace.
fn log_lua_request(payload: &[u8], state: &mut IpcState) {
    let text = lua_request_text(payload);
    let first = state
        .logged_lua_requests
        .insert((text.clone(), state.lua_param));
    let param = match state.lua_param {
        Some(p) => format!("param {}", p),
        None => "no param".to_string(),
    };
    if !first {
        tracing::trace!(
            "Lua/macro request \"{}\" ({}) repeated, already logged",
            text,
            param
        );
        return;
    }
    log_at!(
        LUA_REQUEST_LOG_LEVEL,
        "Lua/macro request \"{}\" ({}), not supported",
        text,
        param
    );
}

unsafe fn read_u32_at(ptr: *const u8) -> u32 {
    unsafe { LittleEndian::read_u32(slice::from_raw_parts(ptr, 4)) }
}

unsafe fn read_u64_at(ptr: *const u8) -> u64 {
    unsafe { LittleEndian::read_u64(slice::from_raw_parts(ptr, 8)) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordKind {
    Read32,
    Read64,
    Write,
}

impl RecordKind {
    fn from_id(id: u32) -> Option<Self> {
        match id {
            FS6IPC_READSTATEDATA_ID => Some(Self::Read32),
            F64IPC_READSTATEDATA_ID => Some(Self::Read64),
            FS6IPC_WRITESTATEDATA_ID => Some(Self::Write),
            _ => None,
        }
    }

    /// Size of this kind's header; the payload follows it.
    pub const fn header_len(self) -> usize {
        match self {
            Self::Read32 => 16,
            Self::Read64 => 20,
            Self::Write => 12,
        }
    }

    pub const fn is_write(self) -> bool {
        matches!(self, Self::Write)
    }
}

impl fmt::Display for RecordKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Read32 => "READ32",
            Self::Read64 => "READ64",
            Self::Write => "WRITE",
        })
    }
}

#[derive(Debug, Clone)]
pub struct ParsedRecord {
    pub kind: RecordKind,
    /// Byte offset of the record's header within the view.
    pub header_offset: usize,
    pub dw_offset: u32,
    pub n_bytes: u32,
    /// The client's destination pointer for a read; 0 for a write.
    pub p_dest: u64,
    pub payload_ptr: *mut u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MalformedReason {
    UnknownId(u32),
    HeaderOverrun(RecordKind),
    PayloadOverrun { dw_offset: u32, n_bytes: u32 },
    MissingTerminator,
}

/// Why and where parsing of a view stopped early.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Malformed {
    /// Byte offset within the view of the record that could not be parsed.
    pub at: usize,
    pub reason: MalformedReason,
}

impl fmt::Display for Malformed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.reason {
            MalformedReason::UnknownId(id) => {
                write!(f, "unknown dwId {:#010x} at {:#06x}", id, self.at)
            }
            MalformedReason::HeaderOverrun(kind) => {
                write!(f, "{} header at {:#06x} runs past the view", kind, self.at)
            }
            MalformedReason::PayloadOverrun { dw_offset, n_bytes } => write!(
                f,
                "record at {:#06x} (dwOffset={:#06x}, nBytes={}) runs past the view",
                self.at, dw_offset, n_bytes
            ),
            MalformedReason::MissingTerminator => {
                write!(f, "view ends at {:#06x} without a terminator", self.at)
            }
        }
    }
}

/// Iterate over the records in a mapped view, calling `on_record` for each,
/// until the zero `dwId` terminator.
///
/// Parsing stops at the first record that can't be framed: an unknown `dwId`,
/// a header or payload past the end of the view, or no terminator. Records
/// before it have already been passed to `on_record`.
///
/// # Safety
///
/// `mapped_view_ptr` must point to `view_size` bytes that stay valid for the
/// whole call. Each record's `payload_ptr` points into that buffer, so the
/// buffer must also be writable if `on_record` writes through it.
pub unsafe fn iterate_records<F>(
    mapped_view_ptr: *const u8,
    view_size: usize,
    mut on_record: F,
) -> Result<(), Malformed>
where
    F: FnMut(ParsedRecord),
{
    let mut pos = 0usize;
    loop {
        let malformed = |reason| Err(Malformed { at: pos, reason });
        let remaining = view_size - pos;
        if remaining < 4 {
            return malformed(MalformedReason::MissingTerminator);
        }
        // SAFETY: pos + 4 <= view_size, and the caller guarantees the view is valid
        let header = unsafe { mapped_view_ptr.add(pos) };
        let id = unsafe { read_u32_at(header) };
        if id == 0 {
            return Ok(());
        }
        let Some(kind) = RecordKind::from_id(id) else {
            return malformed(MalformedReason::UnknownId(id));
        };
        if remaining < kind.header_len() {
            return malformed(MalformedReason::HeaderOverrun(kind));
        }
        // SAFETY: the whole header lies within the view (checked above)
        let (dw_offset, n_bytes, p_dest) = unsafe {
            (
                read_u32_at(header.add(4)),
                read_u32_at(header.add(8)),
                match kind {
                    RecordKind::Read32 => read_u32_at(header.add(12)) as u64,
                    RecordKind::Read64 => read_u64_at(header.add(12)),
                    RecordKind::Write => 0,
                },
            )
        };
        if n_bytes as usize > remaining - kind.header_len() {
            return malformed(MalformedReason::PayloadOverrun { dw_offset, n_bytes });
        }
        tracing::trace!(
            "{} record at {:#06x}: dwOffset={:#06x}, nBytes={}, pDest={:#x}",
            kind,
            pos,
            dw_offset,
            n_bytes,
            p_dest
        );
        on_record(ParsedRecord {
            kind,
            header_offset: pos,
            dw_offset,
            n_bytes,
            p_dest,
            // SAFETY: the payload lies within the view (checked above)
            payload_ptr: unsafe { header.add(kind.header_len()) } as *mut u8,
        });
        pos += kind.header_len() + n_bytes as usize;
    }
}

/// What happened while answering a view.
#[derive(Debug, Default)]
pub struct ProcessOutcome {
    /// Set when parsing stopped early. Records before that point were handled.
    pub malformed: Option<Malformed>,
    /// Write requests that were refused because their size didn't fit the offset's type.
    pub rejected_writes: usize,
}

/// State the IPC window keeps between client messages. Owned by the window and
/// used only on the IPC thread.
pub struct IpcState {
    /// Offsets already warned about, so each warning is logged once.
    pub(crate) warned: WarnedSet,
    /// Last parameter written to `LUA_PARAM_OFFSET`, if any. Kept after use,
    /// as FSUIPC reuses it for later requests.
    pub(crate) lua_param: Option<u32>,
    /// Lua/macro requests (text and parameter) already logged, so each is
    /// logged once.
    pub(crate) logged_lua_requests: HashSet<(String, Option<u32>)>,
}

impl IpcState {
    pub fn new() -> Self {
        Self {
            warned: WarnedSet::new(),
            lua_param: None,
            logged_lua_requests: HashSet::new(),
        }
    }

    /// Forget which offsets have been warned about and which Lua/macro
    /// requests have been logged. The Lua parameter is kept.
    pub fn reset_warnings(&mut self) {
        self.warned.clear_all();
        self.logged_lua_requests.clear();
    }
}

impl Default for IpcState {
    fn default() -> Self {
        Self::new()
    }
}

/// Answer the read and write requests in a client's mapped view.
///
/// # Safety
///
/// `mapped_view_ptr` must point to `view_size` readable and writable bytes
/// that stay valid for the whole call: read requests are answered by writing
/// into the record payloads in place.
pub unsafe fn process_mapped_view(
    mapped_view_ptr: *const u8,
    view_size: usize,
    table: &Table,
    state: &mut IpcState,
) -> ProcessOutcome {
    let mut rejected_writes = 0;
    // SAFETY: caller guarantees mapped_view_ptr..+view_size is valid and writable
    let result = unsafe {
        iterate_records(mapped_view_ptr, view_size, |record| {
            // SAFETY: iterate_records only yields payloads that lie within the view
            let payload = slice::from_raw_parts_mut(record.payload_ptr, record.n_bytes as usize);
            if record.kind.is_write() {
                if !apply_write(&record, payload, table, state) {
                    rejected_writes += 1;
                }
            } else {
                answer_read(&record, payload, table, &state.warned);
            }
        })
    };
    if let Err(malformed) = &result {
        tracing::warn!("Malformed request view: {}", malformed);
    }
    ProcessOutcome {
        malformed: result.err(),
        rejected_writes,
    }
}

fn answer_read(record: &ParsedRecord, payload: &mut [u8], table: &Table, warned_set: &WarnedSet) {
    // Like FSUIPC, always answer with bytes: whatever we don't serve reads as
    // zero rather than whatever the client last left in its buffer.
    payload.fill(0);
    let Ok(offset) = u16::try_from(record.dw_offset) else {
        tracing::debug!(
            "Ignoring read from out-of-range offset {:#x}",
            record.dw_offset
        );
        return;
    };
    let Some(entry) = table.get(offset) else {
        tracing::debug!(
            "Offset {:#06x} (size {} bytes) not found in table",
            offset,
            record.n_bytes
        );
        if warned_set.check_and_set(offset, WarnCategory::ReadNotExist) {
            tracing::warn!(
                "Read from offset {:#06x}, {} bytes not in table",
                offset,
                record.n_bytes
            );
        }
        return;
    };
    tracing::debug!("Offset {:#06x} found in table", offset);
    warned_set.clear_key(offset, WarnCategory::ReadNotExist);
    match &entry.value {
        Value::String(bytes) => {
            tracing::trace!(
                "Writing String ({} bytes) -> offset {:#06x}",
                bytes.len(),
                offset
            );
            let len = bytes.len().min(payload.len());
            payload[..len].copy_from_slice(&bytes[..len]);
        }
        value => {
            // Never write past the request's payload: a narrower read
            // gets the low-order (little-endian) bytes of the value.
            let bytes = value.to_le_bytes();
            let len = bytes.len().min(payload.len());
            tracing::trace!(
                "Writing {:?} ({} of {} bytes) -> offset {:#06x}",
                value,
                len,
                bytes.len(),
                offset
            );
            payload[..len].copy_from_slice(&bytes[..len]);
        }
    }
}

/// Forward a write request to the flight loop. Returns false if it was rejected.
fn apply_write(record: &ParsedRecord, payload: &[u8], table: &Table, state: &mut IpcState) -> bool {
    let Ok(offset) = u16::try_from(record.dw_offset) else {
        tracing::debug!(
            "Ignoring write to out-of-range offset {:#x}",
            record.dw_offset
        );
        return true;
    };
    tracing::debug!(
        "Write operation: offset {:#06x}, n_bytes {}",
        offset,
        record.n_bytes
    );
    match offset {
        APP_KEY_OFFSET => {
            log_key_write(payload);
            return true;
        }
        LUA_PARAM_OFFSET => {
            let param = lua_param(payload);
            tracing::debug!("Lua/macro parameter set to {}", param);
            state.lua_param = Some(param);
            return true;
        }
        LUA_REQUEST_OFFSET => {
            log_lua_request(payload, state);
            return true;
        }
        _ => {}
    }
    let warned_set = &state.warned;
    let entry = table.get(offset).filter(|_| table.is_writable(offset));
    let Some(entry) = entry else {
        if table.is_active(offset)
            && warned_set.check_and_set(offset, WarnCategory::WriteNotWritable)
        {
            tracing::warn!("Attempt to write non-writable offset {:#06x}", offset);
        } else if warned_set.check_and_set(offset, WarnCategory::WriteNotExist) {
            tracing::warn!("Attempt to write non-active offset {:#06x}", offset);
        }
        return true;
    };
    match entry.value.decode_le(payload) {
        Some(value) => {
            tracing::info!("Write request: offset {:#06x} = {}", offset, value);
            try_send_write(offset, value, payload.len());
            true
        }
        None => {
            tracing::warn!(
                "Rejected write to offset {:#06x}: {} bytes does not match {:?}",
                offset,
                record.n_bytes,
                entry.value
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value_table::Entry;

    /// Builds a request view the way the SDK client libraries do.
    #[derive(Default)]
    struct View {
        data: Vec<u8>,
        /// Byte offset of each record's payload, in order.
        payloads: Vec<usize>,
    }

    impl View {
        fn header(mut self, id: u32, offset: u32, n_bytes: u32) -> Self {
            for v in [id, offset, n_bytes] {
                self.data.extend_from_slice(&v.to_le_bytes());
            }
            self
        }

        fn payload(mut self, bytes: &[u8]) -> Self {
            self.payloads.push(self.data.len());
            self.data.extend_from_slice(bytes);
            self
        }

        /// A 32-bit read; the client's payload buffer starts out as `0xAA`s.
        fn read32(self, offset: u32, n_bytes: u32) -> Self {
            let mut v = self.header(FS6IPC_READSTATEDATA_ID, offset, n_bytes);
            v.data.extend_from_slice(&0x0105_FFF8u32.to_le_bytes());
            v.payload(&vec![0xAA; n_bytes as usize])
        }

        /// A 64-bit read; the client's payload buffer starts out as `0xAA`s.
        fn read64(self, offset: u32, n_bytes: u32) -> Self {
            let mut v = self.header(F64IPC_READSTATEDATA_ID, offset, n_bytes);
            v.data
                .extend_from_slice(&0x0000_7FF6_1234_5678u64.to_le_bytes());
            v.payload(&vec![0xAA; n_bytes as usize])
        }

        fn write(self, offset: u32, bytes: &[u8]) -> Self {
            self.header(FS6IPC_WRITESTATEDATA_ID, offset, bytes.len() as u32)
                .payload(bytes)
        }

        /// Terminator, plus slack as in a real (page-sized) mapping.
        fn end(mut self) -> Self {
            self.data.extend_from_slice(&[0u8; 16]);
            self
        }

        fn payload_at(&self, record: usize, len: usize) -> &[u8] {
            let start = self.payloads[record];
            &self.data[start..start + len]
        }

        fn records(&self) -> (Vec<ParsedRecord>, Result<(), Malformed>) {
            let mut records = Vec::new();
            let result = unsafe {
                iterate_records(self.data.as_ptr(), self.data.len(), |r| records.push(r))
            };
            (records, result)
        }

        fn process(&mut self, table: &Table) -> ProcessOutcome {
            self.process_with(table, &mut IpcState::new())
        }

        /// Process with state that persists across calls, like the IPC window's.
        fn process_with(&mut self, table: &Table, state: &mut IpcState) -> ProcessOutcome {
            unsafe { process_mapped_view(self.data.as_mut_ptr(), self.data.len(), table, state) }
        }
    }

    fn table_with(entries: &[(u16, Value, bool)]) -> Table {
        let mut table = Table::new();
        for (offset, value, writable) in entries {
            table.insert(
                *offset,
                Entry {
                    value: value.clone(),
                    source: 0,
                    destination: 0,
                    writable: *writable,
                },
            );
        }
        table
    }

    #[test]
    fn test_narrow_read_does_not_overrun_payload() {
        let table = table_with(&[(0x300, Value::UnsignedInteger32(0x12345678), false)]);
        let mut view = View::default().read32(0x300, 2).read32(0x300, 4).end();
        let outcome = view.process(&table);
        assert!(outcome.malformed.is_none());
        assert_eq!(view.payload_at(0, 2), &[0x78, 0x56]);
        // The next record's header follows the 2-byte payload and must survive.
        let (records, result) = view.records();
        assert_eq!(result, Ok(()));
        assert_eq!(records.len(), 2);
        assert_eq!(view.payload_at(1, 4), &0x12345678u32.to_le_bytes());
    }

    /// All write-path checks share the global write channel, so keep them in one test.
    #[test]
    fn test_writes_decoded_by_entry_type() {
        let (tx, rx) = std::sync::mpsc::channel();
        crate::set_write_channel(tx);

        let table = table_with(&[
            (0x10, Value::Integer16(0), true),
            (0x20, Value::Float32(0.0), true),
            (0x30, Value::UnsignedInteger32(0), true),
            (0x40, Value::Integer64(0), true),
            // Read-only built-in, like the 0x337E activity counter.
            (0x337E, Value::UnsignedInteger16(7), false),
        ]);
        let run = |offset: u32, payload: &[u8]| {
            let outcome = View::default().write(offset, payload).end().process(&table);
            assert!(outcome.malformed.is_none());
            outcome.rejected_writes
        };

        assert_eq!(run(0x10, &[0xFF, 0xFF]), 0);
        let w = rx.try_recv().unwrap();
        assert_eq!((w.offset, w.value), (0x10, -1.0));

        assert_eq!(run(0x20, &1.5f32.to_le_bytes()), 0);
        assert_eq!(rx.try_recv().unwrap().value, 1.5);

        assert_eq!(run(0x40, &(-3i64).to_le_bytes()), 0);
        assert_eq!(rx.try_recv().unwrap().value, -3.0);

        // Size mismatch: rejected, counted, not forwarded.
        assert_eq!(run(0x30, &[1, 2]), 1);
        assert!(rx.try_recv().is_err());

        // Non-writable offset: not forwarded.
        assert_eq!(run(0x337E, &[1, 0]), 0);
        assert!(rx.try_recv().is_err());

        // Out-of-range offset: ignored, not forwarded and not wrapped onto 0x0010.
        assert_eq!(run(0x1_0010, &[1, 0]), 0);
        assert!(rx.try_recv().is_err());

        // Application key write: accepted, not forwarded, even if 0x8001 were mapped.
        let mut table = table;
        table.insert(
            APP_KEY_OFFSET,
            Entry {
                value: Value::String(vec![0; 13]),
                source: 0,
                destination: 0,
                writable: true,
            },
        );
        let outcome = View::default()
            .write(APP_KEY_OFFSET as u32, b"6PETEXPDRVW3\0")
            .end()
            .process(&table);
        assert!(outcome.malformed.is_none());
        assert_eq!(outcome.rejected_writes, 0);
        assert!(rx.try_recv().is_err());

        // Lua/macro parameter and request: accepted, not forwarded, even if mapped.
        for offset in [LUA_PARAM_OFFSET, LUA_REQUEST_OFFSET] {
            table.insert(
                offset,
                Entry {
                    value: Value::UnsignedInteger32(0),
                    source: 0,
                    destination: 0,
                    writable: true,
                },
            );
        }
        let outcome = View::default()
            .write(LUA_PARAM_OFFSET as u32, &3u32.to_le_bytes())
            .write(LUA_REQUEST_OFFSET as u32, b"LuaSet slc_doors\0")
            .end()
            .process(&table);
        assert!(outcome.malformed.is_none());
        assert_eq!(outcome.rejected_writes, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_lua_param_from_short_and_full_writes() {
        for (payload, expected) in [
            (&[3u8][..], 3),
            (&[0x34, 0x12][..], 0x1234),
            (&[0x78, 0x56, 0x34, 0x12][..], 0x1234_5678),
            (&[1, 0, 0, 0, 0xFF][..], 1),
        ] {
            let mut state = IpcState::new();
            View::default()
                .write(LUA_PARAM_OFFSET as u32, payload)
                .end()
                .process_with(&Table::new(), &mut state);
            assert_eq!(state.lua_param, Some(expected), "payload {:02x?}", payload);
        }
    }

    #[test]
    fn test_lua_param_persists_across_messages() {
        let mut state = IpcState::new();
        let table = Table::new();
        View::default()
            .write(LUA_PARAM_OFFSET as u32, &5u32.to_le_bytes())
            .end()
            .process_with(&table, &mut state);
        for request in [&b"LuaToggle slc_doors\0"[..], b"LuaSet slc_doors\0"] {
            View::default()
                .write(LUA_REQUEST_OFFSET as u32, request)
                .end()
                .process_with(&table, &mut state);
            assert_eq!(state.lua_param, Some(5));
        }
    }

    #[test]
    fn test_lua_request_text_cut_at_nul_and_40_bytes() {
        assert_eq!(
            lua_request_text(b"LuaKill slc_doors\0junk"),
            "LuaKill slc_doors"
        );
        let long = [b'x'; 48];
        assert_eq!(lua_request_text(&long), "x".repeat(40));
    }

    #[test]
    fn test_lua_offsets_not_warned_and_read_as_zero() {
        let mut state = IpcState::new();
        let mut view = View::default()
            .write(LUA_PARAM_OFFSET as u32, &3u32.to_le_bytes())
            .write(LUA_REQUEST_OFFSET as u32, b"LuaSet slc_doors\0")
            .read32(LUA_PARAM_OFFSET as u32, 4)
            .read32(LUA_REQUEST_OFFSET as u32, 40)
            .end();
        view.process_with(&Table::new(), &mut state);
        // check_and_set returns true when the offset hadn't been warned about yet.
        for offset in [LUA_PARAM_OFFSET, LUA_REQUEST_OFFSET] {
            assert!(
                state
                    .warned
                    .check_and_set(offset, WarnCategory::WriteNotExist)
            );
            assert!(
                state
                    .warned
                    .check_and_set(offset, WarnCategory::WriteNotWritable)
            );
        }
        assert_eq!(view.payload_at(2, 4), &[0; 4]);
        assert_eq!(view.payload_at(3, 40), &[0; 40]);
    }

    #[test]
    fn test_client_text() {
        assert_eq!(client_text(b"6PETEXPDRVW3\0"), "6PETEXPDRVW3");
        assert_eq!(client_text(b"ABC\0junk"), "ABC");
        assert_eq!(client_text(b"NO-NUL"), "NO-NUL");
        assert_eq!(client_text(b"A\x01B\0"), "A\\x01B");
    }

    #[test]
    fn test_process_empty_view() {
        let table = table_with(&[(0, Value::Integer64(42), false)]);
        let mut view = View {
            data: vec![0u8; 64],
            payloads: vec![],
        };
        let outcome = view.process(&table);
        assert!(outcome.malformed.is_none());
        assert_eq!(view.data, vec![0u8; 64]);
    }

    #[test]
    fn test_process_single_read_integer() {
        let table = table_with(&[(100, Value::Integer64(12345), false)]);
        let mut view = View::default().read32(100, 8).end();
        view.process(&table);
        assert_eq!(view.payload_at(0, 8), &12345i64.to_le_bytes());
    }

    #[test]
    fn test_process_single_read_float() {
        let table = table_with(&[(200, Value::Float64(1.23456), false)]);
        let mut view = View::default().read32(200, 8).end();
        view.process(&table);
        let read = f64::from_le_bytes(view.payload_at(0, 8).try_into().unwrap());
        assert!((read - 1.23456).abs() < 0.0001);
    }

    #[test]
    fn test_process_single_read_bool() {
        let table = table_with(&[(50, Value::Bool(true), false)]);
        let mut view = View::default().read32(50, 1).end();
        view.process(&table);
        assert_eq!(view.payload_at(0, 1), &[1]);
    }

    #[test]
    fn test_process_multiple_reads() {
        let table = table_with(&[
            (100, Value::Integer64(1000), false),
            (200, Value::Integer64(2000), false),
        ]);
        let mut view = View::default().read32(100, 8).read32(200, 8).end();
        view.process(&table);
        assert_eq!(view.payload_at(0, 8), &1000i64.to_le_bytes());
        assert_eq!(view.payload_at(1, 8), &2000i64.to_le_bytes());
    }

    #[test]
    fn test_offset_not_in_table() {
        let table = Table::new();
        let mut view = View::default().read32(100, 8).end();
        let outcome = view.process(&table);
        assert!(outcome.malformed.is_none());
        assert_eq!(outcome.rejected_writes, 0);
    }

    #[test]
    fn test_view_size_too_small_safe() {
        // Not even room for a dwId: no terminator, so the view is malformed.
        let data = [0u8; 2];
        let result = unsafe { iterate_records(data.as_ptr(), data.len(), |_| {}) };
        assert_eq!(
            result,
            Err(Malformed {
                at: 0,
                reason: MalformedReason::MissingTerminator
            })
        );
    }

    #[test]
    fn test_process_read_string_value() {
        let table = table_with(&[(300, Value::String(b"hello\0".to_vec()), false)]);
        let mut view = View::default().read32(300, 10).end();
        view.process(&table);
        assert_eq!(view.payload_at(0, 10), b"hello\0\0\0\0\0");
    }

    #[test]
    fn test_process_read_string_truncated_to_n_bytes() {
        let table = table_with(&[(400, Value::String(b"hello world\0".to_vec()), false)]);
        let mut view = View::default().read32(400, 5).end();
        view.process(&table);
        assert_eq!(view.payload_at(0, 5), b"hello");
    }

    #[test]
    fn test_nbytes_zero_accepted() {
        let view = View::default().read32(0x100, 0).read32(0x104, 4).end();
        let (records, result) = view.records();
        assert_eq!(result, Ok(()));
        assert_eq!(records.len(), 2);
        assert_eq!(records[1].dw_offset, 0x104);
    }

    #[test]
    fn test_nbytes_exceeds_buffer_rejected() {
        let mut view = View::default().header(FS6IPC_READSTATEDATA_ID, 0x100, 100);
        view.data.extend_from_slice(&[0u8; 20]);
        let (records, result) = view.records();
        assert!(records.is_empty());
        assert_eq!(
            result,
            Err(Malformed {
                at: 0,
                reason: MalformedReason::PayloadOverrun {
                    dw_offset: 0x100,
                    n_bytes: 100
                }
            })
        );
    }

    #[test]
    fn test_read64_framing() {
        let table = table_with(&[(0x0238, Value::UnsignedInteger16(0x1234), false)]);
        let mut view = View::default().read64(0x0238, 2).read32(0x0238, 2).end();
        let (records, result) = view.records();
        assert_eq!(result, Ok(()));
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].kind, RecordKind::Read64);
        assert_eq!(records[0].p_dest, 0x0000_7FF6_1234_5678);
        assert_eq!(view.payloads[0], 20);
        assert_eq!(records[1].header_offset, 22);

        view.process(&table);
        assert_eq!(view.payload_at(0, 2), &[0x34, 0x12]);
        assert_eq!(view.payload_at(1, 2), &[0x34, 0x12]);
    }

    #[test]
    fn test_mixed_read_write_stream() {
        let view = View::default()
            .read32(0x3304, 4)
            .write(0x0BC0, &[1, 0])
            .read32(0x3308, 4)
            .end();
        let (records, result) = view.records();
        assert_eq!(result, Ok(()));
        let summary: Vec<_> = records
            .iter()
            .map(|r| (r.kind, r.dw_offset, r.n_bytes))
            .collect();
        assert_eq!(
            summary,
            [
                (RecordKind::Read32, 0x3304, 4),
                (RecordKind::Write, 0x0BC0, 2),
                (RecordKind::Read32, 0x3308, 4),
            ]
        );
    }

    #[test]
    fn test_unknown_id_stops_after_earlier_records() {
        let table = table_with(&[(100, Value::Integer32(42), false)]);
        let mut view = View::default()
            .read32(100, 4)
            .header(7, 100, 4)
            .payload(&[0; 4])
            .end();
        let outcome = view.process(&table);
        assert_eq!(
            outcome.malformed,
            Some(Malformed {
                at: 20,
                reason: MalformedReason::UnknownId(7)
            })
        );
        // The read before the bad record was still answered.
        assert_eq!(view.payload_at(0, 4), &42i32.to_le_bytes());
    }

    #[test]
    fn test_header_overrun() {
        let mut view = View::default().read32(100, 0);
        view.data
            .extend_from_slice(&FS6IPC_READSTATEDATA_ID.to_le_bytes());
        view.data.extend_from_slice(&[0; 8]); // 12 of the 16 header bytes
        let (records, result) = view.records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            result,
            Err(Malformed {
                at: 16,
                reason: MalformedReason::HeaderOverrun(RecordKind::Read32)
            })
        );
    }

    #[test]
    fn test_missing_terminator() {
        let view = View::default().read32(100, 4);
        let (records, result) = view.records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            result,
            Err(Malformed {
                at: 20,
                reason: MalformedReason::MissingTerminator
            })
        );
    }

    #[test]
    fn test_out_of_range_read_is_not_malformed() {
        let mut view = View::default()
            .read32(0x1_0000, 4)
            .read32(0x1_0004, 4)
            .end();
        let outcome = view.process(&Table::new());
        assert!(outcome.malformed.is_none());
        let (records, _) = view.records();
        assert_eq!(records.len(), 2);
        assert_eq!(view.payload_at(0, 4), &[0; 4]);
    }

    #[test]
    fn test_unknown_offset_read_is_zero_filled() {
        let mut view = View::default().read32(100, 2).end();
        assert_eq!(view.payload_at(0, 2), &[0xAA, 0xAA]);
        view.process(&Table::new());
        assert_eq!(view.payload_at(0, 2), &[0, 0]);
    }

    #[test]
    fn test_wide_read_of_narrow_value_zero_fills_rest() {
        let table = table_with(&[(0x300, Value::UnsignedInteger16(0x1234), false)]);
        let mut view = View::default().read32(0x300, 4).end();
        view.process(&table);
        assert_eq!(view.payload_at(0, 4), &[0x34, 0x12, 0, 0]);
    }
}
