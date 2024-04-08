use pyo3::{exceptions::PyRuntimeError, PyErr};
use thiserror::Error;

#[allow(dead_code)] // Remove this later
#[derive(Debug, Error)]
pub enum ServicingError {
    #[error("Service general error: {0}")]
    General(String),
    #[error("{0}")]
    IO(#[from] std::io::Error),
    #[error("Package {0} is not installed")]
    PipPackageError(&'static str),
    #[error("{0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("{0}")]
    ClusterProvisionError(String),
    #[error("{0}")]
    SerdeYamlError(#[from] serde_yaml::Error),
    #[error("Service {0} not found")]
    ServiceNotFound(String),
    #[error("{0}")]
    BinaryEncodeError(#[from] bincode::Error),
}

impl From<ServicingError> for PyErr {
    fn from(err: ServicingError) -> PyErr {
        PyErr::new::<PyRuntimeError, _>(err.to_string())
    }
}
