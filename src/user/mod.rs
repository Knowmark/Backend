use crate::resp::problem::Problem;
use bson::spec::BinarySubtype;
use bson::{doc, Binary, Bson};
use crypto::bcrypt::bcrypt;
use rocket::http::{ContentType, Status};
use rocket::response::Responder;
use rocket::{response, Request, Response};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::convert::{TryFrom, TryInto};
use std::io::Cursor;
use uuid::Uuid;

pub mod profile;

use crate::role::Role;

pub static USER_COLLECTION_NAME: &str = "user";

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PasswordHash([u8; 24]);

impl PasswordHash {
    pub fn new(password: impl AsRef<str>) -> PasswordHash {
        let mut pw_hash: [u8; 24] = [0; 24];

        let mut sha = Sha256::new();
        sha2::Digest::update(&mut sha, password.as_ref().as_bytes());

        bcrypt(
            15,
            &crate::CRYPTO.salt,
            sha.finalize().as_slice(),
            &mut pw_hash,
        );

        PasswordHash(pw_hash)
    }
}

impl From<PasswordHash> for Bson {
    fn from(pw_hash: PasswordHash) -> Self {
        Bson::Binary(Binary {
            subtype: BinarySubtype::Generic,
            bytes: pw_hash.0.to_vec(),
        })
    }
}
impl TryFrom<Bson> for PasswordHash {
    type Error = Problem;

    fn try_from(bson: Bson) -> Result<Self, Self::Error> {
        match bson {
            Bson::Binary(bin) => {
                if let Ok(array) = bin.bytes.try_into() {
                    Ok(PasswordHash(array))
                } else {
                    Err(password_lost_err())
                }
            }
            _ => Err(password_lost_err()),
        }
    }
}

// TODO: Give a password reset form instead.
fn password_lost_err() -> Problem {
    Problem::new_untyped(Status::InternalServerError, "Unable to check password.")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id", with = "bson::serde_helpers::uuid_1_as_binary")]
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub pw_hash: PasswordHash,
    pub user_role: Role,
}

impl User {
    pub fn new(email: impl ToString, username: impl ToString, password: impl ToString) -> User {
        let pw_hash = PasswordHash::new(password.to_string());

        let id = Uuid::new_v5(&Uuid::NAMESPACE_OID, username.to_string().as_bytes());
        tracing::info!("Creating a new user with UUID: {}", id.to_string());

        User {
            id,
            email: email.to_string(),
            username: username.to_string(),
            pw_hash,
            user_role: Role::Normal,
        }
    }

    pub fn response_json(&self) -> String {
        json!({
            "id": self.id.clone(),
            "username": self.username.clone(),
            "user_role": self.user_role,
        })
        .to_string()
    }
}

impl<'r> Responder<'r, 'static> for User {
    fn respond_to(self, _: &Request) -> response::Result<'static> {
        let body: String = self.response_json();

        Response::build()
            .header(ContentType::JSON)
            .sized_body(body.len(), Cursor::new(body))
            .ok()
    }
}
