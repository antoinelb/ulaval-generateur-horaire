pub mod course;
pub mod listing;
pub mod prerequisites;
pub mod program;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Missing element: {selector}")]
    MissingElement { selector: String },
    #[error("Malformed entry for {selector}: {raw}")]
    MalformedEntry { selector: String, raw: String },
}
