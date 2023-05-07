use std::env;
use std::path::PathBuf;

pub static CONFIG_FILE_NAME: &str = "Knowmark.toml";

fn default_mongodb_uri() -> String {
    env::var("MONGODB_URI").unwrap_or("mongodb://localhost:27017".to_string())
}

fn default_mongodb_db() -> String {
    env::var("MONGODB_DB_NAME").unwrap_or("knowmark".to_string())
}

fn default_public_content() -> PathBuf {
    PathBuf::from(env::var("PUBLIC_CONTENT_PATH").unwrap_or("./public".to_string()))
}

#[cfg(debug_assertions)]
fn default_admin_usernames() -> Vec<String> {
    vec![String::from("admin")]
}
#[cfg(not(debug_assertions))]
fn default_admin_usernames() -> Vec<String> {
    vec![]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_mongodb_uri")]
    pub mongodb_uri: String,
    #[serde(default = "default_mongodb_db")]
    pub mongodb_db: String,

    #[serde(default = "default_public_content")]
    pub public_content: PathBuf,

    #[serde(default = "default_admin_usernames")]
    pub admin_usernames: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            mongodb_uri: default_mongodb_uri(),
            mongodb_db: default_mongodb_db(),
            public_content: default_public_content(),
            admin_usernames: default_admin_usernames(),
        }
    }
}
