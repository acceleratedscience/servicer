use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use base64::Engine;
use futures::future::join_all;
use log::{error, info, warn};
use pyo3::{pyclass, pymethods, Bound, PyAny};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::runtime::{self, Runtime};

use crate::{errors::ServicingError, models::UserProvidedConfig, orchestrator::Orchestrators};

pub mod helper;

pub type ServiceCache = Arc<Mutex<HashMap<String, Service>>>;

static CACHE_DIR: &str = ".servicing";
static CACHE_FILE_NAME: &str = "services.bin";
static SERVICE_CHECK_INTERVAL: Duration = Duration::from_secs(5);

static CACHE: OnceLock<ServiceCache> = OnceLock::new();
static RT: OnceLock<Arc<Runtime>> = OnceLock::new();

#[pyclass(subclass)]
pub struct Dispatcher {
    client: Client,
    services: ServiceCache,
    rt: Arc<Runtime>,
}

#[pyclass]
#[derive(Serialize, Deserialize)]
pub struct Service {
    pub config: Option<UserProvidedConfig>,
    pub orchestrator: Orchestrators,
    pub filepath: Option<PathBuf>,
    pub readiness_probe: String,
    pub url: Option<String>,
    pub up: bool,
}

#[pymethods]
impl Dispatcher {
    #[new]
    #[pyo3(signature = (*_args))]
    pub fn new(_args: &Bound<'_, PyAny>) -> Result<Self, ServicingError> {
        let services = CACHE.get_or_init(Default::default).clone();

        // tokio runtime with one dedicated worker
        let rt = runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("servicing")
            .enable_all()
            .build()?;
        let rt = Arc::new(rt);
        let _ = RT.get_or_init(|| rt.clone());

        Ok(Self {
            client: Client::builder()
                .pool_max_idle_per_host(0)
                .timeout(Duration::from_secs(10))
                .build()?,
            services,
            rt,
        })
    }

    pub fn add_service(
        &mut self,
        name: String,
        orchestrators: Orchestrators,
        config: Option<UserProvidedConfig>,
    ) -> Result<(), ServicingError> {
        // create a directory in the user home directory
        let pwd = helper::create_directory(CACHE_DIR, true)?;

        // Turn the orchestrator into a trait object
        let mut orchestrator = orchestrators.get_orchestrator();
        // Run setup
        let filepath =
            orchestrator.setup(self.services.clone(), pwd, name.clone(), config.as_ref())?;

        // Add the service to the cache
        self.services.lock()?.insert(
            name,
            Service {
                config,
                orchestrator: orchestrators,
                filepath: Some(filepath),
                readiness_probe: "/".to_string(),
                url: None,
                up: false,
            },
        );

        Ok(())
    }

    pub fn remove_service(&mut self, name: String) -> Result<(), ServicingError> {
        let mut services = self.services.lock()?;
        let service = services
            .remove(&name)
            .ok_or(ServicingError::ServiceNotFound(format!("{name} not found")))?;

        // Turn the orchestrator into a trait object
        let mut orchestrator = service.orchestrator.get_orchestrator();
        drop(services);

        // Run destroy
        orchestrator.remove(self.services.clone(), name)?;

        Ok(())
    }

    pub fn up(&mut self, name: String, skip_prompt: Option<bool>) -> Result<(), ServicingError> {
        let mut services = self.services.lock()?;
        let service = services
            .get_mut(&name)
            .ok_or(ServicingError::ServiceNotFound(format!("{name} not found")))?;

        // Turn the orchestrator into a trait object
        let mut orchestrator = service.orchestrator.get_orchestrator();
        drop(services);

        // Run up...
        let _guard = self.rt.enter();
        orchestrator.up(
            self.client.clone(),
            self.services.clone(),
            name,
            skip_prompt,
        )?;

        Ok(())
    }

    pub fn down(
        &mut self,
        name: String,
        skip_prompt: Option<bool>,
        force: Option<bool>,
    ) -> Result<(), ServicingError> {
        let mut services = self.services.lock()?;
        let service = services
            .get_mut(&name)
            .ok_or(ServicingError::ServiceNotFound(format!("{name} not found")))?;

        // Turn the orchestrator into a trait object
        let mut orchestrator = service.orchestrator.get_orchestrator();
        drop(services);

        // Run down...
        orchestrator.down(
            self.client.clone(),
            self.services.clone(),
            name,
            skip_prompt,
            force,
        )?;

        Ok(())
    }

    pub fn status(&self, name: String, pretty: Option<bool>) -> Result<String, ServicingError> {
        let services = self.services.lock()?;
        let service = services
            .get(&name)
            .ok_or(ServicingError::ServiceNotFound(format!("{name} not found")))?;

        // Turn the orchestrator into a trait object
        let mut orchestrator = service.orchestrator.get_orchestrator();
        drop(services);

        let _guard = self.rt.enter();
        orchestrator.status(self.client.clone(), self.services.clone(), name, pretty)
    }

    pub fn save(&self, location: Option<PathBuf>) -> Result<(), ServicingError> {
        let bin = bincode::serialize(&*self.services.lock()?)?;

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
        let bin = bincode::serialize(&*self.services.lock()?)?;
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

        self.services
            .lock()?
            .extend(bincode::deserialize::<HashMap<String, Service>>(&bin)?);

        if let Some(true) = update_status {
            info!("Checking for services that may come up while you were away...");

            // Clones to pass to threads
            let service_clone = self.services.clone();
            let client_clone = self.client.clone();
            let mut service_to_check = Vec::new();

            // iterate through the services and find that are down
            self.services
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
                            + &service.readiness_probe,
                        service
                            .orchestrator
                            .get_orchestrator()
                            .replica_check_string(),
                    ))
                });

            if service_to_check.is_empty() {
                info!("No services to check");
                return Ok(());
            }

            info!("Services to check: {:?}", service_to_check);

            self.rt.spawn(async move {
                let mut handles = Vec::new();
                for (name, url, replica_up_check) in service_to_check {
                    let client_clone = client_clone.clone();
                    let url = format!("http://{}", url);
                    let handle = tokio::spawn(async move {
                        match helper::fetch_and_check(
                            &client_clone,
                            &url,
                            replica_up_check,
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
        self.services
            .lock()?
            .extend(bincode::deserialize::<HashMap<String, Service>>(&bin)?);

        Ok(())
    }

    pub fn list(&self) -> Result<Vec<String>, ServicingError> {
        Ok(self.services.lock()?.keys().cloned().collect())
    }

    pub fn get_url(&self, name: String) -> Result<String, ServicingError> {
        if let Some(service) = self.services.lock()?.get(&name) {
            if let Some(url) = &service.url {
                return Ok(url.clone());
            }
            return Err(ServicingError::General("Service is down".to_string()));
        }
        Err(ServicingError::ServiceNotFound(name))
    }
}
