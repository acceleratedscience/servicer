use pyo3::{exceptions::PyRuntimeError, PyErr};
use thiserror::Error;
use yaml_rust::ScanError;

#[allow(dead_code)] // Remove this later
#[derive(Debug, Error)]
pub enum ServicerError {
    #[error("Service general error: {0}")]
    General(String),
    #[error("{0}")]
    IO(#[from] std::io::Error),
    #[error("{0}")]
    ScanError(#[from] ScanError),
    #[error("Package {0} is not installed")]
    PipPackageError(&'static str),
    #[error("{0}")]
    ReqwestError(#[from] reqwest::Error),
}

impl From<ServicerError> for PyErr {
    fn from(err: ServicerError) -> PyErr {
        PyErr::new::<PyRuntimeError, _>(err.to_string())
    }
}
