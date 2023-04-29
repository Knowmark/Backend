use bson::doc;
use mongodb::Database;
use rocket::form::Form;
use rocket::http::private::cookie::CookieBuilder;
use rocket::http::{CookieJar, Status};
use rocket::serde::json::Json;
use rocket::State;
use uuid::Uuid;

use crate::config::Config;
use crate::data::user::db::problem as user_problem;
use crate::data::user::db::{CreateUserDbExt, UserCreatedResponse, UserLoginData, UserSignupData};
use crate::data::user::{PasswordHash, User};
use crate::resp::jwt::{UserRoleToken, AUTH_COOKIE_NAME};
use crate::resp::problem::Problem;
use crate::role::Role;

/* TODO: Support paging
// Responder isn't implemented for Vec.
#[get("/")]
pub async fn user_list(db: &State<Database>) -> Result<Vec<User>, Problem> {
    let mut user_cursor = db.collection(USER_COLLECTION_NAME)
        .find(None, None)
        .await
        .map_err(|e| Problem::from(e))?;

    let mut users: Vec<User> = vec![];
    while let Some(user_result) = user_cursor.next().await {
        let user_document = Bson::Document(user_result.unwrap());
        match from_bson(user_document) {
            Ok(user) => {
                users.push(user)
            }
            Err(_) => {
                // show must go on?
                warn!("Unable to deserialize User document.")
            }
        }
    }

    Ok(users)
}
*/

#[get("/<id>")]
#[tracing::instrument]
pub async fn user_get(
    id: Uuid,
    auth: UserRoleToken,
    db: &State<Database>,
) -> Result<Option<User>, Problem> {
    if auth.role < Role::Normal {
        return Err(Problem::new_untyped(
            Status::Unauthorized,
            "only members can view other users",
        ));
    }

    db.get_user(id).await
}

#[post("/", data = "<create_user>")]
#[tracing::instrument]
pub async fn user_create<'a>(
    create_user: Form<UserSignupData<'_>>,
    cookies: &'a CookieJar<'_>,
    db: &State<Database>,
    c: &State<Config>,
) -> Result<Json<UserCreatedResponse>, Problem> {
    create_user.validate()?;

    let (token, user) = db
        .create_user(create_user.into_inner(), &c.admin_usernames)
        .await?;
    cookies.add(token.cookie()?);

    Ok(Json(UserCreatedResponse::from(user)))
}

#[post("/", data = "<login_user>")]
#[tracing::instrument]
pub async fn login_submit<'a>(
    login_user: Form<UserLoginData>,
    cookies: &'a CookieJar<'_>,
    db: &State<Database>,
) -> Result<User, Problem> {
    let is_email = login_user.is_email();

    login_user.validate(is_email)?;

    // VULN: Prevent login_submit brute force attacks by checking login source

    let document = match is_email {
        true => db.find_user_by_email(login_user.identifier.clone()).await,
        false => {
            db.find_user_by_username(login_user.identifier.clone())
                .await
        }
    }?;

    let user = document.ok_or_else(|| user_problem::bad_login(is_email))?;

    if user.pw_hash != PasswordHash::new(login_user.password.clone()) {
        return Err(user_problem::bad_login(is_email));
    }

    let urt = UserRoleToken::new(&user);
    cookies.add(urt.cookie()?);

    Ok(user)
}

#[delete("/<id>")]
#[tracing::instrument]
pub async fn user_delete<'a>(
    id: Uuid,
    auth: UserRoleToken,
    cookies: &'a CookieJar<'_>,
    db: &State<Database>,
) -> Result<String, Problem> {
    if auth.user != id && auth.role < Role::Admin {
        return Err(Problem::new_untyped(
            Status::Unauthorized,
            "only admin can get a list of all users",
        ));
    }

    let removed = db.delete_user(id).await?;

    if let Some(removed) = removed {
        if auth.user == id {
            cookies.remove(CookieBuilder::new(AUTH_COOKIE_NAME, "").finish())
        }
        Ok(removed.id.to_string())
    } else {
        Err(user_problem::not_found(id))
    }
}

///////////////////////
//       TESTS
///////////////////////

#[cfg(test)]
mod user_endpoints {
    use std::{borrow::Cow, collections::HashMap, str::FromStr};

    use crate::{
        data::user::{db::CreateUserDbExt, User},
        resp::jwt::{HasAuthCookie, UserRoleToken},
        role::Role,
        route::users::UserCreatedResponse,
    };
    use mongodb::Database;
    use rocket::{
        http::{ContentType, Header, Status},
        local::asynchronous::Client,
    };
    use uuid::Uuid;

    use super::UserSignupData;

    fn example_signup_data(user: impl AsRef<str>) -> UserSignupData<'static> {
        UserSignupData {
            email: Cow::Owned(user.as_ref().to_string() + "@example.com"),
            username: Cow::Owned(user.as_ref().to_string()),
            password: Cow::Owned(
                user.as_ref()
                    .replace("o", "0")
                    .replace("e", "3")
                    .replace("a", "4")
                    .replace("_", "#"),
            ),
        }
    }

    fn create_form_body(user: impl AsRef<str>) -> String {
        let data = example_signup_data(user);
        format!(
            "email={}&username={}&password={}",
            data.email, data.username, data.password
        )
    }

    fn login_form_body(user: impl AsRef<str>) -> String {
        let data = example_signup_data(user);
        format!("identifier={}&password={}", data.username, data.password)
    }

    #[rocket::async_test]
    async fn v1_user_create_works() {
        let client = Client::tracked(crate::create().await)
            .await
            .expect("invalid backend");
        let db: &Database = client.rocket().state().unwrap();

        let response: rocket::local::asynchronous::LocalResponse = client
            .post("/api/v1/user")
            .header(Header::new(
                "Content-Type",
                "application/x-www-form-urlencoded",
            ))
            .body(create_form_body("v1_user_create_works"))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok, "an ok response");
        assert_eq!(
            response.content_type(),
            Some(ContentType::JSON),
            "not a application/json response"
        );
        assert!(
            response.get_auth_cookie().is_some(),
            "jwt_auth cookie wasn't present"
        );

        let response_data: UserCreatedResponse =
            response.into_json().await.expect("invalid response json");

        db.delete_user(response_data.id)
            .await
            .expect("unable to delete test user");
    }

    #[rocket::async_test]
    async fn v1_user_create_can_login() {
        let client = Client::tracked(crate::create().await)
            .await
            .expect("invalid backend");
        let db: &Database = client.rocket().state().unwrap();

        let user: UserSignupData = example_signup_data("v1_user_create_can_login");
        db.create_user(user.clone(), &[])
            .await
            .expect("unable to create test user");

        let response = client
            .post("/api/v1/user")
            .header(Header::new(
                "Content-Type",
                "application/x-www-form-urlencoded",
            ))
            .body(create_form_body(user.username))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok, "an ok response");
        assert_eq!(
            response.content_type(),
            Some(ContentType::JSON),
            "not a application/json response"
        );
        assert!(
            response.get_auth_cookie().is_some(),
            "jwt_auth cookie wasn't present"
        );

        let response_data: UserCreatedResponse =
            response.into_json().await.expect("invalid response json");

        db.delete_user(response_data.id)
            .await
            .expect("unable to delete test user");
    }

    #[rocket::async_test]
    async fn v1_login_submit_works() {
        let client = Client::tracked(crate::create().await)
            .await
            .expect("invalid backend");
        let db: &Database = client.rocket().state().unwrap();

        let user = example_signup_data("v1_login_submit_works");
        db.create_user(user.clone(), &[])
            .await
            .expect("unable to create test user");

        let response = client
            .post("/api/v1/login")
            .header(Header::new(
                "Content-Type",
                "application/x-www-form-urlencoded",
            ))
            .body(login_form_body(user.username))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok, "an ok response");
        assert_eq!(
            response.content_type(),
            Some(ContentType::JSON),
            "not a application/json response"
        );
        assert!(
            response.get_auth_cookie().is_some(),
            "no jwt_auth cookie present"
        );

        let user_info: HashMap<String, String> =
            response.into_json().await.expect("invalid response json");
        let response_id = user_info
            .get("id")
            .map(|it| Uuid::from_str(it).ok())
            .flatten()
            .expect("invalid response user UUID");

        db.delete_user(response_id)
            .await
            .expect("unable to delete test user");
    }

    #[rocket::async_test]
    async fn v1_user_delete_doesnt_work_for_unauthorized_users() {
        let client = Client::tracked(crate::create().await)
            .await
            .expect("invalid backend");
        let db: &Database = client.rocket().state().unwrap();

        let user = example_signup_data("v1_user_delete_doesnt_work_for_unauthorized_users");
        db.create_user(user.clone(), &[])
            .await
            .expect("unable to create user");

        let delete_uri = format!("/api/v1/user/{}", user.id());

        let response = client.delete(&delete_uri).dispatch().await;
        assert_eq!(
            response.status(),
            Status::Unauthorized,
            "expected unauthorized response"
        );
    }

    #[rocket::async_test]
    async fn v1_user_delete_works_for_same_user() {
        let client = Client::tracked(crate::create().await)
            .await
            .expect("invalid backend");
        let db: &Database = client.rocket().state().unwrap();

        let user = example_signup_data("v1_user_delete_works_for_same_user");
        db.create_user(user.clone(), &[])
            .await
            .expect("unable to create user");

        let urt = UserRoleToken::new(&User::from(user.clone()));
        assert_eq!(urt.user, user.id());
        let jwt_cookie = urt.cookie().expect("unable to encode UserRoleToken cookie");
        let delete_uri = format!("/api/v1/user/{}", user.id());

        let response = client
            .delete(delete_uri)
            .cookie(jwt_cookie)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok, "an ok response");

        let response_id = response
            .into_string()
            .await
            .map(|it| Uuid::parse_str(&it).ok())
            .flatten();

        assert_eq!(Some(user.id()), response_id, "deleted unexpected user");
    }

    #[rocket::async_test]
    async fn v1_user_delete_works_for_admin_user() {
        let client = Client::tracked(crate::create().await)
            .await
            .expect("invalid backend");
        let db: &Database = client.rocket().state().unwrap();

        let user = example_signup_data("v1_user_delete_works_for_admin_user");
        db.create_user(user.clone(), &[])
            .await
            .expect("unable to create user");

        let mut admin = User::new("admin@example.com", "admin", "admin_pass");
        admin.user_role = Role::Admin;
        let urt = UserRoleToken::new(&admin);
        let jwt_cookie = urt
            .cookie()
            .expect("unable to encode admin UserRoleToken cookie");
        let delete_uri = format!("/api/v1/user/{}", user.id());

        let response = client
            .delete(delete_uri)
            .cookie(jwt_cookie)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok, "an ok response");

        let response_id = response
            .into_string()
            .await
            .map(|it| Uuid::parse_str(&it).ok())
            .flatten();

        assert_eq!(Some(user.id()), response_id, "deleted unexpected user");
    }
}
