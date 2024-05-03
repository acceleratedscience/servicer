use std::{path::PathBuf, sync::OnceLock, time::Duration};

use pyo3::{pyclass, pymethods};
use regex::Regex;
use serde::{ser::SerializeStruct, Deserialize, Serialize};

use crate::{
    error::{Result, ServicingError},
    helper,
};

use super::{Orchestrator, UserProvidedConfig};

#[pyclass]
pub struct Sky {
    pub data: Option<UserProvidedConfig>,
    pub template: Configuration,
    pub filepath: Option<PathBuf>,
    pub url: Option<String>,
    pub up: bool,
}

#[pymethods]
impl Sky {
    #[new]
    pub fn new(config: Option<UserProvidedConfig>) -> Self {
        let mut sky = Self {
            data: None,
            template: Configuration::default(),
            filepath: None,
            url: None,
            up: false,
        };
        if let Some(config) = config {
            sky.template.update(&config);
            sky.data = Some(config);
        }
        sky
    }
}

static CLUSTER_ORCHESTRATOR: &str = "skypilot";
static SERVICE_CHECK_INTERVAL: Duration = Duration::from_secs(5);
static REPLICA_UP_CHECK: &str = "no ready replicas";

pub static REGEX_URL: OnceLock<Regex> = OnceLock::new();

impl Orchestrator for Sky {
    fn setup(&self) -> Result<()> {
        // Check if the user has installed the required python package
        if !helper::check_python_package_installed(CLUSTER_ORCHESTRATOR) {
            return Err(ServicingError::PipPackageError(CLUSTER_ORCHESTRATOR));
        }

        // Setup regex for URL
        let _ = REGEX_URL.get_or_init(|| {
            Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}:\d+\b").expect("Failed to compile regex for URL")
        });

        Ok(())
    }

    fn up(&self) -> Result<()> {
        todo!()
    }

    fn down(&self) -> Result<()> {
        todo!()
    }

    fn status(&self) -> Result<String> {
        todo!()
    }

    fn save(&self) -> Result<()> {
        todo!()
    }

    fn load(&self) -> Result<()> {
        todo!()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    pub service: Service,
    pub resources: Resources,
    pub workdir: String,
    pub setup: String,
    pub run: String,
}

impl Configuration {
    pub fn update(&mut self, config: &UserProvidedConfig) {
        if let Some(port) = config.port {
            self.resources.ports = port;
        }
        if let Some(replicas) = config.replicas {
            self.service.replicas = replicas;
        }
        if let Some(cloud) = &config.cloud {
            self.resources.cloud = cloud.clone();
        }
        if let Some(workdir) = &config.workdir {
            self.workdir = workdir.clone();
        }
        if let Some(disk_size) = config.disk_size {
            self.resources.disk_size = disk_size;
        }
        if let Some(cpu) = &config.cpu {
            self.resources.cpus = cpu.clone();
        }
        if let Some(memory) = &config.memory {
            self.resources.memory = memory.clone();
        }
        if let Some(setup) = &config.setup {
            self.setup = setup.clone();
        }
        if let Some(run) = &config.run {
            self.run = run.clone();
        }
        if let Some(accelerators) = &config.accelerators {
            self.resources.accelerators = Some(accelerators.clone());
        }
    }

    #[allow(dead_code)]
    pub fn test_config() -> Configuration {
        test_config()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Service {
    pub readiness_probe: String,
    pub replicas: u16,
}

#[derive(Deserialize, Debug)]
pub struct Resources {
    pub ports: u16,
    pub cloud: String,
    pub cpus: String,
    pub memory: String,
    pub disk_size: u16,
    pub accelerators: Option<String>,
}

impl Serialize for Resources {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let should_serialize = self.accelerators.is_some() || !serializer.is_human_readable();

        let mut stats = serializer.serialize_struct("Resources", 6)?;
        stats.serialize_field("cloud", &self.cloud)?;
        stats.serialize_field("cpus", &self.cpus)?;
        stats.serialize_field("memory", &self.memory)?;
        stats.serialize_field("disk_size", &self.disk_size)?;
        if should_serialize {
            stats.serialize_field("accelerators", &self.accelerators)?;
        }
        stats.end()
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            service: Service {
                readiness_probe: "/health".to_string(),
                replicas: 2,
            },
            resources: Resources {
                ports: 8080,
                cpus: "4+".to_string(),
                memory: "10+".to_string(),
                cloud: "aws".to_string(),
                disk_size: 100,
                accelerators: None,
            },
            workdir: ".".to_string(),
            setup: "conda install cudatoolkit -y\n".to_string()
                + "pip install poetry\n"
                + "poetry install\n",
            run: "poetry run python service.py\n".to_string(),
        }
    }
}

#[inline]
pub fn test_config() -> Configuration {
    Configuration {
        service: Service {
            readiness_probe: "/".to_string(),
            replicas: 1,
        },
        resources: Resources {
            ports: 8080,
            cpus: "4+".to_string(),
            memory: "10+".to_string(),
            cloud: "aws".to_string(),
            disk_size: 50,
            accelerators: None,
        },
        setup: "".to_string(),
        workdir: ".".to_string(),
        run: "python -m http.server 8080\n".to_string(),
    }
}
