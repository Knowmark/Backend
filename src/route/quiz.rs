use std::time::Duration;

use crate::data::quiz::{Quiz, QuizAnswers, ValidationResult, QUIZ_COLLECTION_NAME};
use crate::resp::jwt::{auth_problem, UserRoleToken};
use crate::resp::problem::Problem;
use crate::role::Role;
use bson::spec::BinarySubtype;
use bson::{doc, from_bson, Bson, Document};
use chrono::{DateTime, Utc};
use mongodb::Database;
use rocket::futures::StreamExt;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

// pub static PART_COLLECTION_NAME: &'static str = "parts";
// pub static PARTICIPANT_COLLECTION_NAME: &'static str = "participants";

#[derive(Debug, Serialize, ToSchema)]
pub struct QuizListResponse {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub desc: String,
    #[serde(
        default = "Uuid::new_v4",
        with = "bson::serde_helpers::uuid_1_as_binary"
    )] // TODO: Remove default
    pub author: Uuid,
    #[serde(default = "Utc::now")]
    pub created: DateTime<Utc>,

    pub time_limit: Option<Duration>,

    pub open_on: Option<DateTime<Utc>>,
    pub close_on: Option<DateTime<Utc>>,
}

impl From<Quiz> for QuizListResponse {
    fn from(value: Quiz) -> Self {
        Self {
            id: value.id,
            name: value.name,
            desc: value.desc,
            author: value.author,
            created: value.created,
            time_limit: value.time_limit,
            open_on: value.open_on,
            close_on: value.close_on,
        }
    }
}

// TODO: Quiz list paging

/// List all quiz documents
#[utoipa::path(
    responses(
        (status = 200, description = "List of quizes", body = Vec<QuizListResponse>),
    )
)]
#[get("/quiz")]
#[tracing::instrument]
pub async fn quiz_list(
    db: &State<Database>,
    user: Option<UserRoleToken>,
) -> Result<Json<Vec<QuizListResponse>>, Problem> {
    let mut documents = db
        .collection(QUIZ_COLLECTION_NAME)
        .find(None, None)
        .await
        .expect("unable to list quizzes");

    let mut quizzes: Vec<QuizListResponse> = vec![];

    while let Some(user_result) = documents.next().await {
        let quiz_document = Bson::Document(user_result.unwrap());
        match from_bson::<Quiz>(quiz_document) {
            Ok(quiz) => quizzes.push(quiz.into()),
            Err(_) => {
                warn!("Unable to deserialize Quiz document.")
            }
        }
    }

    Ok(Json(quizzes))
}

/// Create a quiz
#[utoipa::path(request_body = Quiz)]
#[post("/quiz", format = "application/json", data = "<quiz>")]
#[tracing::instrument]
pub async fn quiz_create(
    quiz: Json<Quiz>,
    auth: UserRoleToken,
    db: &State<Database>,
) -> Result<(), Problem> {
    if auth.role < Role::Author {
        return Err(auth_problem("Permission level too low."));
    }

    db.collection(QUIZ_COLLECTION_NAME)
        .insert_one(
            bson::to_document(&quiz.0).expect("Unable to serialize Quiz struct into BSON"),
            None,
        )
        .await
        .map_err(|e| Problem::from(e))?;

    Ok(())
}

#[inline]
pub fn quiz_id_filter(id: Uuid) -> Document {
    doc! {
        "_id": Bson::Binary(bson::Binary {
            subtype: BinarySubtype::Uuid,
            bytes: id.as_bytes().to_vec(),
        })
    }
}

/// Get quiz information
#[utoipa::path(
    params(
        ("id", description = "quiz ID")
    ),
    responses(
        (status = 401, description = "Missing/expired token or insufficient privileges", body = Problem),
        (status = 200, description = "Information about the quiz", body = Option<Quiz>),
        (status = 404, description = "Querried quiz doesn't exist"),
    )
)]
#[get("/quiz/<id>")]
#[tracing::instrument]
pub async fn quiz_info(id: Uuid, db: &State<Database>) -> Result<Option<Json<Quiz>>, Problem> {
    let quiz_document = db
        .collection(QUIZ_COLLECTION_NAME)
        .find_one(quiz_id_filter(id), None)
        .await
        .expect("Unable to query by id");

    let quiz: Option<Quiz> = match quiz_document {
        Some(doc) => Some(from_bson(Bson::Document(doc)).map_err(|e| Problem::from(e))?),
        None => None,
    };

    Ok(quiz.map(|u| Json(u)))
}

/// Delete a quiz
#[utoipa::path(
    params(
        ("id", description = "quiz ID")
    ),
    responses(
        (status = 401, description = "Missing/expired token", body = Problem),
        (status = 200, description = "Information about existing user", body = UserResponse),
        (status = 404, description = "Querried user doesn't exist"),
    ),
    security(
        ("jwt" = [])
    )
)]
#[delete("/quiz/<id>")]
#[tracing::instrument]
pub async fn quiz_delete(
    id: Uuid,
    auth: UserRoleToken,
    db: &State<Database>,
) -> Result<Option<String>, Problem> {
    let quiz_document = db
        .collection(QUIZ_COLLECTION_NAME)
        .find_one(quiz_id_filter(id), None)
        .await
        .expect("Unable to query by id");

    let quiz: Quiz = match quiz_document {
        Some(doc) => from_bson(Bson::Document(doc)).map_err(|e| Problem::from(e))?,
        None => return Ok(None),
    };

    if auth.role < Role::Admin && quiz.author != auth.user {
        return Err(auth_problem("Quiz not owned by user."));
    }

    db.collection::<Quiz>(QUIZ_COLLECTION_NAME)
        .delete_one(quiz_id_filter(id), None)
        .await
        .map_err(|e| Problem::from(e))?;

    Ok(Some(id.to_string()))
}

/// Submit quiz answers
#[utoipa::path(
    request_body(content = QuizAnswers, content_type="application/json"),
    responses(
        (status = 200, description = "Validation results", body = ValidationResult),
        (status = 400, description = "Provided invalid answers", body = Problem),
        (status = 401, description = "Missing/expired token", body = Problem),
    ),
    security(
        ("jwt" = [])
    )
)]
#[post("/quiz/<id>", format = "application/json", data = "<answers>")]
#[tracing::instrument]
pub async fn quiz_submit_answers(
    id: Uuid,
    answers: Json<QuizAnswers>,
    auth: UserRoleToken,
    db: &State<Database>,
) -> Result<Json<ValidationResult>, Problem> {
    let answer = answers.0;

    let quiz_document = db
        .collection(QUIZ_COLLECTION_NAME)
        .find_one(quiz_id_filter(id), None)
        .await
        .expect("Unable to query by id");

    let quiz: Quiz = match quiz_document {
        Some(doc) => from_bson(Bson::Document(doc)).map_err(|e| Problem::from(e))?,
        None => {
            return Err(Problem::new_untyped(
                Status::BadRequest,
                "Quiz for provided answers doesn't exist.",
            ))
        }
    };

    Ok(Json(answer.validate(&quiz)))
}
