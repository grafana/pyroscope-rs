pub mod cli;
pub mod core;
pub mod utils;

use crate::core::commands;
use crate::utils::error::Result;

fn main() -> Result<()> {
    let cli = cli::parse()?;
    let mut cmd = cli.0;
    let args = cli.1;
    let _guard = utils::logger::setup_logging(args.log_level)?;
    match &mut cmd {
        cli::ValidatedCommands::Exec(exec) => {
            commands::exec(&exec.command, &args)?;
        }
        cli::ValidatedCommands::Connect(connect) => {
            commands::connect(connect.pid, &args)?;
        }
    }
    Ok(())
}
