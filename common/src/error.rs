use thiserror::Error;

#[derive(Error, Debug)]
pub enum RatError {
    #[error("Generic error")]
    Error(String),
}
