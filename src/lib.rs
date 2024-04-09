use env_logger::Builder;
use pyo3::prelude::*;

use self::dispatcher::Dispatcher;

mod dispatcher;
mod error;
mod helper;
mod models;

/// A Python module implemented in Rust.
#[pymodule]
fn servicing(m: &Bound<'_, PyModule>) -> PyResult<()> {
    Builder::new().filter_level(log::LevelFilter::Info).init();
    m.add_class::<Dispatcher>()?;
    Ok(())
}
