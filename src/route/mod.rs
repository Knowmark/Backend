use rocket::{Build, Rocket, Route};

mod files;
mod quiz;
mod users;

use crate::error::{problems, Problem};
use files::*;
use quiz::*;
use std::convert::TryInto;
use users::*;
use uuid::Uuid;

#[inline]
pub fn parse_uuid<Id: Into<String> + Clone>(id: Id) -> Result<Uuid, Problem> {
    let id = id.into();
    match base64::decode(id.as_str())?.try_into() {
        Ok(bytes) => Ok(Uuid::from_bytes(bytes)),
        Err(_) => Err(problems::parse_problem()
            .insert_serialized("parsed", id.as_str())
            .detail("UUID parsing failed.")
            .clone()),
    }
}

pub fn mount_api(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .mount("/api/v1/user", routes![user_get,user_create,user_delete])
        .mount("/api/v1/login", routes![app, login_submit])
        .mount("/api/v1/api", routes![app])
        .mount(
            "/api/v1/quiz",
            routes![quiz_list, quiz_create, quiz_info, quiz_delete],
        )
        .mount("/", routes![app, app_path])
}
