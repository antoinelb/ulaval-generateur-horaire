use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match ulaval_scheduler_scraper::cli::run(
        std::env::args().skip(1).collect(),
    )
    .await
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::from(2)
        }
    }
}
