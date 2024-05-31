use std::path::PathBuf;

use pyo3::{pyclass, pymethods};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::dispatch::ServiceCache;
use crate::errors::Result;

pub trait Orchestrator {
    fn setup(
        &mut self,
        cache: ServiceCache,
        pwd: PathBuf,
        name: String,
        userconfig: Option<&UserProvidedConfig>,
    ) -> Result<PathBuf>;
    fn remove(&mut self, cache: ServiceCache, name: String) -> Result<()>;
    fn update(&mut self, cache: ServiceCache, name: String) -> Result<()>;
    fn up(
        &mut self,
        client: Client,
        cache: ServiceCache,
        name: String,
        skip_prompt: Option<bool>,
    ) -> Result<()>;
    fn down(
        &mut self,
        client: Client,
        cache: ServiceCache,
        name: String,
        skip_prompt: Option<bool>,
        force: Option<bool>,
    ) -> Result<()>;
    fn status(
        &mut self,
        client: Client,
        cache: ServiceCache,
        name: String,
        pretty: Option<bool>,
    ) -> Result<String>;
    fn replica_check_string(&self) -> &'static str;
}

#[pyclass]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserProvidedConfig {
    pub port: Option<u16>,
    pub replicas: Option<u16>,
    pub readiness_probe: Option<String>,
    pub replica_up_check: Option<String>,
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
        readiness_probe: Option<String>,
        replica_up_check: Option<String>,
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
            readiness_probe,
            replica_up_check,
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
