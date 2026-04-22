use std::process::ExitCode;

mod cli;
mod commands;
mod error;

use crate::error::AppError;

fn main() -> ExitCode {
    if let Err(error) = run() {
        eprintln!("rloc: {error:#}");
        return ExitCode::from(error.exit_code());
    }

    ExitCode::SUCCESS
}

fn run() -> Result<(), AppError> {
    let cli = cli::Cli::parse_args();
    commands::run(cli)
}
