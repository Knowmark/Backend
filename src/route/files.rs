use crate::config::Config;
use rocket::{fs::NamedFile, State};

use std::path::PathBuf;

pub async fn app_index_file(c: &State<Config>) -> Option<NamedFile> {
    NamedFile::open(c.public_content.as_path().join("index.html"))
        .await
        .ok()
}

/// Serves client root page
#[utoipa::path]
#[get("/")]
pub async fn app(c: &State<Config>) -> Option<NamedFile> {
    app_index_file(c).await
}

/// Serves client/public content
#[utoipa::path(
    params(
        ("path", description = "content path")
    )
)]
#[get("/<path..>", rank = 10)]
pub async fn app_path(path: PathBuf, c: &State<Config>) -> Option<NamedFile> {
    NamedFile::open(c.public_content.as_path().join(path.as_path()))
        .await
        .ok()
}
