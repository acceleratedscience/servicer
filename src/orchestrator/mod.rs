use pyo3::pyclass;
use serde::{Deserialize, Serialize};

use crate::models::Orchestrator;

use self::sky::Sky;

pub mod sky;
pub mod foo;

#[pyclass]
#[derive(Clone)]
#[derive(Serialize, Deserialize)]
pub enum Orchestrators {
    SkyPilot = 0,
    Local = 1,
}

impl Orchestrators {
    pub fn get_orchestrator(&self) -> Box<dyn Orchestrator> {
        match self {
            Self::SkyPilot => Box::new(Sky::default()),
            Self::Local => panic!("Local orchestrator not implemented"),
        }
    }
}
