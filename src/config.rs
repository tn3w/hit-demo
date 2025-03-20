use serde::Deserialize;
use std::env;
use std::fs;
use std::path::Path;
/// Application configuration structure
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    /// Server host address
    pub host: String,
    /// Server port number
    pub port: u16,
    /// Number of worker threads
    pub workers: usize,
    /// HTTP request timeout in seconds
    pub http_timeout: u64,
    /// Version check interval in seconds
    pub version_check_interval: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            workers: 4,
            http_timeout: 3,
            version_check_interval: 1800,
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    ///
    /// # Arguments
    /// * `path` - Path to the TOML configuration file
    ///
    /// # Returns
    /// * `Result<Config, Box<dyn std::error::Error>>` - Loaded configuration or an error
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Get the server address as a string
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Load configuration from command line arguments or use default
///
/// If an argument is provided, it is treated as the path to a TOML configuration file.
/// If no arguments are provided, default configuration is used.
pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        // Use the first argument as config file path
        Config::from_file(&args[1])
    } else {
        // Use default configuration
        Ok(Config::default())
    }
}
