#![allow(dead_code)] // Remove this later

use std::{
    collections::HashMap,
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex, OnceLock},
    thread::{sleep, spawn},
    time::Duration,
};

use log::{error, info, warn};
use pyo3::{pyclass, pymethods};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    error::ServicingError,
    helper,
    models::{Configuration, UserProvidedConfig},
};

static CLUSTER_ORCHESTRATOR: &str = "skypilot";
static SEVICE_CHECK_INTERVAL: Duration = Duration::from_secs(5);

static REGEX_URL: OnceLock<Regex> = OnceLock::new();

/// Dispatcher is a struct that is responsible for creating the service configuration and launching
/// the cluster on a particular cloud provider.
#[pyclass]
pub struct Dispatcher {
    client: Client,
    service: Arc<Mutex<HashMap<String, Service>>>,
}

#[pyclass]
#[derive(Debug, Deserialize, Serialize)]
struct Service {
    data: Option<UserProvidedConfig>,
    template: Configuration,
    filepath: Option<PathBuf>,
    url: Option<String>,
    up: bool,
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
        let _ = REGEX_URL.get_or_init(|| re);

        let service = Arc::new(Mutex::new(HashMap::new()));

        Ok(Self {
            client: Client::new(),
            service,
        })
    }

    pub fn add_service(
        &mut self,
        name: String,
        config: Option<UserProvidedConfig>,
    ) -> Result<(), ServicingError> {
        // check if service already exists
        if self.service.lock()?.contains_key(&name) {
            return Err(ServicingError::ServiceAlreadyExists(name));
        }

        let mut service = Service {
            data: None,
            template: Configuration::default(),
            filepath: None,
            url: None,
            up: false,
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

        self.service.lock()?.insert(name, service);

        Ok(())
    }

    pub fn remove_service(&mut self, name: String) -> Result<(), ServicingError> {
        // check if service is still up
        let mut service = self.service.lock()?;
        if let Some(service) = service.get(&name) {
            if service.up {
                return Err(ServicingError::ClusterProvisionError(format!(
                    "Service {} is still up",
                    name
                )));
            }
            // remove the configuration file
            if let Some(filepath) = &service.filepath {
                helper::delete_file(filepath)?;
            }
        } else {
            return Err(ServicingError::ServiceNotFound(name));
        }

        // remove from cache
        service.remove(&name);
        Ok(())
    }

    pub fn up(&mut self, name: String) -> Result<(), ServicingError> {
        // get the service configuration
        if let Some(service) = self.service.lock()?.get_mut(&name) {
            if service.up {
                return Err(ServicingError::ClusterProvisionError(format!(
                    "Service {} is already up",
                    name
                )));
            }

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
            let service_clone = self.service.clone();
            let client_clone = self.client.clone();

            let url = url.to_string();

            // spawn a thread to check when service comes online, then update the service status
            spawn(move || {
                let url = format!("http://{}", url);
                loop {
                    match helper::fetch(&client_clone, &url) {
                        Ok(resp) => {
                            if resp.to_lowercase().contains("no ready replicas") {
                                sleep(SEVICE_CHECK_INTERVAL);
                                continue;
                            }
                            match service_clone.lock() {
                                Ok(mut service) => {
                                    info!("Service {} is up", name);
                                    if let Some(service) = service.get_mut(&name) {
                                        service.up = true;
                                    } else {
                                        warn!("Service not found");
                                    }
                                    break;
                                }
                                Err(e) => {
                                    error!("Error fetching the service: {:?}", e);
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error fetching the service: {:?}", e);
                            break;
                        }
                    }
                }
            });

            return Ok(());
        }
        Err(ServicingError::ServiceNotFound(name))
    }

    pub fn down(&mut self, name: String) -> Result<(), ServicingError> {
        // get the service configuration
        match self.service.lock()?.get_mut(&name) {
            Some(service) if !service.up => {
                info!("Destroying the service with the configuration: {:?}", name);
                // launch the cluster
                let mut child = Command::new("sky")
                    .arg("serve")
                    .arg("down")
                    .arg(&name)
                    .spawn()?;

                child.wait()?;

                // Update service status
                service.url = None;
                service.up = false;

                Ok(())
            }
            Some(service) if service.up => Err(ServicingError::ClusterProvisionError(format!(
                "Service {} is up and cannot be removed",
                name
            ))),
            Some(_) => Err(ServicingError::ServiceNotUp(name)),
            None => Err(ServicingError::ServiceNotFound(name)),
        }
    }

    pub fn status(&self, name: String, pretty: Option<bool>) -> Result<String, ServicingError> {
        // Check if the service exists
        if let Some(service) = self.service.lock()?.get(&name) {
            info!("Checking the status of the service: {:?}", name);
            return Ok(match pretty {
                Some(true) => serde_json::to_string_pretty(service)?,
                Some(false) => serde_json::to_string(service)?,
                None => serde_json::to_string(service)?,
            });
        }
        Err(ServicingError::ServiceNotFound(name))
    }

    pub fn save(&self) -> Result<(), ServicingError> {
        let bin = bincode::serialize(&*self.service.lock()?)?;

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
            .lock()?
            .extend(bincode::deserialize::<HashMap<String, Service>>(&bin)?);

        Ok(())
    }

    pub fn list(&self) -> Result<Vec<String>, ServicingError> {
        Ok(self.service.lock()?.keys().cloned().collect())
    }

    pub fn get_url(&self, name: String) -> Result<String, ServicingError> {
        if let Some(service) = self.service.lock()?.get(&name) {
            if let Some(url) = &service.url {
                return Ok(url.clone());
            }
            return Err(ServicingError::General("Service is down".to_string()));
        }
        Err(ServicingError::ServiceNotFound(name))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_fetch() {}
}
