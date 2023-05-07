use std::collections::BTreeMap;

use rocket::{Build, Rocket, Route};

pub mod files;
pub mod quiz;
pub mod users;

use files::*;
use quiz::*;
use users::*;

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    data::{
        quiz as qd,
        user::db::{UserLoginData, UserSignupData},
        user::UserResponse,
    },
    resp::{jwt::doc::JWTAuth, problem::Problem},
    role::Role,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        user_get,
        user_create,
        user_delete,
        app,
        login_submit,
        quiz_list,
        quiz_create,
        quiz_info,
        quiz_delete
    ),
    components(schemas(
        Role,
        qd::Quiz,
        qd::Part,
        qd::AnswerType,
        qd::AnswerValidation,
        qd::AnswerChoice,
        qd::ParticipantInfo,
        UserResponse,
        UserLoginData,
        UserSignupData<'_>,
        Problem
    )),
    modifiers(&JWTAuth, &V1_PREFIX)
)]
pub struct ApiDocV1;

pub struct PathPrefix(pub &'static str);
static V1_PREFIX: PathPrefix = PathPrefix("/api/v1");

impl utoipa::Modify for PathPrefix {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let mut new_paths = BTreeMap::new();

        for (path, item) in std::mem::take(&mut openapi.paths.paths) {
            new_paths.insert(self.0.to_string() + path.as_ref(), item);
        }

        openapi.paths.paths = new_paths;
    }
}

pub fn api_v1() -> Vec<Route> {
    routes![
        user_get,
        user_create,
        user_delete,
        login_submit,
        quiz_list,
        quiz_create,
        quiz_info,
        quiz_delete
    ]
}

pub fn mount_api(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .mount("/api/v1", api_v1())
        .mount(
            "/",
            SwaggerUi::new("/swagger/<_..>").url("/api/v1/openapi.json", ApiDocV1::openapi()),
        )
        .mount("/", routes![app, app_path])
}
