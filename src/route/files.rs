use std::path::PathBuf;

use rocket::State;

use crate::config::Config;
use rocket::fs::NamedFile;

pub async fn app_index_file(c: &State<Config>) -> NamedFile {
    NamedFile::open(c.public_content.as_path().join("index.html"))
        .await
        .expect(
            format!(
                "'{}' does not exist!",
                c.public_content.as_path().join("index.html").display()
            )
            .as_str(),
        )
}

#[get("/")]
pub async fn app(c: &State<Config>) -> NamedFile {
    app_index_file(c).await
}

#[get("/<path..>", rank = 10)]
pub async fn app_path(path: PathBuf, c: &State<Config>) -> NamedFile {
    NamedFile::open(c.public_content.as_path().join(path.as_path()))
        .await
        .ok()
        .unwrap_or(app_index_file(c).await)
}
