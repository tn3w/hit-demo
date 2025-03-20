use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

// Constants
const STATIC_DIR: &str = "static";
const TEMPLATES_DIR: &str = "templates";
const MIN_JS_EXT: &str = ".min.js";
const MIN_CSS_EXT: &str = ".min.css";

// Asset types
#[derive(Debug, Clone)]
pub enum AssetType {
    JavaScript,
    CSS,
}

// Asset entry with file info and content
#[derive(Debug, Clone)]
pub struct Asset {
    content: String,
    asset_type: AssetType,
}

// Asset content with type information
#[derive(Debug, Clone)]
pub struct AssetContent {
    pub content: String,
    pub asset_type: AssetType,
}

#[derive(Debug, Clone)]
pub struct AssetManager {
    // Maps original filenames to their processed assets
    assets: Arc<RwLock<HashMap<String, Asset>>>,
    // Maps template names to their processed content
    templates: Arc<RwLock<HashMap<String, String>>>,
}

impl AssetManager {
    pub async fn new() -> io::Result<Self> {
        let manager = Self {
            assets: Arc::new(RwLock::new(HashMap::new())),
            templates: Arc::new(RwLock::new(HashMap::new())),
        };

        // Process all static assets on initialization
        manager.process_static_assets().await?;
        manager.process_templates().await?;

        Ok(manager)
    }

    // Process all static assets (JS and CSS files)
    async fn process_static_assets(&self) -> io::Result<()> {
        let static_dir = PathBuf::from(STATIC_DIR);
        if !static_dir.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Static directory not found: {}", static_dir.display()),
            ));
        }

        let mut assets = self.assets.write().await;

        // Process each file in the static directory
        for entry in fs::read_dir(&static_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let file_name = path
                    .file_name()
                    .and_then(|os_str| os_str.to_str())
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid filename"))?
                    .to_string();

                let extension = path
                    .extension()
                    .and_then(|os_str| os_str.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                // Skip already minified files or files containing hash codes
                if file_name.contains(".min.") || file_name.contains('-') {
                    continue;
                }

                // Process based on file type
                match extension.as_str() {
                    "js" => {
                        self.process_file(&path, &file_name, AssetType::JavaScript, &mut assets)?;
                    }
                    "css" => {
                        self.process_file(&path, &file_name, AssetType::CSS, &mut assets)?;
                    }
                    _ => continue,
                }
            }
        }

        Ok(())
    }

    // Process a single file (compute hash, minify if needed)
    fn process_file(
        &self,
        path: &Path,
        file_name: &str,
        asset_type: AssetType,
        assets: &mut HashMap<String, Asset>,
    ) -> io::Result<()> {
        // Read file content
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        // Calculate hash
        let hash = calculate_hash(&content);

        // Minify content based on asset type
        let minified_content = match asset_type {
            AssetType::JavaScript => minify_js(&content),
            AssetType::CSS => minify_css(&content),
        };

        // Create asset entry
        let content = minified_content.clone();
        let asset = Asset {
            content: minified_content,
            asset_type: asset_type,
        };

        // Store in assets map
        assets.insert(file_name.to_string(), asset);

        // Save minified content to disk
        self.save_minified_file(path, &hash, &content)?;

        Ok(())
    }

    // Save minified content to a hash-versioned file
    fn save_minified_file(
        &self,
        original_path: &Path,
        hash: &str,
        content: &str,
    ) -> io::Result<()> {
        let file_stem = original_path
            .file_stem()
            .and_then(|os_str| os_str.to_str())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid filename"))?;

        let extension = original_path
            .extension()
            .and_then(|os_str| os_str.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Create the minified filename with hash
        let min_extension = match extension.as_str() {
            "js" => MIN_JS_EXT,
            "css" => MIN_CSS_EXT,
            _ => return Ok(()), // Skip non-JS/CSS files
        };

        let hashed_filename = format!("{}-{}{}", file_stem, hash, min_extension);
        let output_path = original_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(&hashed_filename);

        // Write content to new file
        let mut file = File::create(&output_path)?;
        file.write_all(content.as_bytes())?;

        Ok(())
    }

    // Process HTML templates
    async fn process_templates(&self) -> io::Result<()> {
        let templates_dir = PathBuf::from(TEMPLATES_DIR);
        if !templates_dir.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Templates directory not found: {}", templates_dir.display()),
            ));
        }

        let mut templates = self.templates.write().await;

        // Process each template file
        for entry in fs::read_dir(&templates_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("html") {
                let file_name = path
                    .file_name()
                    .and_then(|os_str| os_str.to_str())
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid filename"))?
                    .to_string();

                // Read template content
                let mut file = File::open(&path)?;
                let mut content = String::new();
                file.read_to_string(&mut content)?;

                // Store processed template
                templates.insert(file_name, content);
            }
        }

        Ok(())
    }

    // Get an asset by filename
    pub async fn get_asset(&self, filename: &str) -> Option<AssetContent> {
        let assets = self.assets.read().await;
        let asset = assets.get(filename)?;
        Some(AssetContent {
            content: asset.content.clone(),
            asset_type: asset.asset_type.clone(),
        })
    }

    // Get a processed template by name
    pub async fn get_template(&self, template_name: &str) -> Option<String> {
        let templates = self.templates.read().await;
        templates.get(template_name).cloned()
    }
}

// Calculate SHA-256 hash for a string
fn calculate_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

// JavaScript minification (not implemented)
fn minify_js(js: &str) -> String {
    js.to_string()
}

// CSS minification
fn minify_css(css: &str) -> String {
    css.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with("/*"))
        .collect::<Vec<&str>>()
        .join("")
        .replace(": ", ":")
        .replace(" {", "{")
        .replace("{ ", "{")
        .replace(" }", "}")
        .replace(", ", ",")
        .replace("; ", ";")
        .replace("  ", " ")
}
