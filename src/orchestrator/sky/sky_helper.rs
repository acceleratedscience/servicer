use std::path::PathBuf;

use crate::{dispatch::helper, errors::Result};

use super::Configuration;

pub(super) fn get_template_from_path(path: &PathBuf) -> Result<Configuration> {
    let raw = helper::read_from_file_binary(path)?;
    let contents = String::from_utf8_lossy(&raw);
    Ok(serde_yaml::from_str::<Configuration>(&contents)?)
}
