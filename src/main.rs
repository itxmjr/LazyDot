use clap::Parser;
use lazydot::cli::{run, Cli};

fn main() -> std::process::ExitCode {
    match run(Cli::parse()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("lazydot: {e:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
