use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Types of assets that can be served
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssetType {
    JavaScript,
    CSS,
}

/// Represents a loaded asset with its content and type
#[derive(Debug, Clone)]
pub struct Asset {
    pub content: String,
    pub asset_type: AssetType,
}

/// Asset manager that loads and caches static files and templates
pub struct AssetManager {
    static_assets: Arc<RwLock<HashMap<String, Asset>>>,
    templates: Arc<RwLock<HashMap<String, String>>>,
}

impl AssetManager {
    /// Create a new AssetManager and load all static and template files
    pub async fn new() -> Result<Self, std::io::Error> {
        let manager = Self {
            static_assets: Arc::new(RwLock::new(HashMap::new())),
            templates: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load all static files
        manager.load_static_files().await?;

        // Load all template files
        manager.load_template_files().await?;

        Ok(manager)
    }

    /// Load all static files from the static directory
    async fn load_static_files(&self) -> Result<(), std::io::Error> {
        // Get list of all files in static directory
        let static_dir = PathBuf::from("static");
        let entries = fs::read_dir(&static_dir)?;

        let mut assets = self.static_assets.write().await;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let filename =
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .ok_or_else(|| {
                            std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid filename")
                        })?;

                // Only process .js and .css files
                if filename.ends_with(".js") || filename.ends_with(".css") {
                    let content = fs::read_to_string(&path)?;

                    // Determine asset type based on file extension
                    let asset_type = if filename.ends_with(".js") {
                        AssetType::JavaScript
                    } else {
                        AssetType::CSS
                    };

                    // Store asset in memory (with both original and minified reference)
                    assets.insert(
                        filename.to_string(),
                        Asset {
                            content,
                            asset_type,
                        },
                    );

                    // Also store a reference for the minified version (client might request .min.js/.min.css)
                    let min_filename = if filename.ends_with(".js") {
                        filename.replace(".js", ".min.js")
                    } else {
                        filename.replace(".css", ".min.css")
                    };

                    println!(
                        "Loaded static asset: {} (also available as {})",
                        filename, min_filename
                    );
                }
            }
        }

        Ok(())
    }

    /// Load all template files from the templates directory
    async fn load_template_files(&self) -> Result<(), std::io::Error> {
        // Get list of all files in templates directory
        let templates_dir = PathBuf::from("templates");
        let entries = fs::read_dir(&templates_dir)?;

        let mut templates = self.templates.write().await;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let filename =
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .ok_or_else(|| {
                            std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid filename")
                        })?;

                // Only process .html files
                if filename.ends_with(".html") {
                    let content = fs::read_to_string(&path)?;

                    // Store template in memory
                    templates.insert(filename.to_string(), content);

                    println!("Loaded template: {}", filename);
                }
            }
        }

        Ok(())
    }

    /// Get a static asset by filename
    pub async fn get_asset(&self, filename: &str) -> Option<Asset> {
        let assets = self.static_assets.read().await;

        // Check if we have a direct match
        if let Some(asset) = assets.get(filename) {
            return Some(asset.clone());
        }

        // If requesting a minified version, check for the original
        if filename.ends_with(".min.js") || filename.ends_with(".min.css") {
            let original_filename = filename
                .replace(".min.js", ".js")
                .replace(".min.css", ".css");

            return assets.get(&original_filename).cloned();
        }

        None
    }

    /// Get a template by filename
    pub async fn get_template(&self, filename: &str) -> Option<String> {
        let templates = self.templates.read().await;
        templates.get(filename).cloned()
    }

    /// Clone the AssetManager
    pub fn clone(&self) -> Self {
        Self {
            static_assets: Arc::clone(&self.static_assets),
            templates: Arc::clone(&self.templates),
        }
    }
}

impl Clone for AssetManager {
    fn clone(&self) -> Self {
        Self {
            static_assets: Arc::clone(&self.static_assets),
            templates: Arc::clone(&self.templates),
        }
    }
}
