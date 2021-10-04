use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rocket::http::{Cookie, CookieJar, Status};
use rocket::request::{self, FromRequest, Request};
use serde::{Deserialize, Serialize};

use crate::error::Problem;
use crate::role::Role;
use crate::user::User;
use bson::serde_helpers::uuid_as_binary;
use rocket::outcome::Outcome::{Failure, Success};
use std::borrow::Borrow;
use uuid::Uuid;

pub static AUTH_COOKIE_NAME: &'static str = "jwt_auth";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRolesToken {
    #[serde(with = "jwt_numeric_date")]
    iat: DateTime<Utc>,
    #[serde(with = "jwt_numeric_date")]
    exp: DateTime<Utc>,
    #[serde(with = "uuid_as_binary")]
    pub user: Uuid,
    pub role: Role,
}

impl UserRolesToken {
    pub fn new(user: &User) -> UserRolesToken {
        let now = Utc::now();
        UserRolesToken {
            iat: now,
            exp: now + Duration::weeks(1),
            user: user.id.clone(),
            role: user.user_role.clone(),
        }
    }

    pub fn encode_jwt(&self) -> Result<String, jsonwebtoken::errors::Error> {
        let header = Header::new(Algorithm::PS256);
        let key = EncodingKey::from_rsa_pem(crate::CRYPTO.user_auth_key.private.as_slice())
            .expect("user_auth private key isn't valid. Unable to encode JWT.");

        Ok(encode(&header, &self, &key)?)
    }

    pub fn cookie(self) -> Result<Cookie<'static>, jsonwebtoken::errors::Error> {
        Ok(Cookie::build(AUTH_COOKIE_NAME, self.encode_jwt()?)
            .secure(true)
            .path("/")
            .http_only(true)
            .finish())
    }
}

pub fn auth_problem<S: Into<String>>(detail: S) -> Problem {
    Problem::new_untyped(Status::Unauthorized, "Unable to authorize user.")
        .detail(detail)
        .clone()
}

pub fn extract_claims(cookies: &CookieJar) -> Result<UserRolesToken, Problem> {
    let auth_cookie = cookies.get_private(AUTH_COOKIE_NAME);
    let token = match auth_cookie {
        Some(jwt) => jwt.value().to_owned(),
        None => {
            return Err(auth_problem("Couldn't extract auth JWT from cookie."));
        }
    };
    tracing::debug!("extracted jwt auth from cookie");

    match decode::<UserRolesToken>(
        &token,
        &DecodingKey::from_rsa_pem(crate::CRYPTO.user_auth_key.public.borrow())
            .expect("user_auth public key isn't valid. Unable to encode JWT."),
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
impl<'r> FromRequest<'r> for UserRolesToken {
    type Error = Problem;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        tracing::debug!("extracting user roles token from request cookies");
        let claims = match extract_claims(req.cookies()) {
            Ok(it) => it,
            Err(e) => {
                tracing::debug!("unable to extract claims from cookies");
                return Failure((Status::Unauthorized, e));
            }
        };

        return Success(claims);
    }
}

mod jwt_numeric_date {
    // Based on: https://github.com/Keats/jsonwebtoken/blob/master/examples/custom_chrono.rs

    //! Custom serialization of DateTime<Utc> to conform to the JWT spec (RFC 7519 section 2, "Numeric Date")
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    /// Serializes a DateTime<Utc> to a Unix timestamp (milliseconds since 1970/1/1T00:00:00T)
    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let timestamp = date.timestamp();
        serializer.serialize_i64(timestamp)
    }

    /// Attempts to deserialize an i64 and use as a Unix timestamp
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Utc.timestamp_opt(i64::deserialize(deserializer)?, 0)
            .single() // If there are multiple or no valid DateTimes from timestamp, return None
            .ok_or_else(|| serde::de::Error::custom("Invalid Unix timestamp value."))
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

        let urt = UserRolesToken {
            iat: now,
            exp: now + Duration::weeks(1),
            user,
            role: Role::Admin,
        };

        let token = urt.encode_jwt().expect("encoding should work for example");

        let decoded: UserRolesToken = match decode(
            &token,
            &DecodingKey::from_rsa_pem(crate::CRYPTO.user_auth_key.public.borrow())
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
