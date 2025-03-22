use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssetType {
    JavaScript,
    CSS,
}

#[derive(Debug, Clone)]
pub struct Asset {
    pub content: String,
    pub asset_type: AssetType,
}

pub struct AssetManager {
    static_assets: Arc<RwLock<HashMap<String, Asset>>>,
    templates: Arc<RwLock<HashMap<String, String>>>,
}

impl AssetManager {
    pub async fn new() -> Result<Self, std::io::Error> {
        let manager = Self {
            static_assets: Arc::new(RwLock::new(HashMap::new())),
            templates: Arc::new(RwLock::new(HashMap::new())),
        };

        manager.load_static_files().await?;
        manager.load_template_files().await?;

        Ok(manager)
    }

    async fn load_static_files(&self) -> Result<(), std::io::Error> {
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

                if filename.ends_with(".min.js") || filename.ends_with(".min.css") {
                    let content = fs::read_to_string(&path)?;

                    let asset_type = if filename.ends_with(".min.js") {
                        AssetType::JavaScript
                    } else {
                        AssetType::CSS
                    };

                    assets.insert(
                        filename.to_string(),
                        Asset {
                            content,
                            asset_type,
                        },
                    );
                }
            }
        }

        Ok(())
    }

    async fn load_template_files(&self) -> Result<(), std::io::Error> {
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

                if filename.ends_with(".html") {
                    let content = fs::read_to_string(&path)?;

                    templates.insert(filename.to_string(), content);
                }
            }
        }

        Ok(())
    }

    pub async fn get_asset(&self, filename: &str) -> Option<Asset> {
        let assets = self.static_assets.read().await;

        if let Some(asset) = assets.get(filename) {
            return Some(asset.clone());
        }

        None
    }

    pub async fn get_template(&self, filename: &str) -> Option<String> {
        let templates = self.templates.read().await;
        templates.get(filename).cloned()
    }

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
