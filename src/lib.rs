use env_logger::Builder;
use pyo3::prelude::*;

mod models;
mod orchestrator;
mod errors;
mod dispatch;


/// A Python module implemented in Rust.
#[pymodule]
fn servicing(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // if release mode, set log level to warn
    if cfg!(not(debug_assertions)) {
        Builder::new().filter_level(log::LevelFilter::Warn).init();
    } else {
        Builder::new().filter_level(log::LevelFilter::Info).init();
    }

    m.add_class::<dispatch::Dispatcher>()?;
    m.add_class::<models::UserProvidedConfig>()?;
    m.add_class::<orchestrator::Orchestrators>()?;
    Ok(())
}
