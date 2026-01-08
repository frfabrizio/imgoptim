use clap::Parser;
use tracing::{info, warn};

use imgoptim::cli::Cmd;
use imgoptim::rules::decision;
use imgoptim::rules::normalize::normalize_options;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    let cmd = Cmd::parse();
    let (mode, opts) = cmd.into_mode_and_options();

    let opts = normalize_options(mode, opts)?;

    let mut had_error = false;

    for input in opts.inputs.iter() {
        let input_path = input.as_path();

        match decision::process_one(input_path, &opts) {
            Ok(summary) => {
                if opts.verbosity.is_verbose() {
                    info!("{summary}");
                }
            }
            Err(e) => {
                had_error = true;
                decision::record_error();
                if !matches!(opts.verbosity, imgoptim::cli::Verbosity::Quiet) {
                    // tracing "structuré" => plus robuste, évite les soucis d'inférence
                    warn!(path = %input_path.display(), error = %e, "processing failed");
                }
            }
        }
    }

    if opts.print_totals {
        decision::print_totals();
    }

    if had_error {
        std::process::exit(2);
    }
    Ok(())
}
