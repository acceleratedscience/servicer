#![allow(dead_code)] // Remove this later

use std::{collections::HashMap, path::PathBuf, process::Command};

use log::info;
use pyo3::{pyclass, pymethods};
use reqwest::{header::ACCEPT, Client};

use crate::{
    error::ServicingError,
    helper,
    models::{Configuration, UserProvidedConfig},
};

static CLUSTER_ORCHESTRATOR: &str = "skypilot";

/// Dispatcher is a struct that is responsible for creating the service configuration and launching
/// the cluster on a particular cloud provider.
#[pyclass]
pub struct Dispatcher {
    client: Client,
    service: HashMap<String, Service>,
}

#[pyclass]
struct Service {
    data: Option<UserProvidedConfig>,
    template: Configuration,
    filepath: Option<PathBuf>,
    url: Option<String>,
}

#[pymethods]
impl Dispatcher {
    #[new]
    pub fn new() -> Result<Self, ServicingError> {
        // Check if the user has installed the required python package
        if !helper::check_python_package_installed(CLUSTER_ORCHESTRATOR) {
            return Err(ServicingError::PipPackageError(CLUSTER_ORCHESTRATOR));
        }

        Ok(Dispatcher {
            client: Client::new(),
            service: HashMap::new(),
        })
    }

    pub fn add_service(
        &mut self,
        name: String,
        config: Option<UserProvidedConfig>,
    ) -> Result<(), ServicingError> {
        let mut service = Service {
            data: None,
            template: Configuration::default(),
            filepath: None,
            url: None,
        };

        // Update the configuration with the user provided configuration
        if let Some(config) = config {
            info!("Updating the configuration with the user provided configuration");
            service.template.update(&config);
            service.data = Some(config);
        }

        // create a directory in the user home directory
        let pwd = helper::create_directory(".servicing", true)?;

        // create a file in the created directory
        let file = helper::create_file(&pwd, &(name.clone() + "_service.yaml"))?;

        // write the configuration to the file
        let content = serde_yaml::to_string(&service.template)?;
        helper::write_to_file(&file, &content)?;

        service.filepath = Some(file);

        self.service.insert(name, service);

        Ok(())
    }

    pub fn up(&self, name: String) -> Result<(), ServicingError> {
        let output = Command::new("sky").arg("--version").output();
        match output {
            Ok(output) => {
                let version = String::from_utf8_lossy(&output.stdout);
                info!("Sky version: {}", version);
            }
            Err(e) => return Err(ServicingError::ClusterProvisionError(e.to_string())),
        }
        // get the service configuration
        if let Some(service) = self.service.get(&name) {
            info!("Launching the cluster with the configuration: {:?}", name);
            // launch the cluster
            let _child = Command::new("sky")
                .arg("serve")
                .arg("up")
                .arg(
                    service
                        .filepath
                        .as_ref()
                        .ok_or(ServicingError::General("filepath not found".to_string()))?,
                )
                .spawn()?;
            return Ok(());
        }
        Err(ServicingError::ServiceNotFound(name))
    }

    pub fn down(&self, name: String) -> Result<(), ServicingError> {
        // get the service configuration
        if let Some(service) = self.service.get(&name) {
            info!("Launching the cluster with the configuration: {:?}", name);
            // launch the cluster
            let _child = Command::new("sky")
                .arg("serve")
                .arg("down")
                .arg(
                    service
                        .filepath
                        .as_ref()
                        .ok_or(ServicingError::General("filepath not found".to_string()))?,
                )
                .spawn()?;
            return Ok(());
        }
        Err(ServicingError::ServiceNotFound(name))
    }

    pub fn status(&self, name: Option<String>) {
        match name {
            Some(name) => {
                info!("Getting the status of the service: {:?}", name);
            }
            _ => {
                info!("Getting the status of all the services");
            }
        }
    }

    pub fn save(&self) {}

    pub fn fetch(&self, url: String) -> Result<String, ServicingError> {
        // create tokio runtime that is single threaded
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let res = self
                    .client
                    .get(url)
                    .header(ACCEPT, "application/json")
                    .send()
                    .await?;
                let body = res.text().await?;
                Ok::<_, ServicingError>(body)
            })?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch() {
        let dispatcher = Dispatcher::new().unwrap();
        let result = dispatcher
            .fetch("https://httpbin.org/get".to_string())
            .unwrap();
        assert!(result.contains("origin"));
    }
}
