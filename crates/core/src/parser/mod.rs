pub mod catalogue;
pub mod course;
pub mod program;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Missing element: {selector}")]
    MissingElement { selector: String },
    #[error("Malformed entry for {selector}: {raw}")]
    MalformedEntry { selector: String, raw: String },
    #[error("Malformed prerequisites {error}: {raw}")]
    MalformedPrerequisites { error: String, raw: String },
}
