#![allow(dead_code)] // Remove this later

use std::{path::PathBuf, process::Command};

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
    data: Option<UserProvidedConfig>,
    template: Configuration,
    client: Client,
    pwd: Option<PathBuf>,
}

#[pymethods]
impl Dispatcher {
    #[new]
    pub fn new() -> Result<Self, ServicingError> {
        // Check if the user has installed the required python package
        if !helper::check_python_package_installed(CLUSTER_ORCHESTRATOR) {
            return Err(ServicingError::PipPackageError(CLUSTER_ORCHESTRATOR));
        }

        Ok(Self {
            data: None,
            template: Configuration::default(),
            client: Client::new(),
            pwd: None,
        })
    }

    pub fn update_service(&mut self, config: UserProvidedConfig) -> Result<(), ServicingError> {
        // Update the configuration with the user provided configuration
        self.template.update(&config);
        self.data = Some(config);

        // create a directory in the user home directory
        let pwd = helper::create_directory(".servicing", true)?;
        self.pwd = Some(pwd);
        Ok(())
    }

    pub fn up(&self) -> Result<(), ServicingError> {
        let output = Command::new("sky").arg("--version").output();
        match output {
            Ok(output) => {
                let version = String::from_utf8_lossy(&output.stdout);
                info!("Sky version: {}", version);
                Ok(())
            }
            Err(e) => Err(ServicingError::ClusterProvisionError(e.to_string())),
        }
    }

    pub fn down(&self) {}

    pub fn status(&self) {}

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
    fn test_new() {
        let dispatcher = Dispatcher::new().unwrap();
        assert_eq!(dispatcher.template.workdir, ".".to_string());
    }

    #[test]
    fn test_fetch() {
        let dispatcher = Dispatcher::new().unwrap();
        let result = dispatcher
            .fetch("https://httpbin.org/get".to_string())
            .unwrap();
        assert!(result.contains("origin"));
    }
}
