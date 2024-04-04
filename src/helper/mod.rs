use std::process::Command;

use log::info;

pub(super) fn check_python_package(package: &str) -> bool {
    info!("Checking for python package: {}", package);
    let output = Command::new("pip").arg("show").arg(package).output();
    match output {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}
