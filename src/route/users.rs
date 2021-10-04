use bson::spec::BinarySubtype;
use bson::{doc, Bson, Document};
use mongodb::Database;
use regex::Regex;
use rocket::form::Form;
use rocket::http::{CookieJar, Status};
use rocket::State;
use uuid::Uuid;

use crate::config::Config;
use crate::error::Problem;
use crate::jwt::UserRolesToken;
use crate::role::Role;
use crate::route::parse_uuid;
use crate::user::{user_pw_hash, User, USER_COLLECTION_NAME};

lazy_static! {
    static ref EMAIL_REGEX: Regex = Regex::new(r"^[A-z0-9_.+-]+@[A-z0-9-.]+\.\w{2,64}$").unwrap();
}

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
        "id": Bson::Binary(bson::Binary {
            subtype: BinarySubtype::Uuid,
            bytes: id.as_bytes().to_vec(),
        })
    }
}

#[inline]
pub fn filter_user_username(username: String) -> Document {
    doc! {
        "username": username
    }
}

#[inline]
pub fn filter_user_email(email: String) -> Document {
    doc! {
        "email": email
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
        .map_err(|e| Problem::from(e))?;

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
fn bad_email_problem<Email: Into<String>, Detail: Into<String>>(
    email: Email,
    detail: Detail,
) -> Problem {
    Problem::new_untyped(Status::BadRequest, "Bad email.")
        .insert_serialized("email", email.into())
        .detail(detail)
        .clone()
}

#[inline]
fn bad_username_problem<Username: Into<String>, Detail: Into<String>>(
    username: Username,
    detail: Detail,
) -> Problem {
    Problem::new_untyped(Status::BadRequest, "Bad username.")
        .insert_serialized("username", username.into())
        .detail(detail)
        .clone()
}

#[inline]
fn bad_password_problem<S: Into<String>>(detail: S) -> Problem {
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
        if !EMAIL_REGEX.is_match(self.email) {
            return Err(bad_email_problem(
                self.email.clone(),
                "Email format not supported.",
            ));
        }

        if self.username.len() < 5 {
            return Err(bad_username_problem(
                self.username.clone(),
                "Username must be at least 5 characters (bytes) long.",
            ));
        }

        if self.username.len() > 32 {
            return Err(bad_username_problem(
                self.username.clone(),
                "Username can't be longer than 32 (bytes) characters.",
            ));
        }

        if self.password.len() < 8 {
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

#[post("/", data = "<create_user>")]
#[tracing::instrument]
pub async fn user_create<'a>(
    create_user: Form<UserSignupInfo<'_>>,
    cookies: &'a CookieJar<'_>,
    db: &State<Database>,
    c: &State<Config>,
) -> Result<User, Problem> {
    create_user.validate()?;

    if db
        .collection::<User>(USER_COLLECTION_NAME)
        .find_one(filter_user_email(create_user.email.to_string()), None)
        .await
        .expect("Unable to query by email")
        .is_some()
    {
        return Err(bad_email_problem(
            create_user.email.clone(),
            "Email already registered.",
        ));
    }

    if db
        .collection::<User>(USER_COLLECTION_NAME)
        .find_one(filter_user_username(create_user.username.to_string()), None)
        .await
        .expect("Unable to query by username")
        .is_some()
    {
        return Err(bad_username_problem(
            create_user.username.clone(),
            "Username already used.",
        ));
    }

    let mut user = User::new(
        create_user.email.clone(),
        create_user.username.clone(),
        create_user.password.clone(),
    );

    if c.admin_usernames.contains(&user.username) {
        user.user_role = Role::Admin;
    }

    db.collection(USER_COLLECTION_NAME)
        .insert_one(
            bson::to_document(&user).expect("User must be serializable to BSON"),
            None,
        )
        .await
        .map_err(|e| Problem::from(e))?;

    let urt = UserRolesToken::new(&user.clone());
    cookies.add_private(urt.cookie()?);

    Ok(user)
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
        EMAIL_REGEX.is_match(self.identifier.as_str())
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

    if id_user.pw_hash != user_pw_hash(login_user.password.clone()) {
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
