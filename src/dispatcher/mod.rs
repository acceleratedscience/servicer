#![allow(dead_code)] // Remove this later

use std::{
    collections::HashMap,
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use base64::Engine;
use futures::future::join_all;
use log::{error, info, warn};
use pyo3::prelude::*;
use pyo3::{pyclass, pymethods, types::PyDict, Bound, PyAny};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::{
    runtime::{self, Runtime},
    time::sleep,
};

use crate::{
    error::ServicingError,
    helper,
    models::{Configuration, UserProvidedConfig},
};

static CACHE_DIR: &str = ".servicing";
static CACHE_FILE_NAME: &str = "services.bin";
static CLUSTER_ORCHESTRATOR: &str = "skypilot";
static SERVICE_CHECK_INTERVAL: Duration = Duration::from_secs(5);
static REPLICA_UP_CHECK: &str = "no ready replicas";

static REGEX_URL: OnceLock<Regex> = OnceLock::new();

/// Dispatcher is a struct that is responsible for creating the service configuration and launching
/// the cluster on a particular cloud provider.
#[pyclass(subclass)]
pub struct Dispatcher {
    client: Client,
    rt: Runtime,
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
    #[pyo3(signature = (*_args, **_kwargs))]
    pub fn new(_args: &Bound<'_, PyAny>, _kwargs: Option<&Bound<'_, PyAny>>) -> Result<Self, ServicingError> {
        // Check if sky_check is True in _kwargs
        let mut skip_sky_validation = false;
        if let Some(kwargs) = _kwargs {
            if let Ok(dict) = kwargs.downcast::<PyDict>() {
                if let Ok(Some(sky_check)) = dict.get_item("skip_sky_validation") {
                    if sky_check.is_truthy().unwrap_or(false) {
                        skip_sky_validation = true;
                        info!("Skipping check for python package: {}", CLUSTER_ORCHESTRATOR);
                    }
                }
            }
        }
        // Check if the user has installed the required python package
        if !skip_sky_validation && !helper::check_python_package_installed(CLUSTER_ORCHESTRATOR) {
            return Err(ServicingError::PipPackageError(CLUSTER_ORCHESTRATOR));
        }

        let re = Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}:\d+\b")?;
        let _ = REGEX_URL.get_or_init(|| re);

        let service = Arc::new(Mutex::new(HashMap::new()));

        // tokio runtime with one dedicated worker
        let rt = runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("servicing")
            .enable_all()
            .build()?;

        Ok(Self {
            client: Client::builder()
                .pool_max_idle_per_host(0)
                .timeout(Duration::from_secs(10))
                .build()?,
            rt,
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
        let pwd = helper::create_directory(CACHE_DIR, true)?;

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
            // check if service is not yet up but started
            if let Some(_) = service.url {
                return Err(ServicingError::ClusterProvisionError(format!(
                    "Service {} is starting",
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

    pub fn up(&mut self, name: String, skip_prompt: Option<bool>) -> Result<(), ServicingError> {
        // get the service configuration
        if let Some(service) = self.service.lock()?.get_mut(&name) {
            // check if service is either up or starting
            if let Some(_) = service.url {
                return Err(ServicingError::ClusterProvisionError(format!(
                    "Service {} is starting or already up",
                    name
                )));
            }

            info!("Launching the service with the configuration: {:?}", name);
            // launch the cluster
            let mut cmd = Command::new("sky");

            cmd.arg("serve").arg("up").arg("-n").arg(&name).arg(
                service
                    .filepath
                    .as_ref()
                    .ok_or(ServicingError::General("filepath not found".to_string()))?,
            );

            if let Some(true) = skip_prompt {
                cmd.arg("-y");
            }

            let mut child = cmd.spawn()?;

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

            let url = url.to_string() + &service.template.service.readiness_probe;

            // spawn a green thread to check when service comes online, then update the service status
            let fut = async move {
                let url = format!("http://{}", url);
                loop {
                    match helper::fetch(&client_clone, &url).await {
                        Ok(resp) => {
                            if resp.to_lowercase().contains(REPLICA_UP_CHECK) {
                                sleep(SERVICE_CHECK_INTERVAL).await;
                                continue;
                            }
                            match service_clone.lock() {
                                Ok(mut service) => {
                                    if let Some(service) = service.get_mut(&name) {
                                        service.up = true;
                                    } else {
                                        warn!("Service not found");
                                    }
                                    info!("Service {} is up", name);
                                    break;
                                }
                                Err(e) => {
                                    error!("Error fetching the service: {:?}", e);
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error fetching the service endpoint: {:?}", e);
                            break;
                        }
                    }
                }
            };
            self.rt.spawn(fut);

            return Ok(());
        }
        Err(ServicingError::ServiceNotFound(name))
    }

    pub fn down(
        &mut self,
        name: String,
        skip_prompt: Option<bool>,
        force: Option<bool>,
    ) -> Result<(), ServicingError> {
        // get the service configuration
        match self.service.lock()?.get_mut(&name) {
            Some(service) if service.up || service.url.is_some() => {
                // Update service status
                service.url = None;
                service.up = false;
            }
            Some(_) => match force {
                Some(true) => {}
                Some(false) | None => {
                    return Err(ServicingError::ServiceNotUp(name));
                }
            },
            None => return Err(ServicingError::ServiceNotFound(name)),
        }
        info!("Destroying the service with the configuration: {:?}", name);
        // launch the cluster
        let mut cmd = Command::new("sky");
        cmd.arg("serve").arg("down").arg(&name);
        if let Some(true) = skip_prompt {
            cmd.arg("-y");
        }
        let mut child = cmd.spawn()?;

        child.wait()?;

        Ok(())
    }

    pub fn status(&mut self, name: String, pretty: Option<bool>) -> Result<String, ServicingError> {
        // Check if the service exists
        if let Some(service) = self.service.lock()?.get_mut(&name) {
            info!("Checking the status of the service: {:?}", name);

            // if service is up poll once to see if it's still up
            if let (true, Some(url)) = (service.up, &service.url) {
                let url = format!(
                    "http://{}{}",
                    url, &service.template.service.readiness_probe
                );

                let r = self.rt.block_on(async {
                    let res = helper::fetch(&self.client, &url).await;
                    match res {
                        Ok(resp) => {
                            if resp.to_lowercase().contains(REPLICA_UP_CHECK) {
                                Err(ServicingError::ServiceNotUp(name.clone()))
                            } else {
                                // it's up
                                Ok(())
                            }
                        }
                        Err(e) => Err::<(), _>(ServicingError::General(e.to_string())),
                    }
                });

                match r {
                    Ok(_) => {
                        //No-op
                        info!("Service {} is up", name);
                    }
                    Err(e) => {
                        warn!("{:?}", e);
                        service.up = false;
                    }
                }
            }

            return Ok(match pretty {
                Some(true) => serde_json::to_string_pretty(service)?,
                _ => serde_json::to_string(service)?,
            });
        }
        Err(ServicingError::ServiceNotFound(name))
    }

    pub fn save(&self, location: Option<PathBuf>) -> Result<(), ServicingError> {
        let bin = bincode::serialize(&*self.service.lock()?)?;

        helper::write_to_file_binary(
            &helper::create_file(
                &{
                    if let Some(location) = location {
                        helper::create_directory(
                            location
                                .to_str()
                                .ok_or(ServicingError::General("Location is None".to_string()))?,
                            false,
                        )?
                    } else {
                        helper::create_directory(CACHE_DIR, true)?
                    }
                },
                CACHE_FILE_NAME,
            )?,
            &bin,
        )?;

        Ok(())
    }

    pub fn save_as_b64(&self) -> Result<String, ServicingError> {
        let bin = bincode::serialize(&*self.service.lock()?)?;
        let b64 = base64::prelude::BASE64_STANDARD.encode(bin);
        Ok(b64)
    }

    pub fn load(
        &mut self,
        location: Option<PathBuf>,
        update_status: Option<bool>,
    ) -> Result<(), ServicingError> {
        let location = if let Some(location) = location {
            helper::create_directory(
                location
                    .to_str()
                    .ok_or(ServicingError::General("Location is None".to_string()))?,
                false,
            )?
            .join(CACHE_FILE_NAME)
        } else {
            helper::create_directory(CACHE_DIR, true)?.join(CACHE_FILE_NAME)
        };

        let bin = helper::read_from_file_binary(&location)?;

        self.service
        .lock()?
        .extend(bincode::deserialize::<HashMap<String, Service>>(&bin)?);

        if let Some(true) = update_status {
            info!("Checking for services that may come up while you were away...");

            // Clones to pass to threads
            let service_clone = self.service.clone();
            let client_clone = self.client.clone();
            let mut service_to_check = Vec::new();

            // iterate through the services and find that are down
            self.service
                .lock()?
                .iter()
                .filter(|(_, service)| !service.up && service.url.is_some())
                .for_each(|(name, service)| {
                    service_to_check.push((
                        name.clone(),
                        service
                            .url
                            .clone()
                            .expect("Gettting url, this should never be None")
                            + &service.template.service.readiness_probe,
                    ))
                });

            if service_to_check.is_empty() {
                info!("No services to check");
                return Ok(());
            }

            info!("Services to check: {:?}", service_to_check);

            self.rt.spawn(async move {
                let mut handles = Vec::new();
                for (name, url) in service_to_check {
                    let client_clone = client_clone.clone();
                    let url = format!("http://{}", url);
                    let handle = tokio::spawn(async move {
                        match helper::fetch_and_check(
                            &client_clone,
                            &url,
                            REPLICA_UP_CHECK,
                            Some(SERVICE_CHECK_INTERVAL),
                        )
                        .await
                        {
                            Ok(_) => {}
                            Err(e) => {
                                return Err(e);
                            }
                        }
                        Ok(name)
                    });
                    handles.push(handle);
                }
                for res in join_all(handles).await {
                    let mut service = match service_clone.lock() {
                        Ok(s) => s,
                        Err(e) => {
                            error!("Poisoned lock {e}");
                            return;
                        }
                    };

                    match res {
                        Ok(Ok(r)) => {
                            if let Some(service) = service.get_mut(&r) {
                                service.up = true;
                                info!("Service {} is up", r);
                            }
                        }
                        Ok(Err(e)) => {
                            warn!("{e}");
                        }
                        Err(e) => {
                            error!("{e}");
                        }
                    }
                }
            });
        }

        Ok(())
    }

    pub fn load_from_b64(&mut self, b64: String) -> Result<(), ServicingError> {
        let bin = base64::prelude::BASE64_STANDARD.decode(b64.as_bytes())?;
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
    use pyo3::{pyclass, types::PyDict, Bound, PyAny, Python};

    use crate::models::UserProvidedConfig;

    #[pyclass]
    struct Empty;

    #[test]
    fn test_dispatcher() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let bound_args = Bound::new(py, Empty).unwrap();
            let bound_kwargs = Some(PyDict::new_bound(py));
            // let bound_kwargs = Some(PyDict::new_bound(py));
            let mut dis = super::Dispatcher::new(&bound_args, bound_kwargs.as_deref()).unwrap();

            dis.add_service(
                "testing".to_string(),
                Some(UserProvidedConfig {
                    port: Some(1234),
                    replicas: Some(5),
                    cloud: Some("aws".to_string()),
                    workdir: None,
                    data: None,
                    setup: None,
                    run: None,
                    disk_size: None,
                    cpu: None,
                    accelerators: None,
                    memory: None,
                }),
            )
            .unwrap();

            // test the runtime... should NOT panic
            dis.rt.block_on(async { "" });

            dis.save(None).unwrap();

            // check what has been added
            {
                let services = dis.service.lock().unwrap();
                let service = services.get("testing").unwrap();
                assert_eq!(service.template.resources.ports, 1234);
                assert_eq!(service.template.service.replicas, 5);
                assert_eq!(service.template.resources.cloud, "aws");
            }

            dis.remove_service("testing".to_string()).unwrap();
            assert!(dis.service.lock().unwrap().get("testing").is_none());

            dis.load(None, None).unwrap();
            {
                let services = dis.service.lock().unwrap();
                let service = services.get("testing").unwrap();
                assert_eq!(service.template.resources.ports, 1234);
            }
        });
    }
}
