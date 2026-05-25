use std::env;
use std::fs;

use ipc_host::mapped_view::iterate_records;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: capture-inspect <file.bin> [file2.bin ...]");
        std::process::exit(1);
    }

    let mut any_corrupted = false;

    for path in &args[1..] {
        let data = match fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Error reading '{}': {}", path, e);
                any_corrupted = true;
                continue;
            }
        };

        println!("File: {}", path);
        let line_len = path.len() + 6;
        println!("{}", "-".repeat(line_len));

        let mut record_num = 0u32;
        let error_count = unsafe {
            iterate_records(data.as_ptr(), data.len(), |record| {
                record_num += 1;
                if record.sentinel_ok {
                    let op = if record.is_write { "WRITE" } else { "READ" };
                    println!(
                        "#{:<4} reqID={:#010x}  offset={:#07x}  {}  {}B  ✓",
                        record_num, record.req_id, record.dw_offset, op, record.n_bytes
                    );
                } else {
                    any_corrupted = true;
                    if let Some(next) = record.recovery_next_offset {
                        println!(
                            "#{:<4} ── BAD SENTINEL @ {:#07x} ──  (scan +{} → next at {:#x})",
                            record_num,
                            record.sentinel_offset,
                            next - record.sentinel_offset,
                            next
                        );
                    } else {
                        println!(
                            "#{:<4} ── BAD SENTINEL @ {:#07x} ──  (no recovery, end of data)",
                            record_num, record.sentinel_offset,
                        );
                    }
                }
                0
            })
        };

        if error_count > 1 {
            // error_count includes bad-sentinel records counted in iterate_records
            // plus per-record errors (but we return 0, so it's just the sentinel count)
        }

        println!(
            "── END OF DATA ── ({} records, {} errors)\n",
            record_num, error_count
        );

        if error_count > 0 || any_corrupted {
            any_corrupted = true;
        }
    }

    std::process::exit(if any_corrupted { 1 } else { 0 });
}
