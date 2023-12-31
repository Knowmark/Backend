#![allow(incomplete_features)]
#![feature(proc_macro_hygiene, decl_macro, async_fn_in_trait)]

extern crate tracing_futures;

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate serde;

use error::BackendError;
use mongodb::Client;
use rocket::http::Method;
use rocket::{Config, Rocket};
use rocket_cors::{AllowedHeaders, AllowedOrigins, Origins};
use std::process::exit;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::route::mount_api;
use crate::security::Security;
use crate::settings::{Settings, CONFIG_FILE_NAME};

pub mod data;
pub mod error;
pub mod middleware;
pub mod resp;
pub mod role;
pub mod route;
pub mod security;
pub mod settings;
pub mod util;

pub async fn create(log_level: Option<Level>) -> Result<Rocket<rocket::Build>, BackendError> {
    if let Some(l) = log_level {
        let subscriber = FmtSubscriber::builder().with_max_level(l).finish();

        if let Err(err) = tracing::subscriber::set_global_default(subscriber) {
            eprintln!("Unable to set global logger: {}", err);
        };
    }

    tracing::info!("Reading .env file...");
    if dotenv::dotenv().is_err() {
        tracing::warn!("Unable to load .env file.");
    }

    // override Rocket settings file name
    std::env::set_var("ROCKET_CONFIG", CONFIG_FILE_NAME);

    let settings = Config::figment().extract::<Settings>().unwrap_or_else(|_| {
        tracing::warn!("Unable to extract Settings from Config figment");
        Settings::default()
    });

    tracing::info!("Initializing security salts and keys...");
    let security = Security::load();

    tracing::info!("Connecting to MongoDB: {}", settings.mongodb_uri);
    let db_client = Client::with_uri_str(settings.mongodb_uri.as_str())
        .await
        .expect("Unable to init MongoDB client! Is URI valid?");

    tracing::info!("Using MongoDB database: {}", settings.mongodb_db);
    let db = db_client.database(settings.mongodb_db.as_str());

    if db.list_collections(None, None).await.is_err() {
        tracing::error!("Unable to connect to MongoDB.");
        exit(1)
    }

    tracing::info!("Initializing Rocket...");
    let mut r = rocket::build().manage(settings).manage(db).manage(security);

    tracing::info!("Setting up CORS...");
    let allowed_origins = rocket_cors::AllOrSome::some_exact(&[
        "http://localhost:5173/",
        "https://localhost:5173/",
        "http://localhost:8000/",
        "https://localhost:8000/",
    ]);

    // You can also deserialize this
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Get, Method::Put, Method::Post, Method::Delete]
            .into_iter()
            .map(From::from)
            .collect(),
        allowed_headers: AllowedHeaders::All,
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()
    .expect("Unable to configure CORS.");

    r = r.attach(cors);
    r = mount_api(r);

    Ok(r)
}
