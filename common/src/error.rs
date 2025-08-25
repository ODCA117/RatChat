use thiserror::Error;

#[derive(Error, Debug)]
pub enum RatError {
    #[error("Generic error")]
    Error(String),

    #[error("Failed to parse")] // Make this from serde
    ParseError,

    #[error("Protocol failure, wrong message in wrong order")]
    ProtocolError,
}
