use base64::{alphabet::Alphabet, engine::GeneralPurposeConfig, Engine};
use rocket::{Build, Request, Rocket};

pub mod files;
pub mod quiz;
pub mod users;

use crate::{
    resp::problem::{problems, Problem},
    util::base64_engine,
};
use files::*;
use quiz::*;
use rocket::request::{FromRequest, Outcome};
use std::convert::{Infallible, TryInto};
use users::*;
use uuid::Uuid;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct PageState {
    pub page_length: u32,
    pub page: u32,
}

impl Default for PageState {
    fn default() -> Self {
        PageState {
            page_length: 20,
            page: 0,
        }
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for PageState {
    type Error = Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let length: Option<u32> = request
            .query_value("len")
            .map(|it| it.ok())
            .flatten()
            .or_else(|| request.query_value("l").map(|it| it.ok()).flatten());

        let page: Option<u32> = request
            .query_value("page")
            .map(|it| it.ok())
            .flatten()
            .or_else(|| request.query_value("p").map(|it| it.ok()).flatten());

        if let Some(p) = page {
            Outcome::Success(PageState {
                page_length: length.unwrap_or(20),
                page: p,
            })
        } else {
            Outcome::Success(Default::default())
        }
    }
}

#[inline]
pub fn parse_uuid(id: impl AsRef<str>) -> Result<Uuid, Problem> {
    match base64_engine().decode(id.as_ref())?.try_into() {
        Ok(bytes) => Ok(Uuid::from_bytes(bytes)),
        Err(_) => Err(problems::parse_problem()
            .insert_serialized("parsed", id.as_ref())
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
