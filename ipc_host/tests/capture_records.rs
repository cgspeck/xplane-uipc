use ipc_host::mapped_view::iterate_records;

#[test]
fn test_working_slc_capture() {
    let data = include_bytes!("fixtures/slc-3-reads.bin");
    let mut records = Vec::new();
    let errors = unsafe {
        iterate_records(data.as_ptr(), data.len(), |rec| {
            records.push(rec);
            0
        })
    };
    assert_eq!(errors, 0);
    assert_eq!(records.len(), 3);
    assert!(records.iter().all(|r| r.sentinel_ok));
    assert_eq!(records[0].dw_offset, 0x3304);
    assert_eq!(records[0].n_bytes, 4);
    assert!(!records[0].is_write);
    assert_eq!(records[1].dw_offset, 0x3308);
    assert_eq!(records[1].n_bytes, 4);
    assert!(!records[1].is_write);
    assert_eq!(records[2].dw_offset, 0x3124);
    assert_eq!(records[2].n_bytes, 1);
    assert!(!records[2].is_write);
}

#[test]
fn test_fsinterrogate_capture() {
    let data = include_bytes!("fixtures/fsinterrogate-2-reads.bin");
    let mut records = Vec::new();
    let errors = unsafe {
        iterate_records(data.as_ptr(), data.len(), |rec| {
            records.push(rec);
            0
        })
    };
    assert_eq!(errors, 0);
    assert_eq!(records.len(), 2);
    assert!(records.iter().all(|r| !r.sentinel_ok));
    assert_eq!(records[0].dw_offset, 0x3304);
    assert_eq!(records[0].n_bytes, 4);
    assert!(!records[0].is_write);
    assert_eq!(records[1].dw_offset, 0x3308);
    assert_eq!(records[1].n_bytes, 4);
    assert!(!records[1].is_write);
}
