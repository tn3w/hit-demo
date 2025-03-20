use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use regex::Regex;
use reqwest;
use reqwest::header::{CACHE_CONTROL, EXPIRES, PRAGMA};
use serde_json::Value;
use sha2::{Digest, Sha512};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
const NO_CACHE_HEADERS: [(reqwest::header::HeaderName, &str); 3] = [
    (CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
    (PRAGMA, "no-cache"),
    (EXPIRES, "0"),
];

const VERSION_REGEX: &str = r"^[0-9]\.[0-9]{1,2}\.[0-9]{1,2}$";

/// Version information containing the version string and SRI hash
#[derive(Clone)]
pub struct VersionInfo {
    pub version: String,
    pub sri_hash: String,
}

/// A version checker that periodically checks for updates of an NPM package.
///
/// This struct maintains a background task that checks for new versions every 30 minutes
/// and provides the current version information on demand.
pub struct VersionChecker {
    client: reqwest::Client,
    package_name: String,
    current_version_info: Arc<RwLock<VersionInfo>>,
    http_timeout_secs: u64,
    version_check_interval_secs: u64,
}

impl VersionChecker {
    /// Creates a new VersionChecker instance for the specified package.
    ///
    /// # Arguments
    /// * `package_name` - The name of the NPM package to check
    /// * `http_timeout_secs` - Timeout duration for HTTP requests in seconds
    /// * `version_check_interval_secs` - Interval between version checks in seconds
    pub fn new(
        package_name: &str,
        http_timeout_secs: u64,
        version_check_interval_secs: u64,
    ) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        for (key, value) in NO_CACHE_HEADERS.iter() {
            headers.insert(
                key.clone(),
                reqwest::header::HeaderValue::from_static(value),
            );
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(http_timeout_secs))
            .user_agent("VersionChecker/1.0")
            .default_headers(headers)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            package_name: package_name.to_string(),
            current_version_info: Arc::new(RwLock::new(VersionInfo {
                version: String::new(),
                sri_hash: String::new(),
            })),
            http_timeout_secs,
            version_check_interval_secs,
        }
    }

    /// Checks the current version of the package from jsDelivr.
    ///
    /// # Returns
    /// * `Result<String, Box<dyn std::error::Error + Send + Sync>>` - The latest version string or an error
    async fn check_version(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://data.jsdelivr.com/v1/package/npm/{}",
            self.package_name
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let json: Value = response
            .json()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Try to get version from latest tag first
        if let Some(tags) = json["tags"].as_object() {
            if let Some(latest) = tags.get("latest") {
                if let Some(version) = latest.as_str() {
                    #[cfg(debug_assertions)]
                    println!("Debug: Found version {} from latest tag", version);
                    return Ok(version.to_string());
                }
            }
        }

        // Fallback to first version in versions array
        if let Some(versions) = json["versions"].as_array() {
            if let Some(latest) = versions.first() {
                if let Some(version) = latest.as_str() {
                    #[cfg(debug_assertions)]
                    println!("Debug: Found version {} from versions array", version);
                    return Ok(version.to_string());
                }
            }
        }

        #[cfg(debug_assertions)]
        println!("Debug: No version found in response: {:?}", json);

        Err("No version found in response".into())
    }

    /// Download minified JS file and calculate SRI hash
    ///
    /// # Arguments
    /// * `version` - The version to download
    ///
    /// # Returns
    /// * `Result<String, Box<dyn std::error::Error + Send + Sync>>` - The SRI hash or an error
    async fn calculate_sri_hash(
        &self,
        version: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://cdn.jsdelivr.net/npm/{}@{}/dist/{}-min.js",
            self.package_name, version, self.package_name
        );

        #[cfg(debug_assertions)]
        println!("Debug: Downloading from {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Calculate SHA-512 hash
        let mut hasher = Sha512::new();
        hasher.update(&bytes);
        let hash = hasher.finalize();

        // Base64 encode the hash using the recommended approach
        let hash_base64 = BASE64.encode(hash);

        // Return the SRI hash format
        let sri_hash = format!("sha512-{}", hash_base64);

        #[cfg(debug_assertions)]
        println!(
            "Debug: Calculated SRI hash for version {}: {}",
            version, sri_hash
        );

        Ok(sri_hash)
    }

    /// Starts the background version checking task.
    ///
    /// This will immediately check for the current version and then start a background task
    /// that checks every 30 minutes for updates.
    pub async fn start_checking(&self) {
        let version_info = Arc::clone(&self.current_version_info);
        let checker = self.clone();

        // Initial version check
        if let Ok(new_version) = checker.check_version().await {
            // Calculate SRI hash for the new version
            if let Ok(sri_hash) = checker.calculate_sri_hash(&new_version).await {
                let mut info = version_info.write().await;
                info.version = new_version.clone();
                info.sri_hash = sri_hash.clone();

                #[cfg(debug_assertions)]
                println!(
                    "Debug: Initial version set to {} with hash {}",
                    new_version, sri_hash
                );
            } else {
                #[cfg(debug_assertions)]
                println!(
                    "Debug: Failed to calculate SRI hash for version {}",
                    new_version
                );
            }
        } else {
            #[cfg(debug_assertions)]
            println!("Debug: Failed to get initial version");
        }

        // Spawn background task
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(checker.version_check_interval_secs)).await;

                match checker.check_version().await {
                    Ok(new_version) => {
                        // Check if version is different from current one
                        let current_version = {
                            let info = version_info.read().await;
                            info.version.clone()
                        };

                        if new_version != current_version {
                            // Calculate SRI hash for the new version
                            if let Ok(sri_hash) = checker.calculate_sri_hash(&new_version).await {
                                let mut info = version_info.write().await;
                                info.version = new_version.clone();
                                info.sri_hash = sri_hash.clone();

                                #[cfg(debug_assertions)]
                                println!(
                                    "Debug: Updated version to {} with hash {}",
                                    new_version, sri_hash
                                );
                            } else {
                                #[cfg(debug_assertions)]
                                println!(
                                    "Debug: Failed to calculate SRI hash for version {}",
                                    new_version
                                );
                            }
                        }
                    }
                    #[cfg(debug_assertions)]
                    Err(e) => {
                        println!("Debug: Error checking version: {}", e);
                    }
                    #[cfg(not(debug_assertions))]
                    Err(_) => {
                        // No debug output in release mode
                    }
                }
            }
        });
    }

    /// Gets the current version info of the package.
    ///
    /// # Returns
    /// * `VersionInfo` - The current version information
    pub async fn get_current_version_info(&self) -> VersionInfo {
        self.current_version_info.read().await.clone()
    }
}

impl Clone for VersionChecker {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            package_name: self.package_name.clone(),
            current_version_info: Arc::clone(&self.current_version_info),
            http_timeout_secs: self.http_timeout_secs,
            version_check_interval_secs: self.version_check_interval_secs,
        }
    }
}

pub fn is_valid_version(version: &str) -> bool {
    if version.len() > 7 {
        return false;
    }

    let regex = Regex::new(VERSION_REGEX).unwrap();
    regex.is_match(version)
}
