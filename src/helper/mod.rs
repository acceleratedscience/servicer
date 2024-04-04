use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

use log::info;

use crate::error::ServicingError;

/// check_python_package_installed checks if the user has installed the required python package.
/// True is returned if the package is installed, otherwise false.
pub(super) fn check_python_package_installed(package: &str) -> bool {
    info!("Checking for python package: {}", package);
    let output = Command::new("pip").arg("show").arg(package).output();
    match output {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

pub(super) fn create_directory(dirname: &str, home: bool) -> Result<PathBuf, ServicingError> {
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

pub(super) fn create_file(dirname: &PathBuf, filename: &str) -> Result<PathBuf, ServicingError> {
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

pub(super) fn write_to_file(filepath: &PathBuf, content: &str) -> Result<(), ServicingError> {
    // write content to a file in the provided file
    match fs::write(filepath, content) {
        Ok(_) => {
            info!("Content written to file '{:?}' successfully.", filepath);
            Ok(())
        }
        Err(e) => Err(e)?,
    }
}
