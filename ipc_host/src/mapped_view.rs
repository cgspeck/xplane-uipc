use byteorder::{ByteOrder, LittleEndian};
use std::slice;

use crate::{
    try_send_write,
    value_table::{Table, Value},
    warning::{WarnCategory, WarnedSet},
};

const SENTINEL: u32 = 0x5061_756C; // "luaP" LE
const FSD_SENTINEL: u32 = 0x4453_463A; // ":FSD" LE

unsafe fn read_u32_at(ptr: *const u8) -> u32 {
    unsafe { LittleEndian::read_u32(slice::from_raw_parts(ptr, 4)) }
}

/// From a zero reqID, scan forward one byte at a time looking for a valid
/// record header (sentinel at +12). If found within `max_gap` bytes, this
/// is a padding gap and we return how many bytes to skip. Otherwise it's
/// a true terminator and we return None.
///
/// `available` is the number of bytes accessible from `cur_ptr` onwards.
/// The scan reads up to `offset + 15` so we stop when fewer than 16 bytes
/// remain at the candidate position.
unsafe fn find_next_record(cur_ptr: *const u8, max_gap: usize, available: usize) -> Option<usize> {
    tracing::trace!(
        "find_next_record: entry, cur_ptr: {:#?}, max_gap: {}, available: {}",
        cur_ptr,
        max_gap,
        available
    );
    // Need at least 16 bytes from cur_ptr + offset to read the sentinel at +12..+15
    let safe_limit = if available >= 16 {
        max_gap.min(available - 16)
    } else {
        tracing::trace!("find_next_record: early return none");
        return None;
    };
    for offset in 0..=safe_limit {
        // SAFETY: bounds checked by safe_limit calculation above
        unsafe {
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
        }
        tracing::trace!("find_next_record: returned offset {:#}", offset);
        return Some(offset);
    }
    tracing::trace!("find_next_record: full scan returned none");
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
    pub payload_ptr: *mut u8,
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
    // SAFETY: caller guarantees mapped_view_ptr..+view_size is valid and readable
    unsafe {
        let mut cur_ptr: *const u8 = mapped_view_ptr;
        let mut error_count = 0;
        let end_ptr = mapped_view_ptr.add(view_size);
        let avail = |p: *const u8| -> usize { end_ptr.offset_from(p).max(0) as usize };

        loop {
            // ── 1. reqID ──────────────────────────────────────────────────
            let req_id = read_u32_at(cur_ptr);
            tracing::trace!("reqID: {:#010x} @ {:p}", req_id, cur_ptr);

            if req_id == 0 {
                tracing::trace!(
                    "Zero reqID at {:p}, scanning for next record header",
                    cur_ptr
                );
                match find_next_record(cur_ptr, 16, avail(cur_ptr)) {
                    Some(0) => {
                        tracing::trace!("reqID is legitimately zero, parsing as normal record");
                    }
                    Some(skip) => {
                        tracing::trace!(
                            "Padding gap of {} bytes at {:p}, advancing",
                            skip,
                            cur_ptr
                        );
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

            // ── 2. dwOffset ───────────────────────────────────────────────
            let dw_offset = read_u32_at(cur_ptr);
            tracing::trace!(
                "dwOffset: {:#06x} ({}) @ {:p}",
                dw_offset,
                dw_offset,
                cur_ptr
            );
            cur_ptr = cur_ptr.add(4);

            // ── 3. nBytes ─────────────────────────────────────────────────
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

            // ── 4. Sentinel ───────────────────────────────────────────────
            let sentinel_before_ptr = cur_ptr;
            let sentinel = read_u32_at(cur_ptr);
            let sentinel_ok = sentinel == SENTINEL;
            let sentinel_offset = cur_ptr.offset_from(mapped_view_ptr) as usize;

            if !sentinel_ok {
                tracing::trace!(
                    "Non-luaP sentinel: reqID={:#010x}, dwOffset={:#06x}, nBytes={} at offset {:#x}, value {:#010x}",
                    req_id,
                    dw_offset,
                    n_bytes,
                    sentinel_offset,
                    sentinel
                );

                // ── Detect ":FSD" trailing text for diagnostics ────────
                if sentinel == FSD_SENTINEL {
                    let mut text = String::new();
                    for i in 0..255usize {
                        let c = *sentinel_before_ptr.add(4 + i);
                        if c == 0 || c < 0x20 || c > 0x7E {
                            break;
                        }
                        text.push(c as char);
                    }
                    if !text.is_empty() {
                        tracing::trace!(
                            "FSD sentinel at offset {:#x} followed by: \"{}\"",
                            sentinel_offset,
                            text
                        );
                    }
                }
            }
            cur_ptr = cur_ptr.add(4);

            // ── 5. Payload ────────────────────────────────────────────────
            let payload_ptr = cur_ptr as *mut u8;

            let record = ParsedRecord {
                req_id,
                dw_offset,
                raw_n,
                n_bytes,
                is_write,
                sentinel_ok,
                sentinel_offset,
                recovery_next_offset: None,
                payload_ptr,
            };
            error_count += on_record(record);

            // ── 6. Advance past payload ───────────────────────────────────
            cur_ptr = cur_ptr.add(n_bytes as usize);
        }

        error_count
    }
}

pub unsafe fn process_mapped_view(
    mapped_view_ptr: *const u8,
    view_size: usize,
    table: &Table,
    warned_set: &mut WarnedSet,
) -> usize {
    // SAFETY: caller guarantees mapped_view_ptr..+view_size is valid
    unsafe {
        iterate_records(mapped_view_ptr, view_size, |record| {
            let mut record_errors = 0;

            if !record.is_write {
                if let Some(entry) = table.get(record.dw_offset as u16) {
                    tracing::debug!("Offset {:#06x} found in table", record.dw_offset);
                    warned_set.clear_key(record.dw_offset as u16, WarnCategory::ReadNotExist);
                    match &entry.value {
                        Value::Bool(v) => {
                            tracing::trace!(
                                "Writing bool {} -> offset {:#06x}",
                                v,
                                record.dw_offset
                            );
                            std::ptr::write_unaligned(record.payload_ptr as *mut u8, *v as u8);
                        }
                        Value::Float32(v) => {
                            tracing::trace!(
                                "Writing f32 {} -> offset {:#06x}",
                                v,
                                record.dw_offset
                            );
                            std::ptr::write_unaligned(record.payload_ptr as *mut f32, *v)
                        }
                        Value::Float64(v) => {
                            tracing::trace!(
                                "Writing f64 {} -> offset {:#06x}",
                                v,
                                record.dw_offset
                            );
                            std::ptr::write_unaligned(
                                record.payload_ptr as *mut f64,
                                f64::from_bits(v.to_bits().to_le()),
                            )
                        }
                        Value::Integer8(v) => {
                            tracing::trace!("Writing i8 {} -> offset {:#06x}", v, record.dw_offset);
                            std::ptr::write_unaligned(record.payload_ptr as *mut i8, v.to_le());
                        }
                        Value::Integer16(v) => {
                            tracing::trace!(
                                "Writing i16 {} -> offset {:#06x}",
                                v,
                                record.dw_offset
                            );
                            std::ptr::write_unaligned(record.payload_ptr as *mut i16, v.to_le());
                        }
                        Value::Integer32(v) => {
                            tracing::trace!(
                                "Writing i32 {} -> offset {:#06x}",
                                v,
                                record.dw_offset
                            );
                            std::ptr::write_unaligned(record.payload_ptr as *mut i32, v.to_le());
                        }
                        Value::Integer64(v) => {
                            tracing::trace!(
                                "Writing i64 {} -> offset {:#06x}",
                                v,
                                record.dw_offset
                            );
                            std::ptr::write_unaligned(record.payload_ptr as *mut i64, v.to_le());
                        }
                        Value::UnsignedInteger8(v) => {
                            tracing::trace!("Writing u8 {} -> offset {:#06x}", v, record.dw_offset);
                            std::ptr::write_unaligned(record.payload_ptr as *mut u8, v.to_le());
                        }
                        Value::UnsignedInteger16(v) => {
                            tracing::trace!(
                                "Writing u16 {} -> offset {:#06x} ({:#?}, {} bytes)",
                                v,
                                record.dw_offset,
                                record.payload_ptr,
                                std::mem::size_of::<u16>()
                            );
                            std::ptr::write_unaligned(record.payload_ptr as *mut u16, v.to_le());
                        }
                        Value::UnsignedInteger32(v) => {
                            tracing::trace!(
                                "Writing u32 {} -> offset {:#06x}",
                                v,
                                record.dw_offset
                            );
                            std::ptr::write_unaligned(record.payload_ptr as *mut u32, v.to_le());
                        }
                        Value::UnsignedInteger64(v) => {
                            tracing::trace!(
                                "Writing u64 {} -> offset {:#06x}",
                                v,
                                record.dw_offset
                            );
                            std::ptr::write_unaligned(record.payload_ptr as *mut u64, v.to_le());
                        }
                        Value::String(bytes) => {
                            tracing::trace!(
                                "Writing String ({} bytes) -> offset {:#06x}",
                                bytes.len(),
                                record.dw_offset
                            );
                            let len = bytes.len().min(record.n_bytes as usize);
                            std::ptr::copy_nonoverlapping(bytes.as_ptr(), record.payload_ptr, len);
                            for i in len..record.n_bytes as usize {
                                *record.payload_ptr.add(i) = 0;
                            }
                        }
                    }
                } else {
                    tracing::debug!(
                        "Offset {:#06x} (size {} bytes) not found in table",
                        record.dw_offset,
                        record.n_bytes
                    );
                    if warned_set.check_and_set(record.dw_offset as u16, WarnCategory::ReadNotExist)
                    {
                        tracing::warn!(
                            "Read from offset {:#06x}, {} bytes not in table",
                            record.dw_offset,
                            record.n_bytes
                        );
                    }
                }
            } else {
                tracing::debug!(
                    "Write operation: offset {:#06x}, n_bytes {}",
                    record.dw_offset,
                    record.n_bytes
                );
                if table.writable.contains(&(record.dw_offset as u16)) {
                    let value = match record.n_bytes {
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
                    };
                    tracing::info!(
                        "Write request: offset {:#06x} = {}",
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
                            "Attempt to write non-writable offset {:#06x}",
                            record.dw_offset
                        );
                    } else if warned_set
                        .check_and_set(record.dw_offset as u16, WarnCategory::WriteNotExist)
                    {
                        tracing::warn!(
                            "Attempt to write non-active offset {:#06x}",
                            record.dw_offset
                        );
                    }
                }
            }
            record_errors
        })
    }
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
    fn test_fsd_sentinel_record_is_processed() {
        // A record with ":FSD" sentinel should be processed normally
        // (value written to payload, not treated as error).
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(&1u32.to_le_bytes()); // reqID
        data[4..8].copy_from_slice(&50u32.to_le_bytes()); // offset
        data[8..12].copy_from_slice(&8u32.to_le_bytes()); // nBytes=8 (read)
        data[12] = b':';
        data[13] = b'F';
        data[14] = b'S';
        data[15] = b'D'; // ":FSD" sentinel

        let mut table = create_test_table();
        table.insert(
            50,
            Entry {
                value: Value::Integer64(7777),
                source: 50,
                destination: 0,
                writable: false,
            },
        );

        let mut warned_set = WarnedSet::new();
        let error_count =
            unsafe { process_mapped_view(data.as_ptr(), data.len(), &table, &mut warned_set) };

        // No errors — record was processed despite FSD sentinel
        assert_eq!(error_count, 0);
        // Value was written to payload area (offset 16)
        let read_val = i64::from_le_bytes(data[16..24].try_into().unwrap());
        assert_eq!(read_val, 7777);
    }

    #[test]
    fn test_non_luap_sentinel_accepted() {
        // A record with a non-"luaP" sentinel (e.g. a pointer value like
        // FSInterrogate writes) should be processed without error.
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(&1u32.to_le_bytes()); // reqID
        data[4..8].copy_from_slice(&50u32.to_le_bytes()); // offset
        data[8..12].copy_from_slice(&4u32.to_le_bytes()); // nBytes=4 (read)
        // Non-"luaP" value — like a writeback pointer
        data[12..16].copy_from_slice(&0x0105FFF8u32.to_le_bytes());

        let mut table = create_test_table();
        table.insert(
            50,
            Entry {
                value: Value::Integer32(42),
                source: 50,
                destination: 0,
                writable: false,
            },
        );

        let mut warned_set = WarnedSet::new();
        let error_count =
            unsafe { process_mapped_view(data.as_ptr(), data.len(), &table, &mut warned_set) };

        assert_eq!(error_count, 0);
        let read_val = i32::from_le_bytes(data[16..20].try_into().unwrap());
        assert_eq!(read_val, 42);
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

    #[test]
    fn test_process_read_string_value() {
        let mut table = create_test_table();
        let bytes: Vec<u8> = b"hello\0".to_vec();
        table.insert(
            300,
            Entry {
                value: Value::String(bytes),
                source: 300,
                destination: 0,
                writable: false,
            },
        );

        let mut data = vec![0u8; 64];
        data[0] = 1; // reqID
        data[4] = 44; // offset 300 (0x012C), just low byte for simplicity
        data[5] = 1; // high byte of offset
        data[8] = 10; // nBytes = 10
        data[12] = 0x6C;
        data[13] = 0x75;
        data[14] = 0x61;
        data[15] = 0x50; // sentinel luaP

        let mut warned_set = WarnedSet::new();
        unsafe { process_mapped_view(data.as_ptr(), data.len(), &table, &mut warned_set) };

        // Payload starts at offset 16
        let payload: Vec<u8> = data[16..26].to_vec();
        assert_eq!(&payload[..6], b"hello\0");
        // Remaining bytes should be zero-filled
        assert_eq!(payload[6], 0);
        assert_eq!(payload[7], 0);
        assert_eq!(payload[8], 0);
        assert_eq!(payload[9], 0);
    }

    #[test]
    fn test_process_read_string_truncated_to_n_bytes() {
        let mut table = create_test_table();
        // String longer than n_bytes
        let bytes: Vec<u8> = b"hello world\0".to_vec();
        table.insert(
            400,
            Entry {
                value: Value::String(bytes),
                source: 400,
                destination: 0,
                writable: false,
            },
        );

        let mut data = vec![0u8; 64];
        data[0] = 1;
        data[4] = 144; // offset 400 (0x0190)
        data[5] = 1;
        data[8] = 5; // nBytes = 5 (smaller than string length)
        data[12] = 0x6C;
        data[13] = 0x75;
        data[14] = 0x61;
        data[15] = 0x50;

        let mut warned_set = WarnedSet::new();
        unsafe { process_mapped_view(data.as_ptr(), data.len(), &table, &mut warned_set) };

        let payload: Vec<u8> = data[16..21].to_vec();
        assert_eq!(&payload[..], b"hello");
    }
}
