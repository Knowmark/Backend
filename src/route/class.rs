use crate::data::class::db::ClassCreateData;
use crate::resp::jwt::auth_problem;
use crate::resp::jwt::UserRoleToken;
use crate::resp::problem::Problem;
use crate::route::Role;
use mongodb::Database;
use rocket::http::CookieJar;
use rocket::serde::json::Json;
use rocket::State;

#[utoipa::path(request_body = ClassCreateData)]
#[post("/class", format = "application/json", data = "<class>")]
#[tracing::instrument]
pub fn class_create<'a>(
    class: Json<ClassCreateData>,
    auth: UserRoleToken,
    cookies: &'a CookieJar<'_>,
    db: &State<Database>,
) -> Result<(), Problem> {
    if auth.role < Role::Author {
        return Err(auth_problem("Permission level too low."));
    }
    Ok(())
}
