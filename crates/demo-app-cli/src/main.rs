//! CLI binary: parse arguments, run one command, map the result to an exit code.

use std::process::ExitCode;

use clap::Parser;
use demo_app_cli::args::{Cli, Command};
use demo_app_cli::commands::{self, Outcome};
use demo_app_cli::error::CliError;
use demo_app_cli::output::{self, Format};

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            // Help and version are "errors" to clap; they exit 0. Real usage errors exit 1.
            let usage = err.use_stderr();
            if err.print().is_err() || usage {
                return ExitCode::from(1);
            }
            return ExitCode::SUCCESS;
        }
    };
    let format = Format::resolve(cli.output);
    match run(&cli, format) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            output::error(format, &err);
            err.into()
        }
    }
}

fn run(cli: &Cli, format: Format) -> Result<(), CliError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::usage("runtime", e.to_string()))?;
    let outcome = runtime.block_on(commands::run(cli))?;
    output::outcome(format, &outcome);
    match (&cli.command, &outcome) {
        (Command::Doctor, Outcome::Doctor(report)) => report.verdict(),
        _ => Ok(()),
    }
}
