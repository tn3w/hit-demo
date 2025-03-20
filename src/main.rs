use actix_web::{App, HttpResponse, HttpServer, Responder, get, middleware, web, HttpRequest};
use serde::Serialize;
mod asset_processor;
mod config;
mod version_checker;

use asset_processor::{AssetManager, AssetType};
use config::{Config, load_config};
use version_checker::{VersionChecker, is_valid_version};

#[derive(Serialize)]
struct VersionResponse {
    version: String,
    sri_hash: String,
    url: String,
}

// Handler for serving the index template
#[get("/")]
pub async fn serve_index(
    version_checker: web::Data<VersionChecker>,
    asset_manager: web::Data<AssetManager>,
) -> impl Responder {
    if let Some(content) = asset_manager.get_template("index.html").await {
        let version = version_checker.get_current_version_info().await.version;
        let sri_hash = version_checker.get_current_version_info().await.sri_hash;
        let content = content
            .replace("VERSION", &version)
            .replace("SRI_HASH", &sri_hash);
        HttpResponse::Ok()
            .content_type("text/html")
            .append_header(("Cache-Control", "public, max-age=60"))
            .body(content)
    } else {
        HttpResponse::NotFound().body("Template not found")
    }
}

// Handler for serving 404 page
pub async fn not_found_handler(
    version_checker: web::Data<VersionChecker>,
    asset_manager: web::Data<AssetManager>,
) -> impl Responder {
    if let Some(content) = asset_manager.get_template("404.html").await {
        let version = version_checker.get_current_version_info().await.version;
        let sri_hash = version_checker.get_current_version_info().await.sri_hash;
        let content = content
            .replace("VERSION", &version)
            .replace("SRI_HASH", &sri_hash);
        HttpResponse::NotFound()
            .content_type("text/html")
            .append_header(("Cache-Control", "public, max-age=60"))
            .body(content)
    } else {
        HttpResponse::NotFound().body("Page not found")
    }
}

// Handler for serving static assets without version
#[get("/static/{filename:.*}")]
pub async fn serve_static(
    path: web::Path<String>,
    asset_manager: web::Data<AssetManager>,
) -> impl Responder {
    let filename = path.into_inner();

    // Check if this is a request for a minified file
    if filename.ends_with(".min.js") || filename.ends_with(".min.css") {
        // Extract the original filename
        let original_filename = filename
            .replace(".min.js", ".js")
            .replace(".min.css", ".css");

        if let Some(asset) = asset_manager.get_asset(&original_filename).await {
            // Set appropriate content type
            let content_type = match asset.asset_type {
                AssetType::JavaScript => "application/javascript",
                AssetType::CSS => "text/css",
            };

            return HttpResponse::Ok()
                .content_type(content_type)
                .append_header(("Cache-Control", "public, max-age=60"))
                .body(asset.content);
        }
    }

    HttpResponse::NotFound().body("Asset not found")
}

// Handler for serving versioned static assets
#[get("/static/{version}/{filename:.*}")]
pub async fn serve_versioned_static(
    path: web::Path<(String, String)>,
    asset_manager: web::Data<AssetManager>,
) -> impl Responder {
    let (version, filename) = path.into_inner();

    if !is_valid_version(&version) {
        return HttpResponse::NotFound().body("Invalid version");
    }

    // Serve the asset as in the non-versioned handler
    let original_filename = filename
        .replace(".min.js", ".js")
        .replace(".min.css", ".css");

    println!("Serving versioned static asset: {}", original_filename);

    if let Some(asset) = asset_manager.get_asset(&original_filename).await {
        // Set appropriate content type
        let content_type = match asset.asset_type {
            AssetType::JavaScript => "application/javascript",
            AssetType::CSS => "text/css",
        };

        let content = asset.content.replace("VERSION", &version);

        return HttpResponse::Ok()
            .content_type(content_type)
            .append_header(("Cache-Control", "public, max-age=3600"))
            .body(content);
    }

    HttpResponse::NotFound().body("Asset not found")
}

#[get("/api/latest")]
async fn get_latest_version(data: web::Data<VersionChecker>) -> impl Responder {
    let version_info = data.get_current_version_info().await;
    HttpResponse::Ok()
        .append_header(("Cache-Control", "public, max-age=60"))
        .json(VersionResponse {
            version: version_info.version.clone(),
            sri_hash: version_info.sri_hash,
            url: format!(
                "https://cdn.jsdelivr.net/npm/highlight-it@{}/dist/highlight-it.min.css",
                version_info.version
            ),
        })
}

// Handler for serving sitemap.xml
#[get("/sitemap.xml")]
pub async fn serve_sitemap(req: HttpRequest) -> impl Responder {
    // Get host info from request
    let connection_info = req.connection_info();
    let base_url = format!("{}://{}", connection_info.scheme(), connection_info.host());
    
    // Use standard library for date formatting in ISO 8601 format
    let current_datetime = {
        let now = std::time::SystemTime::now();
        let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        
        // Convert seconds to date components using basic division
        let total_seconds = duration.as_secs();
        let days_since_epoch = total_seconds / 86400; // seconds per day
        let seconds_in_day = total_seconds % 86400;
        
        // Calculate hours, minutes, seconds
        let hours = seconds_in_day / 3600;
        let minutes = (seconds_in_day % 3600) / 60;
        let seconds = seconds_in_day % 60;
        
        // Simple date calculation with leap years
        let mut year = 1970;
        let mut days_remaining = days_since_epoch;
        
        // Count years
        loop {
            let days_in_year = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                366 // leap year
            } else {
                365
            };
            
            if days_remaining >= days_in_year {
                days_remaining -= days_in_year;
                year += 1;
            } else {
                break;
            }
        }
        
        // Month lengths (accounting for leap years)
        let is_leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let month_days = [31, if is_leap_year {29} else {28}, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        
        // Count months
        let mut month = 0;
        while month < 12 && days_remaining >= month_days[month] {
            days_remaining -= month_days[month];
            month += 1;
        }
        
        // Day is remaining days + 1
        let day = days_remaining + 1;
        
        // Format as ISO 8601 with UTC timezone indicator
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+00:00", 
            year, month + 1, day, hours, minutes, seconds
        )
    };
    
    let sitemap = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://www.sitemaps.org/schemas/sitemap/0.9 http://www.sitemaps.org/schemas/sitemap/0.9/sitemap.xsd">
  <url>
    <loc>{}/</loc>
    <lastmod>{}</lastmod>
    <changefreq>daily</changefreq>
    <priority>1.0</priority>
  </url>
</urlset>"#, base_url, current_datetime);

    HttpResponse::Ok()
        .content_type("application/xml")
        .append_header(("Cache-Control", "public, max-age=86400"))
        .body(sitemap)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load configuration
    let config = load_config().unwrap_or_else(|e| {
        eprintln!("Error loading configuration: {}", e);
        Config::default()
    });

    println!(
        "Starting server on http://{}:{} with {} workers",
        config.host, config.port, config.workers
    );
    println!("HTTP timeout: {} seconds", config.http_timeout);
    println!(
        "Version check interval: {} seconds",
        config.version_check_interval
    );

    // Initialize version checker
    let checker = VersionChecker::new(
        "highlight-it",
        config.http_timeout,
        config.version_check_interval,
    );
    checker.start_checking().await;

    // Initialize asset manager
    let asset_manager = match AssetManager::new().await {
        Ok(manager) => manager,
        Err(e) => {
            eprintln!("Failed to initialize asset manager: {}", e);
            return Err(e);
        }
    };

    // Create a clone of config for the App
    let app_config = web::Data::new(config.clone());

    // Start the server
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .wrap(
                middleware::DefaultHeaders::new()
                    .add(("Strict-Transport-Security", "max-age=31536000; includeSubDomains"))
                    .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
                    .add(("X-Content-Type-Options", "nosniff"))
                    .add(("X-Frame-Options", "DENY"))
                    .add(("Content-Security-Policy", "default-src 'self'; script-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net; style-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net; connect-src 'self'; font-src 'self' data:; img-src 'self' data:; frame-ancestors 'none'"))
                    .add(("Cross-Origin-Embedder-Policy", "require-corp"))
                    .add(("Cross-Origin-Opener-Policy", "same-origin"))
                    .add(("Cross-Origin-Resource-Policy", "same-origin"))
                    .add(("Permissions-Policy", "camera=(), microphone=(), geolocation=()"))
            )
            .app_data(web::Data::new(checker.clone()))
            .app_data(web::Data::new(asset_manager.clone()))
            .app_data(app_config.clone())
            .service(get_latest_version)
            .service(serve_index)
            .service(serve_sitemap)
            .service(serve_versioned_static)
            .service(serve_static)
            .default_service(web::route().to(not_found_handler))
    })
    .bind(config.server_addr())?
    .workers(config.workers)
    .run()
    .await
}
