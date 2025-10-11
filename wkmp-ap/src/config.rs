//! wkmp-ap specific configuration

use std::path::PathBuf;

/// Audio Player configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub root_folder: PathBuf,
    pub db_path: PathBuf,
    pub bind_addr: String,
}
