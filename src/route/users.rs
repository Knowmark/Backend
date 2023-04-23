use bson::{doc, Document};
use mongodb::Database;
use rocket::form::Form;
use rocket::http::{CookieJar, Status};
use rocket::serde::json::Json;
use rocket::State;
use uuid::Uuid;

use crate::config::Config;
use crate::resp::jwt::UserRolesToken;
use crate::resp::problem::Problem;
use crate::role::Role;
use crate::route::parse_uuid;
use crate::user::{PasswordHash, User, USER_COLLECTION_NAME};

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

#[inline]
pub fn filter_user_id(id: Uuid) -> Document {
    doc! {
        "_id": bson::Uuid::from(id)
    }
}

#[inline]
pub fn filter_user_username(username: impl ToString) -> Document {
    doc! {
        "username": username.to_string()
    }
}

#[inline]
pub fn filter_user_email(email: impl ToString) -> Document {
    doc! {
        "email": email.to_string()
    }
}

#[get("/<id>")]
#[tracing::instrument]
pub async fn user_get(id: String, db: &State<Database>) -> Result<Option<User>, Problem> {
    let uuid = parse_uuid(&id)?;

    let user_document = db
        .collection(USER_COLLECTION_NAME)
        .find_one(filter_user_id(uuid), None)
        .await
        .map_err(Problem::from)?;

    match user_document {
        Some(doc) => Ok(Some(
            bson::from_document(doc).expect("Unable to deserialize BSON into User struct."),
        )),
        None => Ok(None),
    }
}

#[derive(Clone, FromForm)]
pub struct UserSignupInfo<'r> {
    email: &'r str,
    username: &'r str,
    password: &'r str,
}

impl std::fmt::Debug for UserSignupInfo<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UserSignupInfo:{}", self.username)
    }
}

#[inline]
fn bad_email_problem(email: impl ToString, detail: impl ToString) -> Problem {
    Problem::new_untyped(Status::BadRequest, "Bad email.")
        .insert_serialized("email", email.to_string())
        .detail(detail)
        .clone()
}

#[inline]
fn bad_username_problem(username: impl ToString, detail: impl ToString) -> Problem {
    Problem::new_untyped(Status::BadRequest, "Bad username.")
        .insert_serialized("username", username.to_string())
        .detail(detail)
        .clone()
}

#[inline]
fn bad_password_problem(detail: impl ToString) -> Problem {
    Problem::new_untyped(Status::BadRequest, "Bad password.")
        .detail(detail)
        .clone()
}

#[inline]
fn user_not_found(id: Uuid) -> Problem {
    Problem::new_untyped(Status::NotFound, "User doesn't exist.")
        .insert_serialized("id", id.to_string())
        .clone()
}
#[inline]
fn login_problem(is_email: bool) -> Problem {
    Problem::new_untyped(
        Status::Unauthorized,
        if is_email {
            "Bad email or password."
        } else {
            "Bad username or password."
        },
    )
}

impl UserSignupInfo<'_> {
    pub fn validate(&self) -> Result<(), Problem> {
        if !self.email.contains("@") {
            return Err(bad_email_problem(
                self.email.to_string(),
                "Not a valid e-mail address.",
            ));
        }

        if self.username.len() < 5 {
            return Err(bad_username_problem(
                self.username.to_string(),
                "Username must be at least 5 characters (bytes) long.",
            ));
        }

        if self.username.len() > 32 {
            return Err(bad_username_problem(
                self.username.to_string(),
                "Username can't be longer than 32 (bytes) characters.",
            ));
        }

        if self.password.len() <= 8 {
            return Err(bad_password_problem(
                "Password must be at least 8 characters (bytes) long.",
            ));
        }

        if self.password.len() > 1024 {
            return Err(bad_password_problem(
                "Passwords longer than 1024 characters aren't supported.",
            ));
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct UserCreatedResponse {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub user_role: Role,
}

impl From<User> for UserCreatedResponse {
    fn from(value: User) -> Self {
        UserCreatedResponse {
            id: value.id,
            email: value.email,
            username: value.username,
            user_role: value.user_role,
        }
    }
}

#[post("/", data = "<create_user>")]
#[tracing::instrument]
pub async fn user_create<'a>(
    create_user: Form<UserSignupInfo<'_>>,
    cookies: &'a CookieJar<'_>,
    db: &State<Database>,
    c: &State<Config>,
) -> Result<Json<UserCreatedResponse>, Problem> {
    create_user.validate()?;

    if let Some(existing) = db
        .collection::<User>(USER_COLLECTION_NAME)
        .find_one(filter_user_email(&create_user.email), None)
        .await
        .expect("Unable to query by email")
    {
        return if existing.pw_hash == PasswordHash::new(create_user.password) {
            let urt = UserRolesToken::new(&existing);
            cookies.add_private(urt.cookie()?);
            Ok(Json(UserCreatedResponse::from(existing)))
        } else {
            Err(bad_email_problem(
                create_user.email.to_string(),
                "Email already registered.",
            ))
        };
    }

    if db
        .collection::<User>(USER_COLLECTION_NAME)
        .find_one(filter_user_username(create_user.username.to_string()), None)
        .await
        .expect("Unable to query by username")
        .is_some()
    {
        return Err(bad_username_problem(
            create_user.username.to_string(),
            "Username already used.",
        ));
    }

    let mut user = User::new(
        create_user.email.to_string(),
        create_user.username.to_string(),
        create_user.password.to_string(),
    );

    if c.admin_usernames.contains(&user.username) {
        user.user_role = Role::Admin;
    }

    let urt = UserRolesToken::new(&user);
    cookies.add_private(urt.cookie()?);

    db.collection(USER_COLLECTION_NAME)
        .insert_one(
            bson::to_document(&user).expect("User must be serializable to BSON"),
            None,
        )
        .await
        .map_err(|e| Problem::from(e))?;

    Ok(Json(UserCreatedResponse::from(user)))
}

#[derive(Clone, FromForm)]
pub struct UserLoginInfo {
    identifier: String,
    password: String,
}

impl std::fmt::Debug for UserLoginInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UserLoginInfo:{}", self.identifier)
    }
}

impl UserLoginInfo {
    fn is_email(&self) -> bool {
        self.identifier.contains("@")
    }

    pub fn validate(&self, is_email: bool) -> Result<(), Problem> {
        if self.identifier.len() < 5
            || self.identifier.len() > 32
            || self.password.len() < 8
            || self.password.len() > 50
        {
            return Err(login_problem(is_email));
        }

        Ok(())
    }
}

#[post("/", data = "<login_user>")]
#[tracing::instrument]
pub async fn login_submit<'a>(
    login_user: Form<UserLoginInfo>,
    cookies: &'a CookieJar<'_>,
    db: &State<Database>,
) -> Result<User, Problem> {
    let is_email = login_user.is_email();

    login_user.validate(is_email)?;

    // VULN(0): Prevent brute force attacks by checking login source

    let user_document = match db
        .collection(USER_COLLECTION_NAME)
        .find_one(
            match is_email {
                true => filter_user_email(login_user.identifier.clone()),
                false => filter_user_username(login_user.identifier.clone()),
            },
            None,
        )
        .await
        .expect("Unable to query by username")
    {
        Some(doc) => doc,
        None => return Err(login_problem(is_email)),
    };

    let id_user: User =
        bson::from_document(user_document).expect("Unable to deserialize BSON into User struct");

    if id_user.pw_hash != PasswordHash::new(login_user.password.clone()) {
        return Err(login_problem(is_email));
    }

    let urt = UserRolesToken::new(&id_user.clone());
    cookies.add_private(urt.cookie()?);

    Ok(id_user)
}

#[delete("/<id>")]
#[tracing::instrument]
pub async fn user_delete(id: String, db: &State<Database>) -> Result<User, Problem> {
    let uuid = parse_uuid(&id)?;

    let removed_document = db
        .collection(USER_COLLECTION_NAME)
        .find_one_and_delete(filter_user_id(uuid), None)
        .await
        .map_err(|e| Problem::from(e))?;

    match removed_document {
        Some(user_document) => {
            let user: User = bson::from_document(user_document)
                .expect("Unable to deserialize User struct from BSON");
            Ok(user)
        }
        None => Err(user_not_found(uuid)),
    }
}

#[cfg(test)]
mod user {
    use crate::route::users::{filter_user_id, UserCreatedResponse};
    use mongodb::Database;
    use rocket::{
        http::{ContentType, Header, Status},
        local::asynchronous::Client,
    };
    use uuid::Uuid;

    use crate::user::{User, USER_COLLECTION_NAME};

    async fn delete_user_entry(db: &Database, id: Uuid) {
        let delete_result = db
            .collection::<User>(USER_COLLECTION_NAME)
            .delete_one(filter_user_id(id), None)
            .await
            .expect("unable to perform delete user operation");

        assert_eq!(delete_result.deleted_count, 1, "delete created user");
    }

    #[rocket::async_test]
    async fn user_create_works() {
        let client = Client::tracked(crate::create().await)
            .await
            .expect("valid backend");
        let db: &Database = client.rocket().state().unwrap();

        let response = client
            .post("/api/v1/user")
            .header(Header::new(
                "Content-Type",
                "application/x-www-form-urlencoded",
            ))
            .body("email=example.user@example.com&username=example_user&password=3x4mpleUs3r")
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok, "ok response");
        assert_eq!(
            response.content_type(),
            Some(ContentType::JSON),
            "application/json response"
        );
        assert!(
            response.cookies().get_private("jwt_auth").is_some(),
            "jwt_auth cookie present"
        );

        let response_data: UserCreatedResponse =
            response.into_json().await.expect("invalid response json");

        delete_user_entry(db, response_data.id).await;
    }

    #[rocket::async_test]
    async fn user_create_can_login() {
        let client = Client::tracked(crate::create().await)
            .await
            .expect("valid backend");
        let db: &Database = client.rocket().state().unwrap();

        client
            .post("/api/v1/user")
            .header(Header::new(
                "Content-Type",
                "application/x-www-form-urlencoded",
            ))
            .body("email=example.user@example.com&username=example_user&password=3x4mpleUs3r")
            .dispatch()
            .await;

        let response = client
            .post("/api/v1/user")
            .header(Header::new(
                "Content-Type",
                "application/x-www-form-urlencoded",
            ))
            .body("email=example.user@example.com&username=example_user&password=3x4mpleUs3r")
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok, "ok response");
        assert_eq!(
            response.content_type(),
            Some(ContentType::JSON),
            "application/json response"
        );
        assert!(
            response.cookies().get_private("jwt_auth").is_some(),
            "jwt_auth cookie present"
        );

        let response_data: UserCreatedResponse =
            response.into_json().await.expect("invalid response json");

        delete_user_entry(db, response_data.id).await;
    }
}
