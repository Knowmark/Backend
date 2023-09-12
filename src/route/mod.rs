use std::collections::BTreeMap;

use rocket::{response::Redirect, Build, Rocket, Route};

pub mod class;
pub mod files;
pub mod quiz;
pub mod users;

use class::*;
use files::*;
use quiz::*;
use users::*;

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    data::{class as cd, class::db as cdbd, quiz as qd, user::db as udbd, user::UserResponse},
    resp::{jwt::doc::JWTAuth, problem::Problem},
    role::Role,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        user_list,
        user_get,
        user_create,
        user_delete,
        user_logout,
        login_submit,
        class_create,
        quiz_list,
        quiz_create,
        quiz_info,
        quiz_delete,
        quiz_submit_answers
    ),
    components(schemas(
        Role,
        qd::Quiz,
        qd::QuizPart,
        qd::PartAnswer,
        qd::QuizAnswers,
        qd::ValidationResult,
        qd::QuestionKind,
        qd::AnswerValidation,
        qd::AnswerChoice,
        qd::QuizParticipant,
        cd::ClassRole,
        cdbd::ClassCreateData,
        cdbd::AddUserData,
        QuizListResponse,
        UserResponse,
        udbd::UserLoginData,
        udbd::UserSignupData<'_>,
        Problem
    )),
    modifiers(&JWTAuth, &V1_PREFIX)
)]
pub struct ApiDocV1;

pub struct PathPrefix {
    pub prefix: &'static str,
    pub tags: &'static [&'static str],
}
impl utoipa::Modify for PathPrefix {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let mut new_paths = BTreeMap::new();

        for (path, mut item) in std::mem::take(&mut openapi.paths.paths) {
            for op in item.operations.values_mut() {
                op.tags = Some(self.tags.iter().map(|it| it.to_string()).collect())
            }
            new_paths.insert(self.prefix.to_string() + path.as_ref(), item);
        }

        openapi.paths.paths = new_paths;
    }
}

static V1_PREFIX: PathPrefix = PathPrefix {
    prefix: "/api/v1",
    tags: &["v1"],
};

pub fn api_v1() -> Vec<Route> {
    routes![
        user_list,
        user_get,
        user_create,
        user_delete,
        user_logout,
        login_submit,
        class_create,
        quiz_list,
        quiz_create,
        quiz_info,
        quiz_delete,
        quiz_submit_answers
    ]
}

pub fn mount_api(rocket: Rocket<Build>) -> Rocket<Build> {
    let mut r = rocket.mount("/api/v1", api_v1());

    #[cfg(debug_assertions)]
    {
        r = r.mount(
            "/",
            SwaggerUi::new("/swagger/<_..>").url("/api/v1/openapi.json", ApiDocV1::openapi()),
        );
    }
    r.mount("/", routes![app, app_path])
}
