use actix_web::{get, middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use serde::Serialize;
mod asset_manager;
mod config;
mod version_checker;

use asset_manager::{AssetManager, AssetType};
use config::{Config, load_config};
use version_checker::{VersionChecker, is_valid_version};

#[derive(Serialize)]
struct VersionResponse {
    version: String,
    sri_hash: String,
    url: String,
}

#[derive(Serialize)]
struct VersionsResponse {
    versions: Vec<VersionResponse>,
    latest: VersionResponse,
}

// Handler for serving the index template
#[get("/")]
pub async fn serve_index(
    version_checker: web::Data<VersionChecker>,
    asset_manager: web::Data<AssetManager>,
) -> impl Responder {
    if let Some(content) = asset_manager.get_template("index.html").await {
        let version_info = version_checker.get_current_version_info().await;
        let version = version_info.version;
        let sri_hash = version_info.sri_hash;

        // Get all versions for displaying in the UI
        let all_versions = version_checker.get_all_versions().await;

        // Create a versions dropdown HTML
        let mut versions_html = String::from(
            "<select id=\"version-selector\" onchange=\"window.location.href='/' + this.value;\">\n",
        );
        versions_html.push_str("  <option value=\"\" selected>Current Version</option>\n");

        for v in all_versions {
            versions_html.push_str(&format!(
                "  <option value=\"{}\">{}</option>\n",
                v.version, v.version
            ));
        }

        versions_html.push_str("</select>");

        // Insert versions_html where VERSION_SELECTOR appears in the template
        let content = content
            .replace("VERSION_SELECTOR", &versions_html)
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

// Helper function to create consistent 404 responses
async fn create_not_found_response(
    reason: &str,
    version_checker: web::Data<VersionChecker>,
    asset_manager: web::Data<AssetManager>,
) -> HttpResponse {
    // Log the reason for the 404
    println!("404 Not Found: {}", reason);
    
    // Use the same logic as the not_found_handler
    if let Some(content) = asset_manager.get_template("404.html").await {
        let version_info = version_checker.get_latest_version_info().await;
        let version = version_info.version;
        let sri_hash = version_info.sri_hash;

        // Get all versions for displaying in the UI
        let all_versions = version_checker.get_all_versions().await;

        // Create a versions dropdown HTML
        let mut versions_html = String::from(
            "<select id=\"version-selector\" onchange=\"window.location.href='/' + this.value;\">\n",
        );
        versions_html.push_str("  <option value=\"\" selected>Current Version</option>\n");

        for v in all_versions {
            versions_html.push_str(&format!(
                "  <option value=\"{}\">{}</option>\n",
                v.version, v.version
            ));
        }

        versions_html.push_str("</select>");

        let content = content
            .replace("VERSION_SELECTOR", &versions_html)
            .replace("VERSION", &version)
            .replace("SRI_HASH", &sri_hash);

        HttpResponse::NotFound()
            .content_type("text/html")
            .append_header(("Cache-Control", "public, max-age=60"))
            .body(content)
    } else {
        HttpResponse::NotFound().body(format!("Page not found: {}", reason))
    }
}

// Handler for serving 404 page
pub async fn not_found_handler(
    req: HttpRequest,
    version_checker: web::Data<VersionChecker>,
    asset_manager: web::Data<AssetManager>,
) -> impl Responder {
    // Extract the path to use as reason
    let path = req.path();
    create_not_found_response(&format!("Path not found: {}", path), version_checker, asset_manager).await
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

// Handler for specific version route
#[get("/{version}")]
pub async fn serve_specific_version(
    path: web::Path<String>,
    data: web::Data<VersionChecker>,
    asset_manager: web::Data<AssetManager>,
) -> impl Responder {
    let version = path.into_inner();

    // Validate version format
    if !is_valid_version(&version) {
        return create_not_found_response("Invalid version", data, asset_manager).await;
    }

    // Get all versions to check if requested version exists
    let all_versions = data.get_all_versions().await;

    // Find the requested version
    if let Some(version_info) = all_versions.iter().find(|v| v.version == version) {
        // Version found, serve the template with this version
        if let Some(content) = asset_manager.get_template("index.html").await {
            // Create a versions dropdown HTML
            let mut versions_html = String::from(
                "<select id=\"version-selector\" onchange=\"window.location.href='/' + this.value;\">\n",
            );
            versions_html.push_str("  <option value=\"\" selected>Current Version</option>\n");

            for v in &all_versions {
                let selected = if v.version == version_info.version {
                    " selected"
                } else {
                    ""
                };
                versions_html.push_str(&format!(
                    "  <option value=\"{}\"{}>{}</option>\n",
                    v.version, selected, v.version
                ));
            }

            versions_html.push_str("</select>");

            let content = content
                .replace("VERSION_SELECTOR", &versions_html)
                .replace("VERSION", &version_info.version)
                .replace("SRI_HASH", &version_info.sri_hash);

            return HttpResponse::Ok()
                .content_type("text/html")
                .append_header(("Cache-Control", "public, max-age=3600"))
                .body(content);
        } else {
            return HttpResponse::InternalServerError().body("Template not found");
        }
    } else {
        // Version not found
        return create_not_found_response("Version not found", data, asset_manager).await;
    }
}

// Handler for serving versioned static assets
#[get("/static/{version}/{filename:.*}")]
pub async fn serve_versioned_static(
    path: web::Path<(String, String)>,
    asset_manager: web::Data<AssetManager>,
    version_checker: web::Data<VersionChecker>,
) -> impl Responder {
    let (version, filename) = path.into_inner();

    // Validate version format
    if !is_valid_version(&version) {
        return create_not_found_response("Invalid version", version_checker, asset_manager).await;
    }

    // Check if version exists
    let all_versions = version_checker.get_all_versions().await;
    let version_exists = all_versions.iter().any(|v| v.version == version);

    if !version_exists {
        return create_not_found_response("Version not found", version_checker, asset_manager).await;
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

    create_not_found_response("Asset not found", version_checker, asset_manager).await
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

#[get("/api/versions")]
async fn get_all_versions(data: web::Data<VersionChecker>) -> impl Responder {
    let all_versions = data.get_all_versions().await;
    let latest_version = data.get_latest_version_info().await;

    let versions_response: Vec<VersionResponse> = all_versions
        .into_iter()
        .map(|v| VersionResponse {
            version: v.version.clone(),
            sri_hash: v.sri_hash,
            url: format!(
                "https://cdn.jsdelivr.net/npm/highlight-it@{}/dist/highlight-it-min.js",
                v.version
            ),
        })
        .collect();

    HttpResponse::Ok()
        .append_header(("Cache-Control", "public, max-age=60"))
        .json(VersionsResponse {
            versions: versions_response,
            latest: VersionResponse {
                version: latest_version.version.clone(),
                sri_hash: latest_version.sri_hash,
                url: format!(
                    "https://cdn.jsdelivr.net/npm/highlight-it@{}/dist/highlight-it-min.js",
                    latest_version.version
                ),
            },
        })
}

// Handler for serving sitemap.xml
#[get("/sitemap.xml")]
pub async fn serve_sitemap(
    req: HttpRequest,
    version_checker: web::Data<VersionChecker>,
) -> impl Responder {
    // Get host info from request
    let connection_info = req.connection_info();
    let base_url = format!("{}://{}", connection_info.scheme(), connection_info.host());

    // Use standard library for date formatting in ISO 8601 format
    let current_datetime = {
        let now = std::time::SystemTime::now();
        let duration = now
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();

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
        let month_days = [
            31,
            if is_leap_year { 29 } else { 28 },
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ];

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
            year,
            month + 1,
            day,
            hours,
            minutes,
            seconds
        )
    };

    // Get all versions to add to sitemap
    let all_versions = version_checker.get_all_versions().await;

    // Start building sitemap
    let mut sitemap = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://www.sitemaps.org/schemas/sitemap/0.9 http://www.sitemaps.org/schemas/sitemap/0.9/sitemap.xsd">
  <url>
    <loc>{}/</loc>
    <lastmod>{}</lastmod>
    <changefreq>daily</changefreq>
    <priority>1.0</priority>
  </url>
"#,
    );

    // Format the root URL entry
    sitemap = sitemap.replace("{}", &base_url);
    sitemap = sitemap.replace("{}", &current_datetime);

    // Add entries for each version
    for version_info in all_versions {
        let version_entry = format!(
            r#"  <url>
    <loc>{}/{}</loc>
    <lastmod>{}</lastmod>
    <changefreq>monthly</changefreq>
    <priority>0.8</priority>
  </url>
"#,
            base_url, version_info.version, current_datetime
        );

        sitemap.push_str(&version_entry);
    }

    // Close sitemap
    sitemap.push_str("</urlset>");

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

    println!(
        "Starting server on http://{}:{} with {} workers",
        config.host, config.port, config.workers
    );

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
            .service(get_all_versions)
            .service(serve_index)
            .service(serve_sitemap)
            .service(serve_specific_version)
            .service(serve_versioned_static)
            .service(serve_static)
            .default_service(web::route().to(not_found_handler))
    })
    .bind(config.server_addr())?
    .workers(config.workers)
    .run()
    .await
}
