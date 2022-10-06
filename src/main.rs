#![feature(proc_macro_hygiene, decl_macro)]

extern crate tracing_futures;

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate lazy_static;

use bson::doc;
use mongodb::Client;
use rocket::http::Method;
use rocket_cors::{AllowedHeaders, AllowedOrigins};
use std::process::exit;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::config::Config;
use crate::resp::crypto::Crypto;
use crate::route::mount_api;
use std::ops::Deref;
use crate::error::ConfigurationError;

mod client;
mod config;
mod data;
mod error;
mod resp;
mod role;
mod route;
mod user;
mod util;

lazy_static! {
    pub static ref CRYPTO: Crypto = Crypto::init();
}

#[rocket::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    tracing::info!("Reading .env file...");
    if dotenv::dotenv().is_err() {
        tracing::error!("Unable to load .env file.")
    }

    tracing::info!("Initializing configuration...");
    let c = match Config::load() {
        Ok(c) => c,
        Err(ConfigurationError::NotFound(_)) => {
            let c = Config::default();
            if c.save().is_err() {
                tracing::error!("Unable to save configuration...");
            }
            c
        },
        Err(other) => std::panic::panic_any(other)
    };

    let _ = CRYPTO.deref();

    tracing::info!("Connecting to MongoDB: {}", c.mongodb_uri);
    let client = Client::with_uri_str(c.mongodb_uri.as_str())
        .await
        .expect("Unable to init MongoDB client! Is URI valid?");

    tracing::info!("Using MongoDB database: {}", c.mongodb_db);
    let db = client.database(c.mongodb_db.as_str());

    if db.list_collections(None, None).await.is_err() {
        tracing::error!("Unable to connect to MongoDB.");
        exit(1)
    }

    tracing::info!("Starting HTTP server...");
    let mut r = rocket::build().manage(c).manage(db);

    tracing::info!("Setting up CORS...");
    let allowed_origins = AllowedOrigins::All;

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

    match r.launch().await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("Error launching HTTP server: {}", e);
        }
    };
}
