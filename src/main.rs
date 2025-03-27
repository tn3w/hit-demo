use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, get, middleware, web};
use serde::Serialize;
mod asset_manager;
mod config;
mod utils;
mod version_checker;

use asset_manager::{AssetManager, AssetType};
use config::{Config, load_config};
use std::process::Command;
use utils::{
    create_not_found_response, get_cdn_url, get_current_datetime, get_hit_demo_version,
    not_found_handler,
};
use version_checker::{VersionChecker, get_versions_selector, is_valid_version};

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

#[get("/")]
async fn serve_index(
    version_checker: web::Data<VersionChecker>,
    asset_manager: web::Data<AssetManager>,
) -> impl Responder {
    if let Some(content) = asset_manager.get_template("index.min.html").await {
        let version_info = version_checker.get_current_version_info().await;
        let version = version_info.version;
        let sri_hash = version_info.sri_hash;

        let all_versions = version_checker.get_all_versions().await;

        let versions_html = get_versions_selector(all_versions, version.clone(), None);

        let content = content
            .replace("DEMO_VERSION", &get_hit_demo_version())
            .replace("VERSION_SELECTOR", &versions_html)
            .replace("VERSION", &version)
            .replace("SRI_HASH", &sri_hash);

        HttpResponse::Ok()
            .content_type("text/html")
            .append_header(("Cache-Control", "public, max-age=60"))
            .body(content)
    } else {
        create_not_found_response(
            "Template not found",
            version_checker,
            asset_manager,
            Some("/"),
        )
        .await
    }
}

#[get("/{version}")]
async fn serve_versioned_index(
    path: web::Path<String>,
    data: web::Data<VersionChecker>,
    asset_manager: web::Data<AssetManager>,
) -> impl Responder {
    let version = path.into_inner();

    if !is_valid_version(&version) {
        return create_not_found_response(
            "Invalid version",
            data,
            asset_manager,
            Some(&format!("/{}", version)),
        )
        .await;
    }

    let all_versions = data.get_all_versions().await;

    if let Some(version_info) = all_versions.iter().find(|v| v.version == version) {
        if let Some(content) = asset_manager.get_template("index.min.html").await {
            let latest_version_info = data.get_current_version_info().await;
            let latest_version = latest_version_info.version.clone();

            let versions_html =
                get_versions_selector(all_versions.clone(), latest_version, Some(version.clone()));

            let content = content
                .replace("DEMO_VERSION", &get_hit_demo_version())
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
        return create_not_found_response(
            "Version not found",
            data,
            asset_manager,
            Some(&format!("/{}", version)),
        )
        .await;
    }
}

#[get("/static/{filename:.*}")]
async fn serve_static(
    path: web::Path<String>,
    asset_manager: web::Data<AssetManager>,
    version_checker: web::Data<VersionChecker>,
) -> impl Responder {
    let filename = path.into_inner();

    if filename.ends_with(".min.js") || filename.ends_with(".min.css") {
        if let Some(asset) = asset_manager.get_asset(&filename).await {
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

    create_not_found_response(
        "Asset not found",
        version_checker,
        asset_manager,
        Some(&format!("/static/{}", filename)),
    )
    .await
}

#[get("/static/{version}/{filename:.*}")]
async fn serve_versioned_static(
    path: web::Path<(String, String)>,
    asset_manager: web::Data<AssetManager>,
    version_checker: web::Data<VersionChecker>,
) -> impl Responder {
    let (version, filename) = path.into_inner();

    if !is_valid_version(&version) {
        return create_not_found_response(
            "Invalid version",
            version_checker,
            asset_manager,
            Some(&format!("/static/{}/{}", version, filename)),
        )
        .await;
    }

    let all_versions = version_checker.get_all_versions().await;
    let version_exists = all_versions.iter().any(|v| v.version == version);

    if !version_exists {
        return create_not_found_response(
            "Version not found",
            version_checker,
            asset_manager,
            Some(&format!("/static/{}/{}", version, filename)),
        )
        .await;
    }

    if let Some(asset) = asset_manager.get_asset(&filename).await {
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

    create_not_found_response(
        "Asset not found",
        version_checker,
        asset_manager,
        Some(&format!("/static/{}/{}", version, filename)),
    )
    .await
}

#[get("/api/latest")]
async fn serve_latest_version_api(data: web::Data<VersionChecker>) -> impl Responder {
    let version_info = data.get_current_version_info().await;
    HttpResponse::Ok()
        .append_header(("Cache-Control", "public, max-age=60"))
        .json(VersionResponse {
            version: version_info.version.clone(),
            sri_hash: version_info.sri_hash,
            url: get_cdn_url(&version_info.version),
        })
}

#[get("/api/versions")]
async fn serve_all_versions_api(data: web::Data<VersionChecker>) -> impl Responder {
    let all_versions = data.get_all_versions().await;
    let latest_version = data.get_latest_version_info().await;

    let versions_response: Vec<VersionResponse> = all_versions
        .into_iter()
        .map(|v| VersionResponse {
            version: v.version.clone(),
            sri_hash: v.sri_hash,
            url: get_cdn_url(&v.version),
        })
        .collect();

    HttpResponse::Ok()
        .append_header(("Cache-Control", "public, max-age=60"))
        .json(VersionsResponse {
            versions: versions_response,
            latest: VersionResponse {
                version: latest_version.version.clone(),
                sri_hash: latest_version.sri_hash,
                url: get_cdn_url(&latest_version.version),
            },
        })
}

#[get("/sitemap.xml")]
async fn serve_sitemap(
    req: HttpRequest,
    version_checker: web::Data<VersionChecker>,
) -> impl Responder {
    let connection_info = req.connection_info();
    let base_url = format!("{}://{}", connection_info.scheme(), connection_info.host());

    let current_datetime = get_current_datetime();

    let all_versions = version_checker.get_all_versions().await;

    let mut sitemap = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://www.sitemaps.org/schemas/sitemap/0.9 http://www.sitemaps.org/schemas/sitemap/0.9/sitemap.xsd">
  <url>
    <loc>BASE_URL/</loc>
    <lastmod>CURRENT_DATETIME</lastmod>
    <changefreq>daily</changefreq>
    <priority>1.0</priority>
  </url>
"#,
    );

    sitemap = sitemap.replace("BASE_URL", &base_url);
    sitemap = sitemap.replace("CURRENT_DATETIME", &current_datetime);

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

    sitemap.push_str("</urlset>");

    HttpResponse::Ok()
        .content_type("application/xml")
        .append_header(("Cache-Control", "public, max-age=86400"))
        .body(sitemap)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Running build script to minify assets...");
    match Command::new("node").arg("build.js").status() {
        Ok(status) => {
            if !status.success() {
                eprintln!("Warning: Build script failed with exit code: {}", status);
            } else {
                println!("Build script completed successfully");
            }
        }
        Err(e) => {
            eprintln!("Warning: Failed to run build script: {}", e);
        }
    }

    let config = load_config().unwrap_or_else(|e| {
        eprintln!("Error loading configuration: {}", e);
        Config::default()
    });

    println!("HTTP timeout: {} seconds", config.http_timeout);
    println!(
        "Version check interval: {} seconds",
        config.version_check_interval
    );

    let checker = VersionChecker::new(
        "highlight-it",
        config.http_timeout,
        config.version_check_interval,
        config.cache_dir.as_deref(),
    );
    checker.start_checking().await;

    let asset_manager = match AssetManager::new().await {
        Ok(manager) => manager,
        Err(e) => {
            eprintln!("Failed to initialize asset manager: {}", e);
            return Err(e);
        }
    };

    let app_config = web::Data::new(config.clone());

    println!(
        "Starting server on http://{}:{} with {} workers",
        config.host, config.port, config.workers
    );

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
            .service(serve_index)
            .service(serve_sitemap)
            .service(serve_latest_version_api)
            .service(serve_all_versions_api)
            .service(serve_versioned_static)
            .service(serve_static)
            .service(serve_versioned_index)
            .default_service(web::route().to(not_found_handler))
    })
    .bind(config.server_addr())?
    .workers(config.workers)
    .run()
    .await
}
