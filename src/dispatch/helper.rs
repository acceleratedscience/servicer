//! Helper module houses all the helper functions used by the service module.
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::Receiver,
    thread::{spawn, JoinHandle},
    time::Duration,
};

use log::info;
use reqwest::{header::ACCEPT, Client};
use tokio::time::sleep;

use crate::errors::ServicingError;

/// check_python_package_installed checks if the user has installed the required python package.
/// True is returned if the package is installed, otherwise false.
pub fn check_python_package_installed(package: &str) -> bool {
    info!("Checking for python package: {}", package);
    let output = Command::new("pip").arg("show").arg(package).output();
    match output {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

pub fn create_directory(dirname: &str, home: bool) -> Result<PathBuf, ServicingError> {
    let dir_name = if home {
        match dirs::home_dir() {
            Some(path) => {
                info!("User home directory found: {:?}", path);
                Path::new(&path).join(dirname)
            }
            None => {
                return Err(ServicingError::General(
                    "User home directory not found".to_string(),
                ))
            }
        }
    } else {
        Path::new(dirname).to_path_buf()
    };
    // create a directory in provided parent directory
    match fs::create_dir(&dir_name) {
        Err(e) => match e.kind() {
            io::ErrorKind::AlreadyExists => {
                info!("Directory '{}' already exists.", dirname);
                Ok(dir_name)
            }
            _ => Err(e)?,
        },
        _ => {
            info!("Directory '{}' created successfully.", dirname);
            Ok(dir_name)
        }
    }
}

pub fn create_file(dirname: &PathBuf, filename: &str) -> Result<PathBuf, ServicingError> {
    // create a file in the provided directory
    let path = Path::new(dirname).join(filename);
    match fs::File::create(&path) {
        Ok(_) => {
            info!("File '{:?}' created successfully.", path);
            Ok(path)
        }
        Err(e) => Err(e)?,
    }
}

pub fn delete_file(filepath: &PathBuf) -> Result<(), ServicingError> {
    // delete a file in the provided directory
    match fs::remove_file(filepath) {
        Ok(_) => {
            info!(
                "Service configuration '{:?}' deleted successfully.",
                filepath
            );
            Ok(())
        }
        Err(e) => Err(e)?,
    }
}

pub fn write_to_file(filepath: &PathBuf, content: &str) -> Result<(), ServicingError> {
    // write content to a file in the provided file
    match fs::write(filepath, content) {
        Ok(_) => {
            info!("Content written to file '{:?}' successfully.", filepath);
            Ok(())
        }
        Err(e) => Err(e)?,
    }
}

pub fn write_to_file_binary(
    filepath: &PathBuf,
    content: &[u8],
) -> Result<(), ServicingError> {
    // write content to a file in the provided file
    match fs::write(filepath, content) {
        Ok(_) => {
            info!("Content written to file '{:?}' successfully.", filepath);
            Ok(())
        }
        Err(e) => Err(e)?,
    }
}

pub fn read_from_file_binary(filepath: &PathBuf) -> Result<Vec<u8>, ServicingError> {
    // read content from a file in the provided file
    match fs::read(filepath) {
        Ok(content) => {
            info!("Content read from file '{:?}' successfully.", filepath);
            Ok(content)
        }
        Err(e) => Err(e)?,
    }
}

#[allow(dead_code)]
pub fn read_from_child<T>(
    mut child: T,
) -> (Receiver<Vec<u8>>, JoinHandle<Result<(), ServicingError>>)
where
    T: Read + Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let handle = spawn(move || {
        loop {
            let mut buffer = [0; 2];

            match child.read_exact(&mut buffer) {
                Ok(_) => {
                    if tx.send(buffer.to_vec()).is_err() {
                        log::warn!("Failed to send data to the receiver.");
                        return Err(ServicingError::General(
                            "Failed to send data to the receiver.".to_string(),
                        ));
                    }
                }
                Err(e) => {
                    if e.kind() == io::ErrorKind::UnexpectedEof {
                        info!("End of file reached.");
                        if tx.send(buffer.to_vec()).is_err() {
                            log::warn!("Failed to send data to the receiver.");
                            return Err(ServicingError::General(
                                "Failed to send data to the receiver.".to_string(),
                            ));
                        }
                        break;
                    } else {
                        return Err(ServicingError::General(e.to_string()));
                    }
                }
            }
        }
        Ok(())
    });
    (rx, handle)
}

pub async fn fetch(client: &Client, url: &str) -> Result<String, reqwest::Error> {
    let res = client
        .get(url)
        .header(ACCEPT, "application/json")
        .send()
        .await?;
    let body = res.text().await?;
    Ok(body)
}

pub async fn fetch_and_check(
    client: &Client,
    url: &str,
    expected: &str,
    delay: Option<Duration>,
) -> Result<(), ServicingError> {
    loop {
        let res = client
            .get(url)
            .header(ACCEPT, "application/json")
            .send()
            .await?;
        let body = res.text().await?;

        if !body.to_lowercase().contains(expected) {
            break;
        }

        if let Some(delay) = delay {
            sleep(delay).await;
        }
    }

    Ok(())
}
