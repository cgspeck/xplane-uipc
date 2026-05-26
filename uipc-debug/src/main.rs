use std::collections::HashMap;

use clap::Parser;

mod eval;
mod ipc;
mod state;
mod trace;
mod tui;

#[derive(Parser, Debug)]
#[command(name = "uipc-debug", about = "FSUIPC mapping debugger")]
struct Cli {
    #[arg(short = 'm', long)]
    mapping: String,

    #[arg(short = 's', long)]
    state: Option<String>,

    #[arg(long, default_value = "trace")]
    log_level: String,

    #[arg(long)]
    log_file: Option<String>,

    #[arg(long, default_value_t = false)]
    no_log_file: bool,

    #[arg(long, default_value_t = false)]
    no_ipc: bool,
}

fn main() {
    let cli = Cli::parse();

    let log_file = if cli.no_log_file {
        None
    } else {
        Some(cli.log_file.unwrap_or_else(|| "uipc-debug.log".into()))
    };

    let _trace_buf = trace::init_tracing(&cli.log_level, log_file);

    let mapping_config = match uipc_mapping::load_mappings(&cli.mapping) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to load mapping file: {}", e);
            std::process::exit(1);
        }
    };

    for err in &mapping_config.load_errors {
        tracing::warn!("Mapping load warning: {}", err);
    }

    tracing::info!(
        "Loaded {} mappings from {}",
        mapping_config.mappings.len(),
        cli.mapping
    );

    let state: HashMap<String, f64> = match &cli.state {
        Some(path) => match state::load_state(path) {
            Ok(s) => {
                tracing::info!("Loaded state: {} entries from {}", s.len(), path);
                s
            }
            Err(e) => {
                tracing::error!("Failed to load state: {}", e);
                HashMap::new()
            }
        },
        None => {
            tracing::info!("No state file provided — all mappings will show missing keys");
            HashMap::new()
        }
    };

    let (ipc_handle, ipc_tx) = if !cli.no_ipc {
        match ipc::spawn() {
            Ok((handle, tx)) => {
                tracing::info!("IPC thread started");
                (Some(handle), Some(tx))
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to start IPC thread, falling back to offline mode: {}",
                    e
                );
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    let ipc_enabled = ipc_handle.is_some();

    let app = tui::App::new(
        mapping_config.mappings,
        state,
        cli.mapping,
        _trace_buf,
        ipc_handle,
        ipc_tx,
        ipc_enabled,
    );

    if let Err(e) = tui::run(app) {
        eprintln!("TUI error: {}", e);
        std::process::exit(1);
    }
}
