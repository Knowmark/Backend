use std::convert::TryInto;
use std::path::PathBuf;
use std::{env, fs};

const PASSWORD_SALT: &'static str = "password.salt";
const USER_AUTH_PUBLIC: &'static str = "user_auth.pem.pub";
const USER_AUTH_PRIVATE: &'static str = "user_auth.pem";

pub type Salt = [u8; 16];

#[derive(Debug, Clone)]
pub struct KeySet {
    pub public: Vec<u8>,
    pub private: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Security {
    pub salt: Salt,
    pub jwt_keys: KeySet,
}

#[inline]
fn security_dir() -> PathBuf {
    PathBuf::from(env::var("SECURITY_DIR").unwrap_or("./security".to_string()))
}

impl Security {
    pub fn load() -> Security {
        let dir = security_dir();

        if cfg!(feature = "generate-security") {
            fs::create_dir_all(dir.clone())
                .expect("unable to create directory for storing security information");
        }

        tracing::info!("Loading password salt...");
        let mut salt: Option<Salt> = fs::read(dir.join(PASSWORD_SALT))
            .map(|s| s.try_into().ok())
            .ok()
            .flatten();

        match salt {
            None => {
                tracing::info!("Salt not found in '{}'.", dir.join(PASSWORD_SALT).display());
                if cfg!(feature = "generate-security") {
                    tracing::info!("Generating a new password salt.");
                    salt = Some(rand::random());

                    fs::write(dir.join(PASSWORD_SALT), salt.unwrap())
                        .expect("unable to write salt");
                }
            }
            Some(_) => tracing::info!("Salt found and loaded."),
        }

        tracing::info!("Loading JWT signing keys...");
        let pub_key = fs::read(dir.join(USER_AUTH_PUBLIC)).ok();
        let priv_key = fs::read(dir.join(USER_AUTH_PRIVATE)).ok();

        let jwt_keys = match (pub_key, priv_key) {
            (Some(public), Some(private)) => {
                tracing::info!("Loaded JWT keys.");
                KeySet { public, private }
            }
            #[cfg(feature = "generate-security")]
            _ => {
                use rsa::pkcs1::{EncodeRsaPrivateKey, LineEnding};
                use rsa::pkcs8::EncodePublicKey;

                tracing::info!(
                    "Unable to load private and/or public user auth key(s). Generating a new pair."
                );

                tracing::info!("Generating a private RSA key. This will take a few minutes...");
                let mut rng = rand::thread_rng();
                let rsa_sk = rsa::RsaPrivateKey::new(&mut rng, 4096)
                    .expect("unable to generate a private RSA key");

                tracing::info!("Creating PS256 private key...");
                let private = rsa_sk
                    .to_pkcs1_pem(LineEnding::LF)
                    .expect("unable to generate PS256 private key")
                    .to_string()
                    .into_bytes();

                fs::write(dir.join(USER_AUTH_PRIVATE), private.as_slice())
                    .expect("unable to write user auth private key");

                tracing::info!("Creating PS256 public key...");
                let public = rsa_sk
                    .to_public_key()
                    .to_public_key_der()
                    .expect("unable to generate PS256 public key")
                    .to_pem("JWT public key", LineEnding::LF)
                    .expect("unable to crate a valid UTF8 pem key")
                    .into_bytes();

                fs::write(dir.join(USER_AUTH_PUBLIC), public.as_slice())
                    .expect("unable to write user auth public key");

                tracing::info!("Done generating JWT keys.");

                KeySet { public, private }
            }
            #[cfg(not(feature = "generate-security"))]
            _ => {
                panic!("Unable to load private and/or public user auth key(s).");
            }
        };

        Security {
            salt: salt.unwrap(),
            jwt_keys,
        }
    }
}
