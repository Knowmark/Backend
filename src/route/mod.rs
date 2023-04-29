use base64::Engine;
use rocket::{Build, Rocket};

pub mod files;
pub mod quiz;
pub mod users;

use crate::{
    resp::problem::{problems, Problem},
    util::base64_engine,
};
use files::*;
use quiz::*;
use std::convert::TryInto;
use users::*;
use uuid::Uuid;

#[inline]
pub fn parse_uuid(id: impl AsRef<str>) -> Result<Uuid, Problem> {
    match base64_engine().decode(id.as_ref())?.try_into() {
        Ok(bytes) => Ok(Uuid::from_bytes(bytes)),
        Err(_) => Err(problems::parse_problem()
            .insert("parsed", id.as_ref())
            .detail("UUID parsing failed.")
            .clone()),
    }
}

pub fn mount_api(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .mount("/api/v1/user", routes![user_get, user_create, user_delete])
        .mount("/api/v1/login", routes![app, login_submit])
        .mount("/api/v1/api", routes![app])
        .mount(
            "/api/v1/quiz",
            routes![quiz_list, quiz_create, quiz_info, quiz_delete],
        )
        .mount("/", routes![app, app_path])
}
