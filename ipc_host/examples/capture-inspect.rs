use std::env;
use std::fs;

use ipc_host::mapped_view::iterate_records;

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
        let error_count = unsafe {
            iterate_records(data.as_ptr(), data.len(), |record| {
                record_num += 1;
                let op = if record.is_write { "WRITE" } else { "READ" };
                let marker = if record.sentinel_ok { "✓" } else { "?" };
                println!(
                    "#{:<4} reqID={:#010x}  offset={:#06x}  {}  {}B  {}",
                    record_num, record.req_id, record.dw_offset, op, record.n_bytes, marker
                );
                0
            })
        };

        println!(
            "── END OF DATA ── ({} records, {} errors)\n",
            record_num, error_count
        );

        if error_count > 0 {
            had_errors = true;
        }
    }

    std::process::exit(if had_errors { 1 } else { 0 });
}
