mod models;
mod routes;
mod templates;

use anyhow::Result;
use axum::{
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{Html, Response},
    routing::{get, post},
    Router,
};
use regex::Regex;
use serde::Serialize;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::{fs::File, net::SocketAddr, path::Path, time::Instant};
use templates::CachedEnvironment;
use tokio::signal::unix::SignalKind;
use tower_http::services::ServeDir;
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pool: SqlitePool,
    environment: &'static CachedEnvironment,
}

impl AppState {
    pub fn render<S: Serialize>(&self, template_path: &str, context: S) -> Html<String> {
        self.environment.render(template_path, context)
    }
}

async fn logging_layer<B>(req: Request<B>, next: Next<B>) -> Result<Response, StatusCode> {
    let start = Instant::now();
    let method = req.method().clone();
    let url = req.uri().clone();
    let resp = next.run(req).await;
    info!(
        "[{}] {} {} [{:?}]",
        resp.status(),
        method,
        url,
        start.elapsed()
    );
    Ok(resp)
}

#[cfg(debug_assertions)]
fn cache_templates() -> bool {
    false
}
#[cfg(not(debug_assertions))]
fn cache_templates() -> bool {
    true
}

fn create_database_if_not_exists(db_url: &str) {
    let regex = Regex::new("sqlite://(.*)").unwrap();
    if let Some(m) = regex.captures(db_url) {
        let path = Path::new(m.get(1).unwrap().as_str());
        if !path.exists() {
            info!("Creating sqlite file {:?}", path);
            File::create(path).expect("Failed to create file");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let db_url = std::env::var("DATABASE_URL").expect("Failed to find database url");
    create_database_if_not_exists(&db_url);
    let pool = SqlitePoolOptions::new().connect(&db_url).await.unwrap();

    info!("Running migrations");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    info!("Migrations finished successfully!");

    let environment = Box::leak(Box::new(templates::CachedEnvironment::new(
        cache_templates(),
    )));

    let app = Router::new()
        .route("/", get(routes::get_index))
        .route("/new", get(routes::get_add))
        .route("/edit", post(routes::post_edit))
        .nest_service("/static", ServeDir::new("ui/static"))
        .layer(middleware::from_fn(logging_layer))
        .with_state(AppState { pool, environment });

    let address_port: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    let s = axum::Server::bind(&address_port)
        .serve(app.into_make_service())
        .with_graceful_shutdown(async {
            tokio::signal::unix::signal(SignalKind::terminate())
                .unwrap()
                .recv()
                .await;
            info!("Recived SIGTERM. Shutting down...");
        });
    info!("Server is running on {}", address_port);
    s.await.unwrap();
    Ok(())
}
