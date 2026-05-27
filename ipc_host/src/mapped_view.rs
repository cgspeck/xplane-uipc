use byteorder::{ByteOrder, LittleEndian};
use std::collections::HashSet;
use std::slice;

use std::sync::{LazyLock, Mutex};

use crate::{
    try_send_write,
    value_table::{Table, Value},
    warning::{WarnCategory, WarnedSet},
};

const SENTINEL: u32 = 0x5061_756C; // "luaP" LE
const FSD_SENTINEL: u32 = 0x4453_463A; // ":FSD" LE

static LOGGED_SENTINEL_VALUES: LazyLock<Mutex<HashSet<u32>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
static FSD_LOGGED_TEXTS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

pub fn reset_logged_sentinels() {
    LOGGED_SENTINEL_VALUES.lock().unwrap().clear();
    FSD_LOGGED_TEXTS.lock().unwrap().clear();
}

unsafe fn read_u32_at(ptr: *const u8) -> u32 {
    LittleEndian::read_u32(slice::from_raw_parts(ptr, 4))
}

/// From a zero reqID, scan forward one byte at a time looking for a valid
/// record header (sentinel at +12). If found within `max_gap` bytes, this
/// is a padding gap and we return how many bytes to skip. Otherwise it's
/// a true terminator and we return None.
unsafe fn find_next_record(cur_ptr: *const u8, max_gap: usize) -> Option<usize> {
    for offset in 0..=max_gap {
        let b0 = *cur_ptr.add(offset + 12);
        if b0 != 0x6C {
            continue;
        }
        let b1 = *cur_ptr.add(offset + 13);
        if b1 != 0x75 {
            continue;
        }
        let b2 = *cur_ptr.add(offset + 14);
        if b2 != 0x61 {
            continue;
        }
        let b3 = *cur_ptr.add(offset + 15);
        if b3 != 0x50 {
            continue;
        }

        return Some(offset);
    }
    None
}

#[derive(Debug, Clone)]
pub struct ParsedRecord {
    pub req_id: u32,
    pub dw_offset: u32,
    pub raw_n: u32,
    pub n_bytes: u32,
    pub is_write: bool,
    pub sentinel_ok: bool,
    pub sentinel_offset: usize,
    /// If sentinel_ok is false and recovery was found, the byte offset of
    /// the next valid record header. None if end of data.
    pub recovery_next_offset: Option<usize>,
    pub payload_ptr: *const u8,
}

/// Iterate over records in a mapped view buffer, calling `on_record` for each.
/// Returns the total number of errors encountered (bad sentinels, plus any
/// additional errors returned by the callback).
pub unsafe fn iterate_records<F>(
    mapped_view_ptr: *const u8,
    view_size: usize,
    mut on_record: F,
) -> usize
where
    F: FnMut(ParsedRecord) -> usize,
{
    let mut cur_ptr: *const u8 = mapped_view_ptr;
    let mut error_count = 0;

    loop {
        // ── 1. reqID ──────────────────────────────────────────────────────
        let req_id = read_u32_at(cur_ptr);
        tracing::trace!("reqID: {:#010x} @ {:p}", req_id, cur_ptr);

        if req_id == 0 {
            tracing::trace!(
                "Zero reqID at {:p}, scanning for next record header",
                cur_ptr
            );
            match find_next_record(cur_ptr, 16) {
                Some(0) => {
                    tracing::trace!("reqID is legitimately zero, parsing as normal record");
                }
                Some(skip) => {
                    tracing::trace!("Padding gap of {} bytes at {:p}, advancing", skip, cur_ptr);
                    cur_ptr = cur_ptr.add(skip);
                    continue;
                }
                None => {
                    tracing::trace!("Zero reqID at {:p} — true terminator, done", cur_ptr);
                    break;
                }
            }
        }

        cur_ptr = cur_ptr.add(4);

        // ── 2. dwOffset ───────────────────────────────────────────────────
        let dw_offset = read_u32_at(cur_ptr);
        tracing::trace!(
            "dwOffset: {:#07x} ({}) @ {:p}",
            dw_offset,
            dw_offset,
            cur_ptr
        );
        cur_ptr = cur_ptr.add(4);

        // ── 3. nBytes ─────────────────────────────────────────────────────
        let raw_n = read_u32_at(cur_ptr);
        let is_write = (raw_n & 0x8000_0000) != 0;
        let n_bytes = raw_n & 0x7FFF_FFFF;
        tracing::trace!(
            "nBytes raw: {:#010x}, is_write: {}, n_bytes: {}",
            raw_n,
            is_write,
            n_bytes
        );
        cur_ptr = cur_ptr.add(4);

        // ── 4. Sentinel ───────────────────────────────────────────────────
        let sentinel_before_ptr = cur_ptr;
        let sentinel = read_u32_at(cur_ptr);
        let sentinel_ok = sentinel == SENTINEL;
        let sentinel_offset = cur_ptr.offset_from(mapped_view_ptr) as usize;

        if !sentinel_ok {
            tracing::debug!(
                "Bad sentinel: reqID={:#010x}, dwOffset={:#07x}, nBytes={} at offset {:#x}, scanning forward",
                req_id,
                dw_offset,
                n_bytes,
                sentinel_offset
            );
            error_count += 1;

            // ── Once-logging of unrecognised sentinel values ─────────────
            {
                let mut logged = LOGGED_SENTINEL_VALUES.lock().unwrap();
                if logged.insert(sentinel) {
                    tracing::warn!(
                        "Bad sentinel at offset {:#x}: value {:#010x}",
                        sentinel_offset,
                        sentinel
                    );
                }
            }

            // ── Once-logging of ":FSD" trailing text ────────────────────
            if sentinel == FSD_SENTINEL {
                let mut text = String::new();
                for i in 0..255usize {
                    let c = *sentinel_before_ptr.add(4 + i);
                    if c == 0 || c < 0x20 || c > 0x7E {
                        break;
                    }
                    text.push(c as char);
                }
                if !text.is_empty() && FSD_LOGGED_TEXTS.lock().unwrap().insert(text.clone()) {
                    tracing::info!(
                        "Bad sentinel at offset {:#x}: ':FSD' followed by: \"{}\"",
                        sentinel_offset,
                        text
                    );
                }
            }

            // ── Phase 1: scan 16 bytes ──────────────────────────────────
            let recovery_next_offset;
            match find_next_record(sentinel_before_ptr.add(1), 16) {
                Some(0) => {
                    recovery_next_offset = Some(sentinel_offset + 1);
                    cur_ptr = sentinel_before_ptr.add(1);
                }
                Some(skip) => {
                    recovery_next_offset = Some(sentinel_offset + 1 + skip);
                    cur_ptr = sentinel_before_ptr.add(1 + skip);
                }
                None => {
                    // ── Phase 2: scan the rest of the buffer ────────────
                    let remaining = view_size.saturating_sub(sentinel_offset + 1 + 12);
                    match find_next_record(sentinel_before_ptr.add(1), remaining) {
                        Some(0) => {
                            recovery_next_offset = Some(sentinel_offset + 1);
                            cur_ptr = sentinel_before_ptr.add(1);
                        }
                        Some(skip) => {
                            recovery_next_offset = Some(sentinel_offset + 1 + skip);
                            cur_ptr = sentinel_before_ptr.add(1 + skip);
                        }
                        None => {
                            let record = ParsedRecord {
                                req_id,
                                dw_offset,
                                raw_n,
                                n_bytes,
                                is_write,
                                sentinel_ok: false,
                                sentinel_offset,
                                recovery_next_offset: None,
                                payload_ptr: std::ptr::null(),
                            };
                            error_count += on_record(record);
                            break;
                        }
                    }
                }
            }
            let record = ParsedRecord {
                req_id,
                dw_offset,
                raw_n,
                n_bytes,
                is_write,
                sentinel_ok: false,
                sentinel_offset,
                recovery_next_offset,
                payload_ptr: std::ptr::null(),
            };
            error_count += on_record(record);
            continue;
        }
        cur_ptr = cur_ptr.add(4);

        // ── 5. Payload ────────────────────────────────────────────────────
        let payload_ptr = cur_ptr;

        let record = ParsedRecord {
            req_id,
            dw_offset,
            raw_n,
            n_bytes,
            is_write,
            sentinel_ok: true,
            sentinel_offset,
            recovery_next_offset: None,
            payload_ptr,
        };
        error_count += on_record(record);

        // ── 6. Advance past payload ───────────────────────────────────────
        cur_ptr = cur_ptr.add(n_bytes as usize);
    }

    error_count
}

pub unsafe fn process_mapped_view(
    mapped_view_ptr: *const u8,
    view_size: usize,
    table: &Table,
    warned_set: &mut WarnedSet,
) -> usize {
    iterate_records(mapped_view_ptr, view_size, |record| {
        let mut record_errors = 0;

        if !record.is_write {
            if let Some(entry) = table.get(record.dw_offset as u16) {
                tracing::debug!("Offset {:#07x} found in table", record.dw_offset);
                warned_set.clear_key(record.dw_offset as u16, WarnCategory::ReadNotExist);
                match &entry.value {
                    Value::Integer64(v) => {
                        tracing::trace!("Writing i64 {} -> offset {:#07x}", v, record.dw_offset);
                        std::ptr::write_unaligned(record.payload_ptr as *mut i64, v.to_le());
                    }
                    Value::Float64(v) => {
                        tracing::trace!("Writing f64 {} -> offset {:#07x}", v, record.dw_offset);
                        std::ptr::write_unaligned(record.payload_ptr as *mut f64, *v)
                    }
                    Value::Bool(v) => {
                        tracing::trace!("Writing bool {} -> offset {:#07x}", v, record.dw_offset);
                        std::ptr::write_unaligned(record.payload_ptr as *mut u8, *v as u8);
                    }
                    Value::UnsignedInteger32(v) => {
                        tracing::trace!("Writing u32 {} -> offset {:#07x}", v, record.dw_offset);
                        std::ptr::write_unaligned(record.payload_ptr as *mut u32, v.to_le());
                    }
                    Value::UnsignedInt8(v) => {
                        tracing::trace!("Writing u8 {} -> offset {:#07x}", v, record.dw_offset);
                        std::ptr::write_unaligned(record.payload_ptr as *mut u8, v.to_le());
                    }
                    Value::UnsignedInt16(v) => {
                        tracing::trace!("Writing u16 {} -> offset {:#07x}", v, record.dw_offset);
                        std::ptr::write_unaligned(record.payload_ptr as *mut u16, v.to_le());
                    }
                }
            } else {
                tracing::debug!(
                    "Offset {:#07x} (size {} bytes) not found in table",
                    record.dw_offset,
                    record.n_bytes
                );
                if warned_set.check_and_set(record.dw_offset as u16, WarnCategory::ReadNotExist) {
                    tracing::warn!(
                        "Read from offset {:#07x}, {} bytes not in table",
                        record.dw_offset,
                        record.n_bytes
                    );
                }
            }
        } else {
            tracing::debug!(
                "Write operation: offset {:#07x}, n_bytes {}",
                record.dw_offset,
                record.n_bytes
            );
            if table.writable.contains(&(record.dw_offset as u16)) {
                let value = unsafe {
                    match record.n_bytes {
                        1 => (*record.payload_ptr) as f64,
                        2 => LittleEndian::read_u16(&*slice::from_raw_parts(record.payload_ptr, 2))
                            as f64,
                        4 => LittleEndian::read_u32(&*slice::from_raw_parts(record.payload_ptr, 4))
                            as f64,
                        8 => LittleEndian::read_f64(&*slice::from_raw_parts(record.payload_ptr, 8)),
                        _ => {
                            tracing::warn!(
                                "Unsupported write size: {}, incrementing error count",
                                record.n_bytes
                            );
                            record_errors += 1;
                            0.0
                        }
                    }
                };
                tracing::info!(
                    "Write request: offset {:#07x} = {}",
                    record.dw_offset,
                    value
                );
                try_send_write(record.dw_offset as u16, value, record.n_bytes as usize);
            } else {
                if table.active.contains(&(record.dw_offset as u16))
                    && warned_set
                        .check_and_set(record.dw_offset as u16, WarnCategory::WriteNotWritable)
                {
                    tracing::warn!(
                        "Attempt to write non-writable offset {:#07x}",
                        record.dw_offset
                    );
                } else if warned_set
                    .check_and_set(record.dw_offset as u16, WarnCategory::WriteNotExist)
                {
                    tracing::warn!(
                        "Attempt to write non-active offset {:#07x}",
                        record.dw_offset
                    );
                }
            }
        }

        record_errors
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value_table::Entry;

    fn create_test_table() -> Table {
        Table::new()
    }

    #[test]
    fn test_process_empty_view() {
        let mut table = create_test_table();
        table.insert(
            0,
            Entry {
                value: Value::Integer64(42),
                source: 0,
                destination: 0,
                writable: false,
            },
        );

        let mut warned_set = WarnedSet::new();
        let data = [0u8; 64];
        unsafe { process_mapped_view(data.as_ptr(), data.len(), &table, &mut warned_set) };
    }

    #[test]
    fn test_process_single_read_integer() {
        let mut table = create_test_table();
        table.insert(
            100,
            Entry {
                value: Value::Integer64(12345),
                source: 100,
                destination: 0,
                writable: false,
            },
        );

        let mut data = vec![0u8; 64];
        // sequence id
        data[0] = 1;
        data[1] = 0;
        data[2] = 0;
        data[3] = 0;
        // dwOffset = 100 (little-endian)
        data[4] = 100;
        data[5] = 0;
        data[6] = 0;
        data[7] = 0;
        // nBytes = 8 (read operation, high bit clear), little-endian
        data[8] = 8;
        data[9] = 0;
        data[10] = 0;
        data[11] = 0;
        // sentinel (0x5061756c "luaP" in LE)
        data[12] = 0x6C;
        data[13] = 0x75;
        data[14] = 0x61;
        data[15] = 0x50;
        // inline data area
        data[16] = 0;
        data[17] = 0;
        data[18] = 0;
        data[19] = 0;
        data[20] = 0;
        data[21] = 0;
        data[22] = 0;
        data[23] = 0;

        let mut warned_set = WarnedSet::new();
        unsafe { process_mapped_view(data.as_ptr(), data.len(), &table, &mut warned_set) };

        // Check that the value was written to the target location
        let read_value = i64::from_le_bytes([
            data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
        ]);
        assert_eq!(read_value, 12345);
    }

    #[test]
    fn test_process_single_read_float() {
        let mut table = create_test_table();
        table.insert(
            200,
            Entry {
                value: Value::Float64(3.14159),
                source: 200,
                destination: 0,
                writable: false,
            },
        );

        let mut data = vec![0u8; 64];
        data[0] = 1;
        data[4] = 200; // offset 200 (just lower byte for simplicity)
        data[8] = 8; // nBytes = 8
        // sentinel
        data[12] = 0x6C;
        data[13] = 0x75;
        data[14] = 0x61;
        data[15] = 0x50;
        let mut warned_set = WarnedSet::new();
        unsafe { process_mapped_view(data.as_ptr(), data.len(), &table, &mut warned_set) };

        // Check float was written
        let read_value = f64::from_le_bytes([
            data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
        ]);
        assert!((read_value - 3.14159).abs() < 0.0001);
    }

    #[test]
    fn test_process_single_read_bool() {
        let mut table = create_test_table();
        table.insert(
            50,
            Entry {
                value: Value::Bool(true),
                source: 50,
                destination: 0,
                writable: false,
            },
        );

        let mut data = vec![0u8; 64];
        data[0] = 1;
        data[4] = 50; // offset 50
        data[8] = 1; // nBytes = 1
        // sentinel
        data[12] = 0x6C;
        data[13] = 0x75;
        data[14] = 0x61;
        data[15] = 0x50;
        let mut warned_set = WarnedSet::new();
        unsafe { process_mapped_view(data.as_ptr(), data.len(), &table, &mut warned_set) };

        assert_eq!(data[16], 1u8);
    }

    #[test]
    fn test_process_multiple_reads() {
        let mut table = create_test_table();
        table.insert(
            100,
            Entry {
                value: Value::Integer64(1000),
                source: 100,
                destination: 0,
                writable: false,
            },
        );
        table.insert(
            200,
            Entry {
                value: Value::Integer64(2000),
                source: 200,
                destination: 0,
                writable: false,
            },
        );

        let mut data = vec![0u8; 128];
        // First record
        data[0] = 1;
        data[4] = 100;
        data[8] = 8;
        data[12] = 0x6C;
        data[13] = 0x75;
        data[14] = 0x61;
        data[15] = 0x50;
        // Second record
        data[24] = 2;
        data[28] = 200;
        data[32] = 8;
        data[36] = 0x6C;
        data[37] = 0x75;
        data[38] = 0x61;
        data[39] = 0x50;
        let mut warned_set = WarnedSet::new();
        unsafe { process_mapped_view(data.as_ptr(), data.len(), &table, &mut warned_set) };

        let val1 = i64::from_le_bytes([
            data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
        ]);
        let val2 = i64::from_le_bytes([
            data[40], data[41], data[42], data[43], data[44], data[45], data[46], data[47],
        ]);
        assert_eq!(val1, 1000);
        assert_eq!(val2, 2000);
    }

    #[test]
    fn test_offset_not_in_table() {
        let table = create_test_table();

        let mut data = vec![0u8; 64];
        data[0] = 1;
        data[4] = 100; // offset not in table
        data[8] = 8;
        let mut warned_set = WarnedSet::new();
        unsafe { process_mapped_view(data.as_ptr(), data.len(), &table, &mut warned_set) };
    }

    #[test]
    fn test_bad_sentinel_with_fsd_and_extended_recovery() {
        // Buffer layout:
        //   orphan record #1: reqID=2, offset=0xD6C, nBytes=4
        //   orphan record #2: reqID=2, offset=0xD70, nBytes=128 (write)
        //   ":FSDX_GSBOARDING_STATE\0"
        //   100 bytes of zeros
        //   valid record: reqID=1, offset=100, nBytes=8, sentinel=luaP
        let mut data = vec![0u8; 256];

        // Orphan record #1 (12 bytes)
        data[0..4].copy_from_slice(&2u32.to_le_bytes()); // reqID
        data[4..8].copy_from_slice(&0x0D6Cu32.to_le_bytes()); // offset
        data[8..12].copy_from_slice(&4u32.to_le_bytes()); // nBytes

        // Orphan record #2 (12 bytes)
        data[12..16].copy_from_slice(&2u32.to_le_bytes()); // reqID
        data[16..20].copy_from_slice(&0x0D70u32.to_le_bytes()); // offset
        data[20..24].copy_from_slice(&(128 | 0x8000_0000u32).to_le_bytes()); // nBytes (write)

        // ":FSD" sentinel at offset 24 (sentinel position of orphan #2)
        data[24] = b':';
        data[25] = b'F';
        data[26] = b'S';
        data[27] = b'D';
        // Trailing ASCII text
        let text = b"X_GSBOARDING_STATE";
        data[28..28 + text.len()].copy_from_slice(text);
        data[28 + text.len()] = 0; // null terminator

        // Zero padding (from ~53 to 152)
        // data is already zeroed

        // Valid record starting at offset 153
        data[153..157].copy_from_slice(&1u32.to_le_bytes()); // reqID
        data[157..161].copy_from_slice(&100u32.to_le_bytes()); // offset=100
        data[161..165].copy_from_slice(&8u32.to_le_bytes()); // nBytes=8
        data[165] = 0x6C;
        data[166] = 0x75;
        data[167] = 0x61;
        data[168] = 0x50; // luaP

        let mut table = create_test_table();
        table.insert(
            100,
            Entry {
                value: Value::Integer64(9999),
                source: 100,
                destination: 0,
                writable: false,
            },
        );

        let mut warned_set = WarnedSet::new();
        let error_count =
            unsafe { process_mapped_view(data.as_ptr(), data.len(), &table, &mut warned_set) };

        // One bad sentinel from orphan #1; orphan #2 is consumed as part of
        // orphan #1's header and never parsed independently. Phase 2 recovers
        // past both orphans + :FSD junk to reach the valid record.
        assert_eq!(error_count, 1);
        // The valid record at offset 153 should have been processed
        let read_val = i64::from_le_bytes(data[169..177].try_into().unwrap());
        assert_eq!(read_val, 9999);
    }

    #[test]
    fn test_fsd_no_text_after() {
        // ":FSD" followed immediately by null bytes — no text log expected
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(&1u32.to_le_bytes());
        data[4..8].copy_from_slice(&100u32.to_le_bytes());
        data[8..12].copy_from_slice(&8u32.to_le_bytes());
        // ":FSD" as sentinel
        data[12] = b':';
        data[13] = b'F';
        data[14] = b'S';
        data[15] = b'D';
        // rest is zeros — no printable text follows

        let table = create_test_table();
        let mut warned_set = WarnedSet::new();
        let error_count =
            unsafe { process_mapped_view(data.as_ptr(), data.len(), &table, &mut warned_set) };
        assert_eq!(error_count, 1);
    }

    #[test]
    fn test_view_size_too_small_safe() {
        // Tiny buffer where view_size prevents over-scanning
        let data = [0u8; 4];
        let table = create_test_table();
        let mut warned_set = WarnedSet::new();
        let error_count =
            unsafe { process_mapped_view(data.as_ptr(), data.len(), &table, &mut warned_set) };
        // Should handle safely — zero reqID at offset 0, scan forward finds nothing, break
        assert_eq!(error_count, 0);
    }
}
