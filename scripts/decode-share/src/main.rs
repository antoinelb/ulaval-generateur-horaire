// Decode an organigramme share link (the `#…` fragment of an app URL) into
// readable JSON, using the app's own `persist::decode_organigramme` so the
// tool can never drift from what the UI actually reads.
//
// Usage: cargo run --manifest-path scripts/decode-share/Cargo.toml -- <url-ou-fragment>

use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(argument) = std::env::args().nth(1) else {
        eprintln!("usage: decode-share <url-ou-fragment>");
        return ExitCode::FAILURE;
    };
    // a full URL carries the payload after its last `#`; a bare fragment
    // has no `#` and passes through unchanged
    let fragment = argument.rsplit('#').next().unwrap_or(&argument);
    match ulaval_scheduler_ui::persist::decode_organigramme(fragment) {
        Ok((plan, manual_courses)) => {
            let document = serde_json::json!({
                "plan": plan,
                "manual_courses": manual_courses,
            });
            match serde_json::to_string_pretty(&document) {
                Ok(text) => {
                    println!("{text}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("sérialisation JSON impossible : {error}");
                    ExitCode::FAILURE
                }
            }
        }
        Err(error) => {
            eprintln!("lien illisible : {error}");
            ExitCode::FAILURE
        }
    }
}
