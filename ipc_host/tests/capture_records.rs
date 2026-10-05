use ipc_host::mapped_view::{Malformed, ParsedRecord, RecordKind, iterate_records};

fn parse(data: &[u8]) -> (Vec<ParsedRecord>, Result<(), Malformed>) {
    let mut records = Vec::new();
    let result = unsafe { iterate_records(data.as_ptr(), data.len(), |rec| records.push(rec)) };
    (records, result)
}

#[test]
fn test_working_slc_capture() {
    let (records, result) = parse(include_bytes!("fixtures/slc-3-reads.bin"));
    assert_eq!(result, Ok(()));
    assert_eq!(records.len(), 3);
    assert!(records.iter().all(|r| r.kind == RecordKind::Read32));
    // The .NET client puts "Paul" in the pDest slot.
    assert!(records.iter().all(|r| r.p_dest == 0x5061_756C));
    assert_eq!(records[0].dw_offset, 0x3304);
    assert_eq!(records[0].n_bytes, 4);
    assert_eq!(records[1].dw_offset, 0x3308);
    assert_eq!(records[1].n_bytes, 4);
    assert_eq!(records[2].dw_offset, 0x3124);
    assert_eq!(records[2].n_bytes, 1);
}

#[test]
fn test_fsinterrogate_capture() {
    let (records, result) = parse(include_bytes!("fixtures/fsinterrogate-2-reads.bin"));
    assert_eq!(result, Ok(()));
    assert_eq!(records.len(), 2);
    assert!(records.iter().all(|r| r.kind == RecordKind::Read32));
    assert_eq!(records[0].dw_offset, 0x3304);
    assert_eq!(records[0].n_bytes, 4);
    assert_eq!(records[0].p_dest, 0x0105_FFF8);
    assert_eq!(records[1].dw_offset, 0x3308);
    assert_eq!(records[1].n_bytes, 4);
    assert_eq!(records[1].p_dest, 0x0105_FFFC);
}

#[test]
fn test_fsinterrogate_app_key_write() {
    // FSInterrogate registers its application key with a 13-byte write to 0x8001.
    let data = include_bytes!("fixtures/fsinterrogate-offset-8001.bin");
    let (records, result) = parse(data);
    assert_eq!(result, Ok(()));
    assert_eq!(records.len(), 1);
    let key = &records[0];
    assert_eq!(key.kind, RecordKind::Write);
    assert_eq!(key.header_offset, 0);
    assert_eq!(key.dw_offset, 0x8001);
    assert_eq!(key.n_bytes, 13);
    assert_eq!(key.payload_ptr as *const u8, data[0x0C..].as_ptr());
    assert_eq!(&data[0x0C..0x19], b"6PETEXPDRVW3\0");
}
