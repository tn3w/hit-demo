use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use regex::Regex;
use reqwest;
use reqwest::header::{CACHE_CONTROL, EXPIRES, PRAGMA};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha512};
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

const NO_CACHE_HEADERS: [(reqwest::header::HeaderName, &str); 3] = [
    (CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
    (PRAGMA, "no-cache"),
    (EXPIRES, "0"),
];

const VERSION_REGEX: &str = r"^[0-9]\.[0-9]{1,2}\.[0-9]{1,2}$";
const CACHE_FILE_NAME: &str = "version_cache.json";

#[derive(Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub sri_hash: String,
}

pub struct VersionChecker {
    client: reqwest::Client,
    package_name: String,
    current_version_info: Arc<RwLock<VersionInfo>>,
    all_versions: Arc<RwLock<Vec<VersionInfo>>>,
    latest_version: Arc<RwLock<VersionInfo>>,
    http_timeout_secs: u64,
    version_check_interval_secs: u64,
    cache_file_path: String,
}

impl VersionChecker {
    pub fn new(
        package_name: &str,
        http_timeout_secs: u64,
        version_check_interval_secs: u64,
        cache_dir: Option<&str>,
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

        let cache_file_path = match cache_dir {
            Some(dir) => format!("{}/{}-{}", dir, package_name, CACHE_FILE_NAME),
            None => format!("{}-{}", package_name, CACHE_FILE_NAME),
        };

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
            cache_file_path,
        }
    }

    async fn load_cache(&self) -> Vec<VersionInfo> {
        let path = Path::new(&self.cache_file_path);
        if !path.exists() {
            return Vec::new();
        }

        match fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<Vec<VersionInfo>>(&content) {
                Ok(versions) => {
                    #[cfg(debug_assertions)]
                    println!("Debug: Loaded {} versions from cache", versions.len());
                    versions
                }
                #[cfg(debug_assertions)]
                Err(e) => {
                    println!("Debug: Failed to parse cache file: {}", e);
                    Vec::new()
                }
                #[cfg(not(debug_assertions))]
                Err(_) => Vec::new(),
            },
            #[cfg(debug_assertions)]
            Err(e) => {
                println!("Debug: Failed to read cache file: {}", e);
                Vec::new()
            }
            #[cfg(not(debug_assertions))]
            Err(_) => Vec::new(),
        }
    }

    async fn save_cache(&self, versions: &[VersionInfo]) -> io::Result<()> {
        if let Some(parent) = Path::new(&self.cache_file_path).parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        let json = serde_json::to_string_pretty(versions)?;
        fs::write(&self.cache_file_path, json)?;

        #[cfg(debug_assertions)]
        println!("Debug: Saved {} versions to cache", versions.len());

        Ok(())
    }

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

        if let Some(version_array) = json["versions"].as_array() {
            for version_value in version_array {
                if let Some(version) = version_value.as_str() {
                    versions.push(version.to_string());
                }
            }
        }

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

        let mut hasher = Sha512::new();
        hasher.update(&bytes);
        let hash = hasher.finalize();

        let hash_base64 = BASE64.encode(hash);
        let sri_hash = format!("sha512-{}", hash_base64);

        #[cfg(debug_assertions)]
        println!(
            "Debug: Calculated SRI hash for version {}: {}",
            version, sri_hash
        );

        Ok(sri_hash)
    }

    pub async fn start_checking(&self) {
        let cached_versions = self.load_cache().await;

        if !cached_versions.is_empty() {
            let latest_version = cached_versions
                .first()
                .cloned()
                .unwrap_or_else(|| VersionInfo {
                    version: String::new(),
                    sri_hash: String::new(),
                });

            {
                let mut all = self.all_versions.write().await;
                *all = cached_versions.clone();

                let mut current = self.current_version_info.write().await;
                *current = latest_version.clone();

                let mut latest = self.latest_version.write().await;
                *latest = latest_version;

                #[cfg(debug_assertions)]
                println!("Debug: Initialized from cache with {} versions", all.len());
            }
        }

        let all_versions = Arc::clone(&self.all_versions);
        let latest_version = Arc::clone(&self.latest_version);
        let version_info = Arc::clone(&self.current_version_info);
        let checker = self.clone();

        let check_versions = {
            let checker = checker.clone();
            let all_versions = all_versions.clone();
            let latest_version = latest_version.clone();
            let version_info = version_info.clone();
            
            async move {
                if let Ok(versions) = checker.check_all_versions().await {
                    let mut stored_versions = Vec::new();
                    
                    let all = all_versions.read().await;
                    let existing_versions: Vec<String> = all.iter().map(|v| v.version.clone()).collect();
                    
                    if !all.is_empty() {
                        stored_versions = all.clone();
                    }
                    drop(all);
                    
                    if let Some(latest_version_str) = versions.first() {
                        if !existing_versions.contains(latest_version_str) {
                            if let Ok(sri_hash) = checker.calculate_sri_hash(latest_version_str).await {
                                let latest_version_info = VersionInfo {
                                    version: latest_version_str.clone(),
                                    sri_hash: sri_hash.clone(),
                                };

                                let mut current = version_info.write().await;
                                current.version = latest_version_str.clone();
                                current.sri_hash = sri_hash.clone();

                                let mut latest = latest_version.write().await;
                                *latest = latest_version_info.clone();

                                if !stored_versions.iter().any(|v| v.version == latest_version_str.clone()) {
                                    stored_versions.insert(0, latest_version_info);
                                }

                                #[cfg(debug_assertions)]
                                println!(
                                    "Debug: Initial version set to {} with hash {}",
                                    latest_version_str, sri_hash
                                );
                            }
                        }
                    }

                    for version_str in versions.iter().skip(1).take(9) {
                        if !existing_versions.contains(version_str) {
                            if let Ok(sri_hash) = checker.calculate_sri_hash(version_str).await {
                                if !stored_versions.iter().any(|v| v.version == version_str.clone()) {
                                    stored_versions.push(VersionInfo {
                                        version: version_str.clone(),
                                        sri_hash,
                                    });
                                }
                            }
                        }
                    }

                    if stored_versions.len() > existing_versions.len() {
                        let mut all = all_versions.write().await;
                        *all = stored_versions.clone();
                        
                        all.sort_by(|a, b| {
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
                            b_ver.cmp(&a_ver)
                        });

                        #[cfg(debug_assertions)]
                        if let Err(e) = checker.save_cache(&all).await {
                            println!("Debug: Failed to save cache: {}", e);
                        }
                        #[cfg(not(debug_assertions))]
                        let _ = checker.save_cache(&all).await;
                    }
                } else {
                    #[cfg(debug_assertions)]
                    println!("Debug: Failed to get initial versions");
                }
            }
        };
        
        if cached_versions.is_empty() {
            check_versions.await;
        } else {
            tokio::spawn(check_versions);
        }

        let checker_periodic = checker.clone();
        let all_versions_periodic = all_versions.clone();
        let latest_version_periodic = latest_version.clone();
        let version_info_periodic = version_info.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(checker_periodic.version_check_interval_secs)).await;

                match checker_periodic.check_all_versions().await {
                    Ok(versions) => {
                        if let Some(new_version_str) = versions.first() {
                            let current_latest = {
                                let latest = latest_version_periodic.read().await;
                                latest.version.clone()
                            };

                            if new_version_str != &current_latest {
                                if let Ok(sri_hash) =
                                    checker_periodic.calculate_sri_hash(new_version_str).await
                                {
                                    let new_version_info = VersionInfo {
                                        version: new_version_str.clone(),
                                        sri_hash: sri_hash.clone(),
                                    };

                                    let mut current = version_info_periodic.write().await;
                                    current.version = new_version_str.clone();
                                    current.sri_hash = sri_hash.clone();

                                    let mut latest = latest_version_periodic.write().await;
                                    *latest = new_version_info.clone();

                                    let mut all = all_versions_periodic.write().await;
                                    if !all.iter().any(|v| v.version == new_version_str.clone()) {
                                        all.insert(0, new_version_info);

                                        #[cfg(debug_assertions)]
                                        if let Err(e) = checker_periodic.save_cache(&all).await {
                                            println!("Debug: Failed to save cache: {}", e);
                                        }
                                        #[cfg(not(debug_assertions))]
                                        let _ = checker_periodic.save_cache(&all).await;
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

                            let mut new_versions_to_process = Vec::new();
                            
                            let all = all_versions_periodic.read().await;
                            let existing_versions: Vec<String> = 
                                all.iter().map(|v| v.version.clone()).collect();

                            for version_str in versions.iter().take(10) {
                                if !existing_versions.contains(version_str) {
                                    new_versions_to_process.push(version_str.clone());
                                }
                            }
                            drop(all);

                            if !new_versions_to_process.is_empty() {
                                let mut all = all_versions_periodic.write().await;
                                let mut cache_updated = false;

                                for version_str in new_versions_to_process {
                                    if !all.iter().any(|v| v.version == version_str) {
                                        if let Ok(sri_hash) =
                                            checker_periodic.calculate_sri_hash(&version_str).await
                                        {
                                            let version_info = VersionInfo {
                                                version: version_str.clone(),
                                                sri_hash,
                                            };
                                            all.push(version_info);
                                            cache_updated = true;
                                        }
                                    }
                                }

                                all.sort_by(|a, b| {
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
                                    b_ver.cmp(&a_ver)
                                });

                                if cache_updated {
                                    #[cfg(debug_assertions)]
                                    if let Err(e) = checker_periodic.save_cache(&all).await {
                                        println!("Debug: Failed to save cache: {}", e);
                                    }
                                    #[cfg(not(debug_assertions))]
                                    let _ = checker_periodic.save_cache(&all).await;
                                }
                            }
                        }
                    }
                    #[cfg(debug_assertions)]
                    Err(e) => {
                        println!("Debug: Error checking version: {}", e);
                    }
                    #[cfg(not(debug_assertions))]
                    Err(_) => {}
                }
            }
        });
    }

    pub async fn get_current_version_info(&self) -> VersionInfo {
        self.current_version_info.read().await.clone()
    }

    pub async fn get_latest_version_info(&self) -> VersionInfo {
        self.latest_version.read().await.clone()
    }

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
            cache_file_path: self.cache_file_path.clone(),
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

pub fn get_versions_selector(
    all_versions: Vec<VersionInfo>,
    latest_version: String,
    selected_version: Option<String>,
) -> String {
    let mut versions_html = String::from(
        "<select id=\"version-selector\" onchange=\"window.location.href='/' + this.value; document.getElementById('current-version-display').textContent = this.options[this.selectedIndex].text;\">",
    );

    let latest_selected = selected_version.is_none()
        || selected_version
            .as_ref()
            .map_or(false, |s| s == &latest_version);
    let latest_selected_attr = if latest_selected { " selected" } else { "" };

    versions_html.push_str(&format!(
        "<option value=\"\"{}>Latest ({})</option>",
        latest_selected_attr, latest_version
    ));

    for v in all_versions {
        if v.version == latest_version {
            continue;
        }

        let selected = match &selected_version {
            Some(selected) if *selected == v.version => " selected",
            _ => "",
        };

        versions_html.push_str(&format!(
            "<option value=\"{}\"{}>{}</option>",
            v.version, selected, v.version
        ));
    }

    versions_html.push_str("</select>");
    versions_html
}
