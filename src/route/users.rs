use bson::doc;
use mongodb::Database;
use rocket::form::Form;
use rocket::http::private::cookie::CookieBuilder;
use rocket::http::{CookieJar, Status};
use rocket::serde::json::Json;
use rocket::State;
use uuid::Uuid;

use crate::data::user::db::problem as user_problem;
use crate::data::user::db::{CreateUserDbExt, UserLoginData, UserSignupData};
use crate::data::user::{PasswordHash, UserResponse};
use crate::resp::jwt::{UserRoleToken, AUTH_COOKIE_NAME};
use crate::resp::problem::Problem;
use crate::role::Role;
use crate::security::Security;
use crate::settings::Settings;

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

/// Get information about the user
#[utoipa::path(
    params(
        ("id", description = "user ID")
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
#[get("/user/<id>")]
#[tracing::instrument]
pub async fn user_get(
    id: Uuid,
    auth: UserRoleToken,
    db: &State<Database>,
) -> Result<Option<UserResponse>, Problem> {
    if auth.role < Role::Normal {
        return Err(Problem::new_untyped(
            Status::Unauthorized,
            "only members can view other users",
        ));
    }

    db.get_user(id).await.map(|ok| ok.map(UserResponse::from))
}

/// Create a user
#[utoipa::path(
    request_body(content = UserSignupData<'_>, content_type="application/x-www-form-urlencoded"),
    responses(
        (status = 401, description = "Missing/expired token", body = Problem),
        (status = 200, description = "User created", body = UserResponse)
    )
)]
#[post("/user", data = "<create_user>")]
#[tracing::instrument]
pub async fn user_create<'a>(
    create_user: Form<UserSignupData<'_>>,
    cookies: &'a CookieJar<'_>,
    db: &State<Database>,
    c: &State<Settings>,
    security: &State<Security>,
) -> Result<Json<UserResponse>, Problem> {
    create_user.validate()?;

    let (token, user) = db
        .create_user(create_user.into_inner(), &security.salt, &c.admin_usernames)
        .await?;
    cookies.add(token.cookie(&security.jwt_keys.private)?);

    Ok(Json(UserResponse::from(user)))
}

/// Login via a login form
#[utoipa::path(
    request_body(content = UserLoginData, content_type="application/x-www-form-urlencoded"),
    responses(
        (status = 401, description = "Bad login infomation", body = Problem),
        (status = 200, description = "Login user info and cookies", body = UserResponse)
    )
)]
#[post("/login", data = "<login_user>")]
#[tracing::instrument]
pub async fn login_submit<'a>(
    login_user: Form<UserLoginData>,
    cookies: &'a CookieJar<'_>,
    db: &State<Database>,
    security: &State<Security>,
) -> Result<UserResponse, Problem> {
    let is_email = login_user.is_email();

    login_user.validate(is_email)?;

    // VULN: Prevent login_submit brute force attacks by checking login source

    let document = match is_email {
        true => db.find_user_by_email(login_user.username.clone()).await,
        false => db.find_user_by_username(login_user.username.clone()).await,
    }?;

    let user = document.ok_or_else(|| user_problem::bad_login(is_email))?;

    if user.pw_hash != PasswordHash::new(login_user.password.clone(), security.salt) {
        return Err(user_problem::bad_login(is_email));
    }

    let urt = UserRoleToken::new(&user);
    cookies.add(urt.cookie(&security.jwt_keys.private)?);

    Ok(UserResponse::from(user))
}

/// Delete a user
#[utoipa::path(
    params(
        ("id", description = "ID of user to delete")
    ),
    responses(
        (status = 401, description = "Insufficient privileges to delete user", body = Problem),
        (status = 200, description = "ID of deleted user", body = Uuid)
    ),
    security(
        ("jwt" = [])
    )
)]
#[delete("/user/<id>")]
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
        data::user::{
            db::{CreateUserDbExt, UserSignupData},
            User, UserResponse,
        },
        resp::jwt::{HasAuthCookie, UserRoleToken},
        role::Role,
        security::{self, Security},
    };
    use mongodb::Database;
    use rocket::{
        http::{ContentType, Header, Status},
        local::asynchronous::Client,
        Rocket,
    };
    use tracing::Level;
    use uuid::Uuid;

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

    async fn test_backend() -> Rocket<rocket::Build> {
        crate::create(Some(Level::TRACE))
            .await
            .expect("unable to build test backend")
    }

    #[rocket::async_test]
    async fn v1_user_create_works() {
        let client = Client::tracked(test_backend().await)
            .await
            .expect("invalid backend");
        let db: &Database = client.rocket().state().unwrap();
        let security: &Security = client.rocket().state().unwrap();

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
            response
                .get_auth_cookie(&security.jwt_keys.public)
                .is_some(),
            "jwt_auth cookie wasn't present"
        );
        tracing::info!("{:#?}", &response);

        let response_data: UserResponse =
            response.into_json().await.expect("invalid response json");

        db.delete_user(response_data.id)
            .await
            .expect("unable to delete test user");
    }

    #[rocket::async_test]
    async fn v1_user_create_can_login() {
        let client = Client::tracked(test_backend().await)
            .await
            .expect("invalid backend");
        let db: &Database = client.rocket().state().unwrap();
        let security: &Security = client.rocket().state().unwrap();

        let user: UserSignupData = example_signup_data("v1_user_create_can_login");
        db.create_user(user.clone(), &security.salt, &[])
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
            response
                .get_auth_cookie(&security.jwt_keys.public)
                .is_some(),
            "jwt_auth cookie wasn't present"
        );

        let response_data: UserResponse =
            response.into_json().await.expect("invalid response json");

        db.delete_user(response_data.id)
            .await
            .expect("unable to delete test user");
    }

    #[rocket::async_test]
    async fn v1_login_submit_works() {
        let client = Client::tracked(test_backend().await)
            .await
            .expect("invalid backend");
        let db: &Database = client.rocket().state().unwrap();
        let security: &Security = client.rocket().state().unwrap();

        let user = example_signup_data("v1_login_submit_works");
        db.create_user(user.clone(), &security.salt, &[])
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
            response
                .get_auth_cookie(&security.jwt_keys.public)
                .is_some(),
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
        let client = Client::tracked(test_backend().await)
            .await
            .expect("invalid backend");
        let db: &Database = client.rocket().state().unwrap();
        let security: &Security = client.rocket().state().unwrap();

        let user = example_signup_data("v1_user_delete_doesnt_work_for_unauthorized_users");
        db.create_user(user.clone(), &security.salt, &[])
            .await
            .expect("unable to create user");

        let delete_uri = format!("/api/v1/user/{}", user.id());

        let response = client.delete(&delete_uri).dispatch().await;
        assert_eq!(
            response.status(),
            Status::Unauthorized,
            "expected unauthorized response"
        );

        db.delete_user(user.id())
            .await
            .expect("unable to delete test user");
    }

    #[rocket::async_test]
    async fn v1_user_delete_works_for_same_user() {
        let client = Client::tracked(test_backend().await)
            .await
            .expect("invalid backend");
        let db: &Database = client.rocket().state().unwrap();
        let security: &Security = client.rocket().state().unwrap();

        let user = example_signup_data("v1_user_delete_works_for_same_user");
        db.create_user(user.clone(), &security.salt, &[])
            .await
            .expect("unable to create user");

        let urt = UserRoleToken::new(&user.to_user(&security.salt));
        assert_eq!(urt.user, user.id());
        let jwt_cookie = urt
            .cookie(&security.jwt_keys.private)
            .expect("unable to encode UserRoleToken cookie");
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
        let client = Client::tracked(test_backend().await)
            .await
            .expect("invalid backend");
        let db: &Database = client.rocket().state().unwrap();
        let security: &Security = client.rocket().state().unwrap();

        let user = example_signup_data("v1_user_delete_works_for_admin_user");
        db.create_user(user.clone(), &security.salt, &[])
            .await
            .expect("unable to create user");

        let mut admin = User::new("admin@example.com", "admin", "admin_pass", &security.salt);
        admin.user_role = Role::Admin;
        let urt = UserRoleToken::new(&admin);
        let jwt_cookie = urt
            .cookie(&security.jwt_keys.private)
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
