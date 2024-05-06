use pyo3::{pyclass, pymethods};
use serde::{ser::SerializeStruct, Deserialize, Serialize};

#[pyclass]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserProvidedConfig {
    pub port: Option<u16>,
    pub replicas: Option<u16>,
    pub cloud: Option<String>,
    pub workdir: Option<String>,
    pub data: Option<String>,
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
        data: Option<String>,
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
            data,
            disk_size,
            cpu,
            memory,
            accelerators,
            setup,
            run,
        }
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
        if let Some(accelerators) = &config.accelerators {
            self.resources.accelerators = Some(accelerators.clone());
        }
        if let Some(setup) = &config.setup {
            self.setup = setup.clone();
        }
        if let Some(run) = &config.run {
            self.run = run.clone();
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
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let should_serialize = self.accelerators.is_some() || !serializer.is_human_readable();

        let mut stats = serializer.serialize_struct("Resources", 6)?;
        stats.serialize_field("ports", &self.ports)?;
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
                accelerators: None,
                cloud: "aws".to_string(),
                disk_size: 100,
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
            accelerators: None,
            cloud: "aws".to_string(),
            disk_size: 50,
        },
        setup: "".to_string(),
        workdir: ".".to_string(),
        run: "python -m http.server 8080\n".to_string(),
    }
}
