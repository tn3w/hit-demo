use serde::Deserialize;
use std::env;
use std::fs;
use std::path::Path;

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
    /// Cache directory for version information
    pub cache_dir: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            workers: 4,
            http_timeout: 3,
            version_check_interval: 1800,
            cache_dir: Some("./".to_string()),
        }
    }
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        Config::from_file(&args[1])
    } else {
        Ok(Config::default())
    }
}
