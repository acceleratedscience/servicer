use std::{path::PathBuf, process::Command, sync::OnceLock, time::Duration};

use log::{error, info, warn};
use regex::Regex;
use reqwest::Client;
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use tokio::{runtime::Handle, time::sleep};

use crate::{
    dispatch::{helper, ServiceCache},
    errors::{Result, ServicingError},
    models::{Orchestrator, UserProvidedConfig},
};

mod sky_helper;

static CLUSTER_ORCHESTRATOR: &str = "skypilot";
static REGEX_URL: OnceLock<Regex> = OnceLock::new();
static SERVICE_CHECK_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Default, Debug)]
pub struct Sky {
    template: Configuration,
}

impl Orchestrator for Sky {
    fn setup(
        &mut self,
        _cache: ServiceCache,
        pwd: PathBuf,
        name: String,
        userconfig: Option<&UserProvidedConfig>,
    ) -> Result<PathBuf> {
        if !helper::check_python_package_installed(CLUSTER_ORCHESTRATOR) {
            return Err(ServicingError::PipPackageError(CLUSTER_ORCHESTRATOR));
        }

        let re = Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}:\d+\b")?;
        let _ = REGEX_URL.get_or_init(|| re);

        if let Some(config) = userconfig {
            self.template.update(config);
        }

        let content = serde_yaml::to_string(&self.template)?;
        let file = helper::create_file(&pwd, &(name.clone() + "_service.yaml"))?;
        helper::write_to_file(&file, &content)?;

        Ok(file)
    }

    fn remove(&mut self, cache: ServiceCache, name: String) -> Result<()> {
        let mut service = cache.lock()?;
        if let Some(service) = service.get(&name) {
            if service.up {
                return Err(ServicingError::ClusterProvisionError(format!(
                    "Service {} is still up",
                    name
                )));
            }
            // check if service is not yet up but started
            if service.url.is_some() {
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

    fn update(&mut self, _cache: ServiceCache, _name: String) -> Result<()> {
        // noop for now
        Ok(())
    }

    fn up(
        &mut self,
        client: Client,
        cache: ServiceCache,
        name: String,
        skip_prompt: Option<bool>,
    ) -> Result<()> {
        if let Some(service) = cache.lock()?.get_mut(&name) {
            if service.url.is_some() {
                return Err(ServicingError::ClusterProvisionError(format!(
                    "Service {} is already running",
                    name
                )));
            }

            info!("Launching service with configuration from: {}", name);

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
            let cache_clone = cache.clone();

            let url = url.to_string() + &self.template.service.readiness_probe;
            let replica_check_string = self.replica_check_string();

            // spawn a green thread to check when service comes online, then update the service status
            let fut = async move {
                let url = format!("http://{}", url);
                loop {
                    match helper::fetch(&client, &url).await {
                        Ok(resp) => {
                            if resp.to_lowercase().contains(replica_check_string) {
                                sleep(SERVICE_CHECK_INTERVAL).await;
                                continue;
                            }
                            match cache_clone.lock() {
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
            tokio::spawn(fut);

            return Ok(());
        }
        Err(ServicingError::ServiceNotFound(name))
    }

    fn down(
        &mut self,
        _client: Client,
        cache: ServiceCache,
        name: String,
        skip_prompt: Option<bool>,
        force: Option<bool>,
    ) -> Result<()> {
        match cache.lock()?.get_mut(&name) {
            Some(service) if service.up || service.url.is_some() => {
                service.url = None;
                service.up = false;
            }
            Some(_) => {
                if let Some(false) | None = force {
                    return Err(ServicingError::ServiceNotUp(name));
                }
            }
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

    fn status(
        &mut self,
        client: Client,
        cache: ServiceCache,
        name: String,
        pretty: Option<bool>,
    ) -> Result<String> {
        // Check if the service exists
        if let Some(service) = cache.lock()?.get_mut(&name) {
            // retrieve the service from the yaml
            self.template = sky_helper::get_template_from_path(
                service
                    .filepath
                    .as_ref()
                    .ok_or(ServicingError::General("filepath not found".to_string()))?,
            )?;

            info!("Checking the status of the service: {:?}", name);

            // if service is up poll once to see if it's still up
            if let (true, Some(url)) = (service.up, &service.url) {
                let url = format!("http://{}{}", url, self.template.service.readiness_probe);

                let handle = Handle::try_current()?;
                let r = handle.block_on(async {
                    let res = helper::fetch(&client, &url).await;
                    match res {
                        Ok(resp) => {
                            if resp.to_lowercase().contains(self.replica_check_string()) {
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

    fn replica_check_string(&self) -> &'static str {
        "no ready replicas"
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
        stats.serialize_field("ports", &self.ports)?;
        if should_serialize {
            stats.serialize_field("accelerators", &self.accelerators)?;
        }
        stats.end()
    }
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
            self.resources.cloud.clone_from(cloud);
        }
        if let Some(workdir) = &config.workdir {
            self.workdir.clone_from(workdir);
        }
        if let Some(disk_size) = config.disk_size {
            self.resources.disk_size = disk_size;
        }
        if let Some(cpu) = &config.cpu {
            self.resources.cpus.clone_from(cpu);
        }
        if let Some(memory) = &config.memory {
            self.resources.memory.clone_from(memory);
        }
        if let Some(setup) = &config.setup {
            self.setup.clone_from(setup);
        }
        if let Some(run) = &config.run {
            self.run.clone_from(run);
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
