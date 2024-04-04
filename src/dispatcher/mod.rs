#![allow(dead_code)] // Remove this later

use std::fs::read_to_string;

use log::info;
use pyo3::{pyclass, pymethods};
use reqwest::{header::ACCEPT, Client};
use yaml_rust::{Yaml, YamlLoader};

use crate::error::ServicerError;
use crate::helper;

#[pyclass]
#[derive(Clone)]
pub struct ServiceConfig {
    name: String,
    port: u16,
    replicas: u16,
}

static DEFAULT_TEMPLATE: &str = include_str!("../../template/service.yaml");
static CLUSTER_ORCHESTRATOR: &str = "skypilot";

/// Dispatcher is a struct that is responsible for creating the service configuration and launching
/// the cluster on a particular cloud provider.
#[pyclass]
pub struct Dispatcher {
    data: Option<ServiceConfig>,
    template: Yaml,
    client: Client,
}

#[pymethods]
impl Dispatcher {
    #[new]
    pub fn new(path: Option<String>) -> Result<Self, ServicerError> {
        // Check if the user has installed the required python package
        if !helper::check_python_package(CLUSTER_ORCHESTRATOR) {
            return Err(ServicerError::PipPackageError(CLUSTER_ORCHESTRATOR));
        }

        // fetch yaml template(s)
        let yamls = match path {
            Some(path) => {
                let raw = read_to_string(path)?;
                YamlLoader::load_from_str(&raw)?
            }
            _ => YamlLoader::load_from_str(DEFAULT_TEMPLATE)?,
        };

        let yaml = &yamls[0];

        info!("{:?}", yaml["service"]);

        Ok(Self {
            data: None,
            template: yaml.clone(),
            client: Client::new(),
        })
    }

    pub fn update_service(&mut self, config: ServiceConfig) {
        self.data = Some(config);
    }

    pub fn up(&self) {}

    pub fn down(&self) {}

    pub fn status(&self) {}

    pub fn fetch(&self, url: String) -> Result<String, ServicerError> {
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
                Ok::<_, ServicerError>(body)
            })?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let dispatcher = Dispatcher::new(None).unwrap();
        assert_eq!(dispatcher.template["name"].as_str().unwrap(), "my-task");
    }

    #[test]
    fn test_fetch() {
        let dispatcher = Dispatcher::new(None).unwrap();
        let result = dispatcher
            .fetch("https://httpbin.org/get".to_string())
            .unwrap();
        assert!(result.contains("origin"));
    }
}
