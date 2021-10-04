use bson::doc;
use bson::serde_helpers::uuid_as_binary;
use crypto::bcrypt::bcrypt;
use rocket::http::ContentType;
use rocket::response::Responder;
use rocket::{response, Request, Response};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::Cursor;
use uuid::Uuid;

use crate::role::Role;

pub static USER_COLLECTION_NAME: &'static str = "users";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    #[serde(with = "uuid_as_binary")]
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub pw_hash: [u8; 24],
    pub user_role: Role,
}

pub fn user_pw_hash<Password: Into<String>>(password: Password) -> [u8; 24] {
    let mut pw_hash: [u8; 24] = [0; 24];

    let mut sha = Sha256::new();
    sha2::Digest::update(&mut sha, password.into().as_bytes());

    bcrypt(
        15,
        &crate::CRYPTO.salt,
        sha.finalize().as_slice(),
        &mut pw_hash,
    );

    pw_hash
}

impl User {
    pub fn new<Email: Into<String>, Username: Into<String>, Password: Into<String>>(
        email: Email,
        username: Username,
        password: Password,
    ) -> User {
        let pw_hash = user_pw_hash(password);

        let uuid = Uuid::new_v4();

        info!("Creating a new user with UUID: {}", uuid.to_string());

        User {
            id: uuid, // TODO: While highly unlikely, what if UUID exists?
            email: email.into(),
            username: username.into(),
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
