use crate::error::ConfigurationError;
use crate::util;
use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

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
    #[serde(skip)]
    file_path: PathBuf,

    #[serde(default = "default_mongodb_uri")]
    pub mongodb_uri: String,
    #[serde(default = "default_mongodb_db")]
    pub mongodb_db: String,

    #[serde(default = "default_public_content")]
    pub public_content: PathBuf,

    #[serde(default = "default_admin_usernames")]
    pub admin_usernames: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            file_path: config_dir().join("settings.yml"),
            mongodb_uri: default_mongodb_uri(),
            mongodb_db: default_mongodb_db(),
            public_content: default_public_content(),
            admin_usernames: default_admin_usernames(),
        }
    }
}

#[inline]
fn config_dir() -> PathBuf {
    PathBuf::from(env::var("CONFIG_DIR").unwrap_or("./config".to_string()))
}

impl Config {
    pub fn load() -> Result<Config, ConfigurationError> {
        let config_file = util::find_first_subpath(
            config_dir(),
            &["settings.yml", "settings.yaml"],
            Path::exists,
        )
        .ok_or_else(|| ConfigurationError::NotFound(config_dir()))?;

        let file = File::open(config_file)?;
        let config = serde_yaml::from_reader(BufReader::new(file))?;

        Ok(config)
    }

    pub fn save(&self) -> Result<(), ConfigurationError> {
        let file = File::create(&self.file_path)?;
        let mut out = BufWriter::new(file);
        serde_yaml::to_writer(&mut out, self)?;
        out.flush()?;
        Ok(())
    }
}
