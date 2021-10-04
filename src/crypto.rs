use rsa::pkcs1::ToRsaPrivateKey;
use rsa::pkcs8::ToPublicKey;
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
pub struct Crypto {
    pub salt: Salt,
    pub user_auth_key: KeySet,
}

#[inline]
fn security_dir() -> PathBuf {
    PathBuf::from(env::var("SECURITY_DIR").unwrap_or("./security".to_string()))
}

impl Crypto {
    pub fn init() -> Crypto {
        let dir = security_dir();

        fs::create_dir_all(dir.clone())
            .expect("unable to create directory for storing security information");

        tracing::info!("Loading password salt...");
        let mut salt: Option<Salt> = fs::read(dir.join(PASSWORD_SALT))
            .map(|s| s.try_into().ok())
            .ok()
            .flatten();

        match salt {
            None => {
                tracing::info!(
                    "Salt not found in '{}'. Generating a new password salt.",
                    dir.join(PASSWORD_SALT).display()
                );
                salt = Some(rand::random());

                fs::write(dir.join(PASSWORD_SALT), salt.unwrap()).expect("unable to write salt");
            }
            Some(_) => tracing::info!("Salt found and loaded."),
        }

        tracing::info!("Loading JWT signing keys...");
        let mut public = fs::read(dir.join(USER_AUTH_PUBLIC)).unwrap_or(vec![]);

        let mut private = fs::read(dir.join(USER_AUTH_PRIVATE)).unwrap_or(vec![]);

        if public.len() == 0 || private.len() == 0 {
            tracing::info!("Private and/or public user auth key(s) empty. Generating a new pair.");

            tracing::info!("Generating a private RSA key. This will take a few minutes...");
            let mut rng = rand::thread_rng();
            let rsa_sk = rsa::RsaPrivateKey::new(&mut rng, 4096)
                .expect("unable to generate a private RSA key");

            tracing::info!("Creating PS256 private key...");
            private = rsa_sk
                .to_pkcs1_pem()
                .expect("unable to generate PS256 private key")
                .to_string()
                .bytes()
                .collect();

            fs::write(dir.join(USER_AUTH_PRIVATE), private.as_slice())
                .expect("unable to write user auth private key");

            tracing::info!("Creating PS256 public key...");
            public = rsa_sk
                .to_public_key()
                .to_public_key_der()
                .expect("unable to generate PS256 public key")
                .to_pem()
                .as_bytes()
                .to_vec();

            fs::write(dir.join(USER_AUTH_PUBLIC), public.as_slice())
                .expect("unable to write user auth public key");

            tracing::info!("Done generating JWT keys.");
        } else {
            tracing::info!("Loaded JWT keys.");
        };

        Crypto {
            salt: salt.unwrap(),
            user_auth_key: KeySet { public, private },
        }
    }
}
