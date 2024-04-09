#![allow(dead_code)] // Remove this later

use std::{collections::HashMap, path::PathBuf, process::Command, sync::OnceLock};

use log::info;
use pyo3::{pyclass, pymethods};
use regex::Regex;
use reqwest::{header::ACCEPT, Client};
use serde::{Deserialize, Serialize};

use crate::{
    error::ServicingError,
    helper,
    models::{Configuration, UserProvidedConfig},
};

static CLUSTER_ORCHESTRATOR: &str = "skypilot";

static REGEX_URL: OnceLock<Regex> = OnceLock::new();

/// Dispatcher is a struct that is responsible for creating the service configuration and launching
/// the cluster on a particular cloud provider.
#[pyclass]
pub struct Dispatcher {
    client: Client,
    service: HashMap<String, Service>,
}

#[pyclass]
#[derive(Debug, Deserialize, Serialize)]
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

        let re = Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}:\d+\b")?;

        REGEX_URL.get_or_init(|| re);

        Ok(Self {
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

        // Update the configuration with the user provided configuration, if provided
        if let Some(config) = config {
            info!("Adding the configuration with the user provided configuration");
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

    pub fn up(&mut self, name: String) -> Result<(), ServicingError> {
        let output = Command::new("sky").arg("--version").output();
        match output {
            Ok(output) => {
                let version = String::from_utf8_lossy(&output.stdout);
                info!("Sky version: {}", version);
            }
            Err(e) => return Err(ServicingError::ClusterProvisionError(e.to_string())),
        }
        // get the service configuration
        if let Some(service) = self.service.get_mut(&name) {
            info!("Launching the service with the configuration: {:?}", name);
            // launch the cluster
            let mut child = Command::new("sky")
                // .stdout(Stdio::piped())
                .arg("serve")
                .arg("up")
                .arg("-n")
                .arg(&name)
                .arg(
                    service
                        .filepath
                        .as_ref()
                        .ok_or(ServicingError::General("filepath not found".to_string()))?,
                )
                .spawn()?;

            // ley skypilot handle the CLI interaction

            let output = child.wait()?;
            if !output.success() {
                return Err(ServicingError::ClusterProvisionError(format!(
                    "Cluster provision failed with code {:?}",
                    output
                )));
            }

            // get the url of the service
            let output = Command::new("sky")
                .arg("serve")
                .arg("status")
                .arg(&name)
                .output()?
                .stdout;

            // parse the output to get the url
            let output = String::from_utf8_lossy(&output);

            let url = REGEX_URL
                .get()
                .ok_or(ServicingError::General("Could not get REGEX".to_string()))?
                .find(&output)
                .ok_or(ServicingError::General(
                    "Cannot find service URL".to_string(),
                ))?
                .as_str();

            service.url = Some(url.to_string());

            return Ok(());
        }
        Err(ServicingError::ServiceNotFound(name))
    }

    pub fn down(&mut self, name: String) -> Result<(), ServicingError> {
        // get the service configuration
        if let Some(service) = self.service.get_mut(&name) {
            info!("Destroying the service with the configuration: {:?}", name);
            // launch the cluster
            let mut child = Command::new("sky")
                .arg("serve")
                .arg("down")
                .arg(&name)
                .spawn()?;

            child.wait()?;

            service.url = None;

            return Ok(());
        }
        Err(ServicingError::ServiceNotFound(name))
    }

    pub fn status(&self, name: String) -> Result<(), ServicingError> {
        // Check if the service exists
        if let Some(service) = self.service.get(&name) {
            info!("Checking the status of the service: {:?}", name);
            info!("Service configuration: {:?}", service);
            return Ok(());
        }
        Err(ServicingError::ServiceNotFound(name))
    }

    pub fn save(&self) -> Result<(), ServicingError> {
        let bin = bincode::serialize(&self.service)?;

        helper::write_to_file_binary(
            &helper::create_file(
                &helper::create_directory(".servicing", true)?,
                "services.bin",
            )?,
            &bin,
        )?;

        Ok(())
    }

    pub fn load(&mut self, location: Option<PathBuf>) -> Result<(), ServicingError> {
        let location = if let Some(location) = location {
            location
        } else {
            helper::create_directory(".servicing", true)?.join("services.bin")
        };

        let bin = helper::read_from_file_binary(&location)?;

        self.service
            .extend(bincode::deserialize::<HashMap<String, Service>>(&bin)?);

        Ok(())
    }

    pub fn list(&self) -> Result<Vec<String>, ServicingError> {
        Ok(self.service.keys().cloned().collect())
    }

    pub fn get_url(&self, name: String) -> Result<String, ServicingError> {
        if let Some(service) = self.service.get(&name) {
            if let Some(url) = &service.url {
                return Ok(url.clone());
            }
            return Err(ServicingError::General("Service is down".to_string()));
        }
        Err(ServicingError::ServiceNotFound(name))
    }

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
