use crate::resp::problem::Problem;
use bson::spec::BinarySubtype;
use bson::{doc, Binary, Bson};
use crypto::bcrypt::bcrypt;
use rocket::http::{ContentType, Status};
use rocket::response::Responder;
use rocket::{response, Request, Response};
use sha2::{Digest, Sha256};
use std::convert::{TryFrom, TryInto};
use std::io::Cursor;
use utoipa::ToSchema;
use uuid::Uuid;

pub mod db;
pub mod profile;

use crate::role::Role;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PasswordHash([u8; 24]);

impl PasswordHash {
    pub fn new(password: impl AsRef<str>, salt: impl AsRef<[u8]>) -> PasswordHash {
        let mut pw_hash: [u8; 24] = [0; 24];

        let mut sha = Sha256::new();
        sha2::Digest::update(&mut sha, password.as_ref().as_bytes());

        bcrypt(15, salt.as_ref(), sha.finalize().as_slice(), &mut pw_hash);

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
    pub fn new(
        email: impl AsRef<str>,
        username: impl AsRef<str>,
        password: impl AsRef<str>,
        salt: impl AsRef<[u8]>,
    ) -> User {
        let pw_hash = PasswordHash::new(password, salt);

        let id = Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            [email.as_ref(), username.as_ref()].join("").as_bytes(),
        );
        tracing::info!("Creating a new user with UUID: {}", id.to_string());

        User {
            id,
            email: email.as_ref().to_string(),
            username: username.as_ref().to_string(),
            pw_hash,
            user_role: Role::Normal,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    /// User UUID
    pub id: Uuid,
    /// User email
    pub email: String,
    /// User username
    pub username: String,
    /// User role
    pub user_role: Role,
}

impl UserResponse {
    pub fn json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        UserResponse {
            id: user.id,
            email: user.email,
            username: user.username,
            user_role: user.user_role,
        }
    }
}

impl<'r> Responder<'r, 'static> for UserResponse {
    fn respond_to(self, _: &Request) -> response::Result<'static> {
        let body: String = self.json().expect("unable to serialize UserResponse");

        Response::build()
            .header(ContentType::JSON)
            .sized_body(body.len(), Cursor::new(body))
            .ok()
    }
}

pub mod filter {
    use bson::{doc, Document};
    use uuid::Uuid;

    #[inline]
    pub fn by_id(id: Uuid) -> Document {
        doc! {
            "_id": bson::Uuid::from(id)
        }
    }

    #[inline]
    pub fn by_username(username: String) -> Document {
        doc! {
            "username": username
        }
    }

    #[inline]
    pub fn by_email(email: String) -> Document {
        doc! {
            "email": email
        }
    }
}
