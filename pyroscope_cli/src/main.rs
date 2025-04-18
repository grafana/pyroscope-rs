use clap::Parser;
pub mod cli;
pub mod core;
pub mod utils;

use crate::core::commands;
use crate::utils::error::Result;

fn main() -> Result<()> {
    let cli1 = cli::Cli::parse();
    match cli1.command {
        cli::Commands::Exec(exec) => {
            let _guard = utils::logger::setup_logging(exec.common.log_level)?;
            commands::exec(exec.command, exec.common)?;
        }
        cli::Commands::Connect(connect) => {
            let _guard = utils::logger::setup_logging(connect.common.log_level)?;
            commands::connect(connect.pid, connect.common)?;
        }
    }
    Ok(())
}
