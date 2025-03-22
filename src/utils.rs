use crate::asset_manager::AssetManager;
use crate::version_checker::VersionChecker;
use actix_web::{HttpRequest, HttpResponse, Responder, web};

pub fn get_hit_demo_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

pub fn get_cdn_url(version: &str) -> String {
    format!(
        "https://cdn.jsdelivr.net/npm/highlight-it@{}/dist/highlight-it-min.js",
        version
    )
}

pub fn get_current_datetime() -> String {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();

    let total_seconds = duration.as_secs();
    let days_since_epoch = total_seconds / 86400;
    let seconds_in_day = total_seconds % 86400;

    let hours = seconds_in_day / 3600;
    let minutes = (seconds_in_day % 3600) / 60;
    let seconds = seconds_in_day % 60;

    let mut year = 1970;
    let mut days_remaining = days_since_epoch;

    loop {
        let days_in_year = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
            366
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

    let mut month = 0;
    while month < 12 && days_remaining >= month_days[month] {
        days_remaining -= month_days[month];
        month += 1;
    }

    let day = days_remaining + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+00:00",
        year,
        month + 1,
        day,
        hours,
        minutes,
        seconds
    )
}

pub async fn create_not_found_response(
    reason: &str,
    version_checker: web::Data<VersionChecker>,
    asset_manager: web::Data<AssetManager>,
) -> HttpResponse {
    if let Some(content) = asset_manager.get_template("404.html").await {
        let version_info = version_checker.get_latest_version_info().await;
        let version = version_info.version;
        let sri_hash = version_info.sri_hash;

        let content = content
            .replace("DEMO_VERSION", &get_hit_demo_version())
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

pub async fn not_found_handler(
    req: HttpRequest,
    version_checker: web::Data<VersionChecker>,
    asset_manager: web::Data<AssetManager>,
) -> impl Responder {
    let path = req.path();
    create_not_found_response(
        &format!("Path not found: {}", path),
        version_checker,
        asset_manager,
    )
    .await
}
