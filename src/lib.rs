use env_logger::Builder;
use pyo3::{pymodule, types::PyModule, Bound, PyResult};

use crate::{dispatcher::Dispatcher, models::UserProvidedConfig};

mod dispatcher;
mod error;
mod helper;
mod models;

/// A Python module implemented in Rust.
#[pymodule]
fn servicing(m: &Bound<'_, PyModule>) -> PyResult<()> {
    Builder::new().filter_level(log::LevelFilter::Info).init();
    m.add_class::<Dispatcher>()?;
    m.add_class::<UserProvidedConfig>()?;
    Ok(())
}
