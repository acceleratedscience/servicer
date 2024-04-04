use pyo3::{exceptions::PyRuntimeError, PyErr};
use thiserror::Error;

#[allow(dead_code)] // Remove this later
#[derive(Debug, Error)]
pub enum ServicingError {
    #[error("Service general error: {0}")]
    General(String),
    #[error("{0}")]
    IO(#[from] std::io::Error),
    #[error("{0}")]
    PipPackageError(&'static str),
    #[error("{0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("{0}")]
    ClusterProvisionError(String),
}

impl From<ServicingError> for PyErr {
    fn from(err: ServicingError) -> PyErr {
        PyErr::new::<PyRuntimeError, _>(err.to_string())
    }
}
