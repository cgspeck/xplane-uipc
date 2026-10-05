use std::env;
use std::fs;

use ipc_host::mapped_view::{RecordKind, iterate_records};

/// Write payloads longer than this are truncated in the listing.
const MAX_WRITE_BYTES_SHOWN: usize = 16;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: capture-inspect <file.bin> [file2.bin ...]");
        std::process::exit(1);
    }

    let mut had_errors = false;

    for path in &args[1..] {
        let data = match fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Error reading '{}': {}", path, e);
                had_errors = true;
                continue;
            }
        };

        println!("File: {}", path);
        let line_len = path.len() + 6;
        println!("{}", "-".repeat(line_len));

        let mut record_num = 0u32;
        let result = unsafe {
            iterate_records(data.as_ptr(), data.len(), |record| {
                record_num += 1;
                let detail = if record.kind == RecordKind::Write {
                    let start = record.header_offset + record.kind.header_len();
                    let shown = (record.n_bytes as usize).min(MAX_WRITE_BYTES_SHOWN);
                    let mut hex: Vec<String> = data[start..start + shown]
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect();
                    if shown < record.n_bytes as usize {
                        hex.push("…".to_string());
                    }
                    hex.join(" ")
                } else {
                    format!("pDest={:#010x}", record.p_dest)
                };
                println!(
                    "#{:<4} @{:#06x}  {:<6}  offset={:#06x}  {}B  {}",
                    record_num,
                    record.header_offset,
                    record.kind,
                    record.dw_offset,
                    record.n_bytes,
                    detail
                );
            })
        };

        match result {
            Ok(()) => println!("── END OF DATA ── ({} records)\n", record_num),
            Err(malformed) => {
                println!("── MALFORMED: {} ── ({} records)\n", malformed, record_num);
                had_errors = true;
            }
        }
    }

    std::process::exit(if had_errors { 1 } else { 0 });
}
