use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use regex::Regex;
use reqwest;
use reqwest::header::{CACHE_CONTROL, EXPIRES, PRAGMA};
use serde::{Deserialize, Serialize};
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
#[derive(Clone, Serialize, Deserialize)]
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
    all_versions: Arc<RwLock<Vec<VersionInfo>>>,
    latest_version: Arc<RwLock<VersionInfo>>,
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
            all_versions: Arc::new(RwLock::new(Vec::new())),
            latest_version: Arc::new(RwLock::new(VersionInfo {
                version: String::new(),
                sri_hash: String::new(),
            })),
            http_timeout_secs,
            version_check_interval_secs,
        }
    }

    /// Checks all available versions of the package from jsDelivr.
    ///
    /// # Returns
    /// * `Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>` - List of all versions or an error
    async fn check_all_versions(
        &self,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
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

        let mut versions = Vec::new();

        // Get all versions from versions array
        if let Some(version_array) = json["versions"].as_array() {
            for version_value in version_array {
                if let Some(version) = version_value.as_str() {
                    versions.push(version.to_string());
                }
            }
        }

        // Make sure "latest" tag is included
        if let Some(tags) = json["tags"].as_object() {
            if let Some(latest) = tags.get("latest") {
                if let Some(version) = latest.as_str() {
                    if !versions.contains(&version.to_string()) {
                        versions.push(version.to_string());
                    }
                }
            }
        }

        if versions.is_empty() {
            #[cfg(debug_assertions)]
            println!("Debug: No versions found in response: {:?}", json);
            return Err("No versions found in response".into());
        }

        #[cfg(debug_assertions)]
        println!("Debug: Found {} versions", versions.len());

        Ok(versions)
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
        let all_versions = Arc::clone(&self.all_versions);
        let latest_version = Arc::clone(&self.latest_version);
        let version_info = Arc::clone(&self.current_version_info);
        let checker = self.clone();

        // Initial version check
        if let Ok(versions) = checker.check_all_versions().await {
            let mut stored_versions = Vec::new();

            // Get the first version as latest (they come in descending order from jsDelivr)
            if let Some(latest_version_str) = versions.first() {
                // Calculate SRI hash for the latest version
                if let Ok(sri_hash) = checker.calculate_sri_hash(latest_version_str).await {
                    let latest_version_info = VersionInfo {
                        version: latest_version_str.clone(),
                        sri_hash: sri_hash.clone(),
                    };

                    // Update current version (for backwards compatibility)
                    let mut current = version_info.write().await;
                    current.version = latest_version_str.clone();
                    current.sri_hash = sri_hash.clone();

                    // Update latest version
                    let mut latest = latest_version.write().await;
                    *latest = latest_version_info.clone();

                    #[cfg(debug_assertions)]
                    println!(
                        "Debug: Initial version set to {} with hash {}",
                        latest_version_str, sri_hash
                    );

                    // Add to stored versions
                    stored_versions.push(latest_version_info);
                } else {
                    #[cfg(debug_assertions)]
                    println!(
                        "Debug: Failed to calculate SRI hash for version {}",
                        latest_version_str
                    );
                }
            }

            // Process the remaining versions (limit to 10 most recent for performance)
            for version_str in versions.iter().skip(1).take(9) {
                if let Ok(sri_hash) = checker.calculate_sri_hash(version_str).await {
                    stored_versions.push(VersionInfo {
                        version: version_str.clone(),
                        sri_hash,
                    });
                }
            }

            // Store all processed versions
            let mut all = all_versions.write().await;
            *all = stored_versions;
        } else {
            #[cfg(debug_assertions)]
            println!("Debug: Failed to get initial versions");
        }

        // Spawn background task
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(checker.version_check_interval_secs)).await;

                match checker.check_all_versions().await {
                    Ok(versions) => {
                        // Get the first version as latest (they come in descending order from jsDelivr)
                        if let Some(new_version_str) = versions.first() {
                            // Check if version is different from current one
                            let current_latest = {
                                let latest = latest_version.read().await;
                                latest.version.clone()
                            };

                            if new_version_str != &current_latest {
                                // Calculate SRI hash for the new version
                                if let Ok(sri_hash) =
                                    checker.calculate_sri_hash(new_version_str).await
                                {
                                    let new_version_info = VersionInfo {
                                        version: new_version_str.clone(),
                                        sri_hash: sri_hash.clone(),
                                    };

                                    // Update current version (for backwards compatibility)
                                    let mut current = version_info.write().await;
                                    current.version = new_version_str.clone();
                                    current.sri_hash = sri_hash.clone();

                                    // Update latest version
                                    let mut latest = latest_version.write().await;
                                    *latest = new_version_info.clone();

                                    // Add to stored versions if not already present
                                    let mut all = all_versions.write().await;
                                    if !all.iter().any(|v| v.version == new_version_str.clone()) {
                                        all.insert(0, new_version_info);
                                    }

                                    #[cfg(debug_assertions)]
                                    println!(
                                        "Debug: Updated version to {} with hash {}",
                                        new_version_str, sri_hash
                                    );
                                } else {
                                    #[cfg(debug_assertions)]
                                    println!(
                                        "Debug: Failed to calculate SRI hash for version {}",
                                        new_version_str
                                    );
                                }
                            }

                            // Check for any versions we don't have yet and add them
                            let mut need_update = false;
                            {
                                let all = all_versions.read().await;
                                let existing_versions: Vec<String> =
                                    all.iter().map(|v| v.version.clone()).collect();

                                for version_str in versions.iter().take(10) {
                                    // Limit to 10 versions
                                    if !existing_versions.contains(version_str) {
                                        need_update = true;
                                        break;
                                    }
                                }
                            }

                            if need_update {
                                let mut processed_versions = Vec::new();

                                // Get existing versions first
                                {
                                    let all = all_versions.read().await;
                                    processed_versions.extend(all.iter().cloned());
                                }

                                // Get versions we don't have yet
                                for version_str in versions.iter().take(10) {
                                    if !processed_versions.iter().any(|v| &v.version == version_str)
                                    {
                                        if let Ok(sri_hash) =
                                            checker.calculate_sri_hash(version_str).await
                                        {
                                            processed_versions.push(VersionInfo {
                                                version: version_str.clone(),
                                                sri_hash,
                                            });
                                        }
                                    }
                                }

                                // Sort versions by semantic versioning (newest first)
                                processed_versions.sort_by(|a, b| {
                                    // Simple version comparison for 3-component versions
                                    let parse_version = |v: &str| -> (u32, u32, u32) {
                                        let parts: Vec<&str> = v.split('.').collect();
                                        if parts.len() == 3 {
                                            (
                                                parts[0].parse().unwrap_or(0),
                                                parts[1].parse().unwrap_or(0),
                                                parts[2].parse().unwrap_or(0),
                                            )
                                        } else {
                                            (0, 0, 0)
                                        }
                                    };

                                    let a_ver = parse_version(&a.version);
                                    let b_ver = parse_version(&b.version);
                                    b_ver.cmp(&a_ver) // Newest first
                                });

                                // Update stored versions
                                let mut all = all_versions.write().await;
                                *all = processed_versions;
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

    /// Gets the latest version info of the package.
    ///
    /// # Returns
    /// * `VersionInfo` - The latest version information
    pub async fn get_latest_version_info(&self) -> VersionInfo {
        self.latest_version.read().await.clone()
    }

    /// Gets all stored version infos of the package.
    ///
    /// # Returns
    /// * `Vec<VersionInfo>` - All stored version information
    pub async fn get_all_versions(&self) -> Vec<VersionInfo> {
        self.all_versions.read().await.clone()
    }
}

impl Clone for VersionChecker {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            package_name: self.package_name.clone(),
            current_version_info: Arc::clone(&self.current_version_info),
            all_versions: Arc::clone(&self.all_versions),
            latest_version: Arc::clone(&self.latest_version),
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
