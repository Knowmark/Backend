use std::borrow::Cow;

use mongodb::Database;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::resp::jwt::UserRoleToken;
use crate::{resp::problem::Problem, role::Role};

use super::filter;
use super::{PasswordHash, User};

pub static USER_COLLECTION_NAME: &str = "user";
pub static USER_SETTINGS_COLLECTION_NAME: &str = "user.settings";

pub mod problem {
    use crate::resp::problem::Problem;
    use rocket::http::Status;
    use uuid::Uuid;

    #[inline]
    pub fn bad_email(email: impl ToString, detail: impl ToString) -> Problem {
        Problem::new_untyped(Status::BadRequest, "Bad email.")
            .insert_str("email", email)
            .detail(detail)
            .to_owned()
    }

    #[inline]
    pub fn bad_username(username: impl ToString, detail: impl ToString) -> Problem {
        Problem::new_untyped(Status::BadRequest, "Bad username.")
            .insert_str("username", username)
            .detail(detail)
            .to_owned()
    }

    #[inline]
    pub fn bad_password(detail: impl ToString) -> Problem {
        Problem::new_untyped(Status::BadRequest, "Bad password.")
            .detail(detail)
            .to_owned()
    }

    #[inline]
    pub fn not_found(id: Uuid) -> Problem {
        Problem::new_untyped(Status::NotFound, "User doesn't exist.")
            .insert("id", id.to_string())
            .clone()
    }

    #[inline]
    pub fn bad_login(is_email: bool) -> Problem {
        Problem::new_untyped(
            Status::Unauthorized,
            if is_email {
                "Bad email or password."
            } else {
                "Bad username or password."
            },
        )
    }
}

#[derive(Clone, FromForm, ToSchema)]
pub struct UserSignupData<'r> {
    #[schema(format = "email")]
    pub email: Cow<'r, str>,
    pub username: Cow<'r, str>,
    #[schema(format = "password")]
    pub password: Cow<'r, str>,
}

impl<'r> UserSignupData<'r> {
    pub fn id(&self) -> Uuid {
        Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            [self.email.as_ref(), self.username.as_ref()]
                .join("")
                .as_bytes(),
        )
    }
}

impl std::fmt::Debug for UserSignupData<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UserSignupInfo:{}", self.username)
    }
}

impl UserSignupData<'_> {
    pub fn validate(&self) -> Result<(), Problem> {
        if !self.email.contains("@") {
            return Err(problem::bad_email(
                self.email.to_string(),
                "Not a valid e-mail address.",
            ));
        }

        if self.username.len() < 5 {
            return Err(problem::bad_username(
                self.username.to_string(),
                "Username must be at least 5 characters (bytes) long.",
            ));
        }

        if self.username.len() > 32 {
            return Err(problem::bad_username(
                self.username.to_string(),
                "Username can't be longer than 32 (bytes) characters.",
            ));
        }

        if self.password.len() <= 8 {
            return Err(problem::bad_password(
                "Password must be at least 8 characters (bytes) long.",
            ));
        }

        if self.password.len() > 1024 {
            return Err(problem::bad_password(
                "Passwords longer than 1024 characters aren't supported.",
            ));
        }

        Ok(())
    }
}

#[derive(Clone, FromForm, ToSchema)]
pub struct UserLoginData {
    pub username: String,
    #[schema(format = "password")]
    pub password: String,
}

impl std::fmt::Debug for UserLoginData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UserLoginInfo:{}", self.username)
    }
}

impl UserLoginData {
    pub fn is_email(&self) -> bool {
        self.username.contains("@")
    }

    pub fn validate(&self, is_email: bool) -> Result<(), Problem> {
        if self.username.len() < 5
            || self.username.len() > 32
            || self.password.len() < 8
            || self.password.len() > 50
        {
            return Err(problem::bad_login(is_email));
        }

        Ok(())
    }
}

// TODO: CreateUserDbExt fns shouldn't be erroring with Problem
pub trait CreateUserDbExt {
    async fn create_user<'a>(
        &self,
        create_user: UserSignupData<'_>,
        admin_names: impl AsRef<[String]>,
    ) -> Result<(UserRoleToken, User), Problem>;

    async fn get_user(&self, id: Uuid) -> Result<Option<User>, Problem>;

    async fn find_user_by_email(&self, email: impl AsRef<str>) -> Result<Option<User>, Problem>;
    async fn find_user_by_username(
        &self,
        username: impl AsRef<str>,
    ) -> Result<Option<User>, Problem>;

    async fn delete_user(&self, id: Uuid) -> Result<Option<User>, Problem>;
}

impl CreateUserDbExt for Database {
    async fn create_user<'a>(
        &self,
        create_user: UserSignupData<'_>,
        admin_names: impl AsRef<[String]>,
    ) -> Result<(UserRoleToken, User), Problem> {
        let existing_email = self.find_user_by_email(&create_user.email).await?;

        if let Some(existing) = existing_email {
            let create_hash = PasswordHash::new(create_user.password.as_ref());
            return if existing.pw_hash == create_hash {
                let urt = UserRoleToken::new(&existing);
                Ok((urt, existing))
            } else {
                Err(problem::bad_email(
                    create_user.email.to_string(),
                    "Email already registered.",
                ))
            };
        }

        if self
            .find_user_by_username(&create_user.username)
            .await?
            .is_some()
        {
            return Err(problem::bad_username(
                create_user.username.to_string(),
                "Username already used.",
            ));
        }

        let mut user = User::from(create_user);

        if admin_names.as_ref().contains(&user.username) {
            user.user_role = Role::Admin;
        }

        let urt = UserRoleToken::new(&user);

        self.collection(USER_COLLECTION_NAME)
            .insert_one(
                bson::to_document(&user).expect("User must be serializable to BSON"),
                None,
            )
            .await
            .map_err(|e| Problem::from(e))?;

        Ok((urt, user))
    }

    async fn get_user(&self, id: Uuid) -> Result<Option<User>, Problem> {
        self.collection(USER_COLLECTION_NAME)
            .find_one(filter::by_id(id), None)
            .await
            .map_err(Problem::from)
    }

    async fn find_user_by_email(&self, email: impl AsRef<str>) -> Result<Option<User>, Problem> {
        self.collection(USER_COLLECTION_NAME)
            .find_one(filter::by_email(email.as_ref().to_string()), None)
            .await
            .map_err(Problem::from)
    }

    async fn find_user_by_username(
        &self,
        username: impl AsRef<str>,
    ) -> Result<Option<User>, Problem> {
        self.collection(USER_COLLECTION_NAME)
            .find_one(filter::by_username(username.as_ref().to_string()), None)
            .await
            .map_err(Problem::from)
    }

    async fn delete_user(&self, id: Uuid) -> Result<Option<User>, Problem> {
        self.collection(USER_COLLECTION_NAME)
            .find_one_and_delete(filter::by_id(id), None)
            .await
            .map_err(|e| Problem::from(e))
    }
}
