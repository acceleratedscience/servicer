use thiserror::Error;

#[allow(dead_code)] // Remove this later
#[derive(Debug, Error)]
pub enum ServicerError {
    #[error("Service general error: {0}")]
    General(String)
}
