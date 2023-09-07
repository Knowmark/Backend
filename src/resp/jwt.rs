use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rocket::http::{Cookie, CookieJar, Status};
use rocket::request::{self, FromRequest, Request};
use rocket::time::OffsetDateTime;
use serde::{Deserialize, Serialize};

use super::util::date_time_as_unix_seconds;
use crate::data::user::User;
use crate::resp::problem::Problem;
use crate::role::Role;
use crate::security::Security;
use rocket::outcome::Outcome::{Failure, Success};
use uuid::Uuid;

pub static AUTH_COOKIE_NAME: &'static str = "jwt_auth";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRoleToken {
    #[serde(with = "date_time_as_unix_seconds")]
    iat: DateTime<Utc>,
    #[serde(with = "date_time_as_unix_seconds")]
    exp: DateTime<Utc>,
    pub user: Uuid,
    pub role: Role,
}

impl UserRoleToken {
    pub fn new(user: &User) -> UserRoleToken {
        let now = Utc::now();
        UserRoleToken {
            iat: now,
            exp: now + Duration::weeks(1),
            user: user.id.clone(),
            role: user.user_role.clone(),
        }
    }

    pub fn encode_jwt(
        &self,
        private_key: impl AsRef<[u8]>,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let header = Header::new(Algorithm::PS256);
        let key = EncodingKey::from_rsa_pem(private_key.as_ref())
            .expect("user_auth private key isn't valid. Unable to encode JWT.");

        Ok(encode(&header, &self, &key)?)
    }

    pub fn cookie(
        &self,
        private_key: impl AsRef<[u8]>,
    ) -> Result<Cookie<'static>, jsonwebtoken::errors::Error> {
        Ok(
            Cookie::build(AUTH_COOKIE_NAME, self.encode_jwt(private_key)?)
                .secure(true)
                .expires(OffsetDateTime::from_unix_timestamp(self.exp.timestamp()).ok())
                .path("/")
                .http_only(true)
                .finish(),
        )
    }
}

pub fn auth_problem(detail: impl ToString) -> Problem {
    Problem::new_untyped(Status::Unauthorized, "Unable to authorize user.")
        .detail(detail)
        .clone()
}

pub fn extract_claims(
    cookies: &CookieJar,
    public_key: impl AsRef<[u8]>,
) -> Result<UserRoleToken, Problem> {
    let auth_cookie = cookies.get(AUTH_COOKIE_NAME);
    let token = match auth_cookie {
        Some(jwt) => jwt.value().to_owned(),
        None => {
            return Err(auth_problem("No JWT auth cookie."));
        }
    };
    tracing::debug!("extracted jwt auth from cookie");

    match decode::<UserRoleToken>(
        &token,
        &DecodingKey::from_rsa_pem(public_key.as_ref())
            .expect("user_auth public key isn't valid. Unable to decode JWT."),
        &Validation::new(Algorithm::PS256),
    )
    .map(|data| data.claims)
    {
        Ok(it) => {
            tracing::debug!("decoded user roles token for user: {}", it.user);

            Ok(it)
        }
        Err(_) => Err(auth_problem("JWT cookie was malformed.")),
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UserRoleToken {
    type Error = Problem;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let security: &Security = req.rocket().state().unwrap();

        tracing::trace!("extracting user roles token from request cookies");
        let claims: UserRoleToken = match extract_claims(req.cookies(), &security.jwt_keys.public) {
            Ok(it) => it,
            Err(e) => {
                tracing::debug!("unable to extract claims from cookies");
                return Failure((Status::Unauthorized, e));
            }
        };

        return Success(claims);
    }
}

pub mod doc {
    use utoipa::openapi::security::*;

    #[derive(Clone, Copy)]
    pub struct JWTAuth;

    impl Into<SecurityScheme> for JWTAuth {
        fn into(self) -> SecurityScheme {
            let mut http = Http::new(HttpAuthScheme::Bearer);
            http.bearer_format = Some("JWT".to_string());
            http.scheme = HttpAuthScheme::Bearer;
            SecurityScheme::Http(http)
        }
    }

    impl utoipa::Modify for JWTAuth {
        fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
            let c = openapi.components.as_mut().unwrap();
            c.add_security_scheme("jwt", *self)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::SubsecRound;

    #[test]
    fn jwt_configured_properly() {
        let mut now = Utc::now();
        now = now.round_subsecs(0);

        let user = Uuid::new_v4();

        let urt = UserRoleToken {
            iat: now,
            exp: now + Duration::weeks(1),
            user,
            role: Role::Admin,
        };

        let security = Security::load();

        let token = urt
            .encode_jwt(&security.jwt_keys.private)
            .expect("encoding should work for example");

        let decoded: UserRoleToken = match decode(
            &token,
            &DecodingKey::from_rsa_pem(&security.jwt_keys.public)
                .expect("user_auth public key isn't valid. Unable to encode JWT."),
            &Validation::new(Algorithm::PS256),
        )
        .map(|data| data.claims)
        {
            Ok(it) => it,
            Err(_) => panic!("unable to decode encoded token"),
        };

        assert_eq!(now, decoded.iat);
        assert_eq!(now + Duration::weeks(1), decoded.exp);
        assert_eq!(user, decoded.user);
        assert_eq!(decoded.role, Role::Admin);
    }
}

pub trait HasAuthCookie {
    fn get_auth_cookie(&self, public_key: impl AsRef<[u8]>) -> Option<UserRoleToken>;
}

#[cfg(test)]
impl HasAuthCookie for rocket::local::asynchronous::LocalResponse<'_> {
    fn get_auth_cookie(&self, public_key: impl AsRef<[u8]>) -> Option<UserRoleToken> {
        tracing::trace!("extracting user roles token from request cookies");
        extract_claims(self.cookies(), public_key).ok()
    }
}

#[cfg(test)]
impl HasAuthCookie for rocket::local::blocking::LocalResponse<'_> {
    fn get_auth_cookie(&self, public_key: impl AsRef<[u8]>) -> Option<UserRoleToken> {
        tracing::trace!("extracting user roles token from request cookies");
        extract_claims(self.cookies(), public_key).ok()
    }
}
