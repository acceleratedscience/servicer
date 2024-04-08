use pyo3::pyclass;
use serde::{Deserialize, Serialize};

#[pyclass]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserProvidedConfig {
    pub name: String,
    pub port: u16,
    pub replicas: u16,
    pub cloud: String,
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
        self.service.replicas = config.replicas;
        self.resources.ports = config.port;
        self.resources.cloud = config.cloud.clone();
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Service {
    pub readiness_probe: String,
    pub replicas: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Resources {
    pub ports: u16,
    pub cloud: String,
    pub cpus: String,
    pub memory: String,
    pub disk_size: u16,
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
                disk_size: 50,
            },
            workdir: ".".to_string(),
            setup: "conda install cudatoolkit -y\n".to_string()
                + "pip install gt4sd-trainer-hf-pl\n"
                + "pip install .\n"
                + "pip install fastapi\n"
                + "pip install uvicorn\n",
            run: "python service.py\n".to_string(),
        }
    }
}

#[allow(dead_code)]
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
        },
        setup: "".to_string(),
        workdir: ".".to_string(),
        run: "python -m http.server 8080\n".to_string(),
    }
}
