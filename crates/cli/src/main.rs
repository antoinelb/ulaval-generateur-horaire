use std::process::ExitCode;

fn main() -> ExitCode {
    match ulaval_scheduler_cli::cli::run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::from(2)
        }
    }
}
