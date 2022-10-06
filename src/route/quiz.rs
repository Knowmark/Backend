use crate::data::{Quiz, QUIZ_COLLECTION_NAME};
use crate::resp::jwt::{auth_problem, UserRolesToken};
use crate::resp::problem::Problem;
use crate::role::Role;
use crate::route::parse_uuid;
use bson::spec::BinarySubtype;
use bson::{doc, from_bson, Bson, Document};
use mongodb::Database;
use rocket::futures::StreamExt;
use rocket::serde::json::Json;
use rocket::State;
use uuid::Uuid;

// pub static PART_COLLECTION_NAME: &'static str = "parts";
// pub static PARTICIPANT_COLLECTION_NAME: &'static str = "participants";

#[get("/")]
#[tracing::instrument]
pub async fn quiz_list(db: &State<Database>) -> Result<Json<Vec<Quiz>>, Problem> {
    let mut documents = db
        .collection(QUIZ_COLLECTION_NAME)
        .find(None, None)
        .await
        .expect("unable to list quizzes");

    let mut quizzes: Vec<Quiz> = vec![];

    while let Some(user_result) = documents.next().await {
        let quiz_document = Bson::Document(user_result.unwrap());
        match from_bson(quiz_document) {
            Ok(user) => quizzes.push(user),
            Err(_) => {
                // show must go on?
                warn!("Unable to deserialize Quiz document.")
            }
        }
    }

    Ok(Json(quizzes))
}

#[post("/", format = "application/json", data = "<quiz>")]
#[tracing::instrument]
pub async fn quiz_create(
    quiz: Json<Quiz>,
    auth: UserRolesToken,
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
        "id": Bson::Binary(bson::Binary {
            subtype: BinarySubtype::Uuid,
            bytes: id.as_bytes().to_vec(),
        })
    }
}

#[post("/<id>")]
#[tracing::instrument]
pub async fn quiz_info(id: String, db: &State<Database>) -> Result<Option<Json<Quiz>>, Problem> {
    let uuid = parse_uuid(&id)?;

    let quiz_document = db
        .collection(QUIZ_COLLECTION_NAME)
        .find_one(quiz_id_filter(uuid), None)
        .await
        .expect("Unable to query by id");

    let quiz: Option<Quiz> = match quiz_document {
        Some(doc) => Some(from_bson(Bson::Document(doc)).map_err(|e| Problem::from(e))?),
        None => None,
    };

    Ok(quiz.map(|u| Json(u)))
}

#[delete("/<id>")]
#[tracing::instrument]
pub async fn quiz_delete(
    id: String,
    auth: UserRolesToken,
    db: &State<Database>,
) -> Result<Option<String>, Problem> {
    if auth.role < Role::Author {
        return Err(auth_problem("Permission level too low."));
    }

    let uuid = parse_uuid(&id)?;

    let quiz_document = db
        .collection(QUIZ_COLLECTION_NAME)
        .find_one(quiz_id_filter(uuid), None)
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
        .delete_one(quiz_id_filter(uuid), None)
        .await
        .map_err(|e| Problem::from(e))?;

    Ok(Some(uuid.to_string()))
}
