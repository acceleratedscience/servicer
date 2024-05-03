use pyo3::{pyclass, pymethods};
use serde::{Deserialize, Serialize};

use crate::error::Result;

// skypilot as the orchestrator
pub mod sky;

pub trait Orchestrator {
    fn setup(&self) -> Result<()>;
    fn up(&self) -> Result<()>;
    fn down(&self) -> Result<()>;
    fn status(&self) -> Result<String>;
    fn save(&self) -> Result<()>;
    fn load(&self) -> Result<()>;
}

pub trait UserConfig {
    fn update(&self) -> Result<()>;
}

#[pyclass]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserProvidedConfig {
    pub port: Option<u16>,
    pub replicas: Option<u16>,
    pub cloud: Option<String>,
    pub workdir: Option<String>,
    pub disk_size: Option<u16>,
    pub cpu: Option<String>,
    pub memory: Option<String>,
    pub accelerators: Option<String>,
    pub setup: Option<String>,
    pub run: Option<String>,
}

#[pymethods]
impl UserProvidedConfig {
    #[new]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        port: Option<u16>,
        replicas: Option<u16>,
        cloud: Option<String>,
        workdir: Option<String>,
        disk_size: Option<u16>,
        cpu: Option<String>,
        memory: Option<String>,
        accelerators: Option<String>,
        setup: Option<String>,
        run: Option<String>,
    ) -> Self {
        UserProvidedConfig {
            port,
            replicas,
            cloud,
            workdir,
            disk_size,
            cpu,
            memory,
            accelerators,
            setup,
            run,
        }
    }
}
