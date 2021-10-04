use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

fn default_mongodb_uri() -> String {
    env::var("MONGODB_URI").unwrap_or("mongodb://localhost:27017".to_string())
}

fn default_mongodb_db() -> String {
    env::var("MONGODB_DB_NAME").unwrap_or("knowmark".to_string())
}

fn default_public_content() -> PathBuf {
    PathBuf::from(env::var("PUBLIC_CONTENT_PATH").unwrap_or("./public".to_string()))
}

fn default_admin_usernames() -> Vec<String> {
    vec![String::from("admin")]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_mongodb_uri")]
    pub mongodb_uri: String,
    #[serde(default = "default_mongodb_db")]
    pub mongodb_db: String,

    #[serde(default = "default_public_content")]
    pub public_content: PathBuf,

    #[serde(default = "default_admin_usernames")]
    pub admin_usernames: Vec<String>,
}

#[inline]
fn config_dir() -> PathBuf {
    PathBuf::from(env::var("CONFIG_DIR").unwrap_or("./config".to_string()))
}

impl Config {
    pub fn init() -> Config {
        let config_file = if config_dir().join("settings.yml").exists() {
            config_dir().join("settings.yml")
        } else {
            config_dir().join("settings.yaml")
        };

        let file = File::open(config_file);

        match file {
            Ok(f) => match serde_yaml::from_reader(BufReader::new(f)) {
                Ok(it) => it,
                Err(_) => None,
            },
            Err(_) => None,
        }
        .unwrap_or(Config {
            mongodb_uri: default_mongodb_uri(),
            mongodb_db: default_mongodb_db(),
            public_content: default_public_content(),
            admin_usernames: default_admin_usernames(),
        })
    }
}
