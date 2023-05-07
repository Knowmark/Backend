use std::io::Cursor;
use From;

use rocket::http::hyper::header::CONTENT_LANGUAGE;
use rocket::http::ContentType;
use rocket::http::Status;
use rocket::response::Responder;
use rocket::{response, Request, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt::{Display, Formatter};
use utoipa::ToSchema;

/// Implements [RFC7807](https://tools.ietf.org/html/rfc7807).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Problem {
    #[serde(skip)]
    pub status: Status,
    pub type_uri: String,
    pub title: String,

    pub detail: Option<String>,
    pub instance_uri: Option<String>,

    pub body: Map<String, Value>,
}

impl Default for Problem {
    fn default() -> Self {
        Problem {
            status: Status::InternalServerError,
            type_uri: "about:blank".to_string(),
            title: "Problem".to_string(),
            detail: None,
            instance_uri: None,
            body: Map::new(),
        }
    }
}

impl Problem {
    pub fn new(status: Status, type_uri: impl ToString, title: impl ToString) -> Problem {
        Problem {
            status,
            type_uri: type_uri.to_string(),
            title: title.to_string(),
            ..Default::default()
        }
    }

    // TODO: Add problem type URIs
    pub fn new_untyped(status: Status, title: impl ToString) -> Problem {
        Problem {
            status,
            type_uri: "about:blank".to_string(),
            title: title.to_string(),
            ..Default::default()
        }
    }

    pub fn detail(&mut self, value: impl ToString) -> &mut Problem {
        self.detail = Some(value.to_string());
        self
    }

    pub fn instance_uri(&mut self, value: String) -> &mut Problem {
        self.instance_uri = Some(value);
        self
    }

    pub fn insert_json_value(&mut self, key: impl ToString, value: Value) -> &mut Problem {
        self.body.insert(key.to_string(), value);
        self
    }

    pub fn insert<V: Serialize>(&mut self, key: impl ToString, value: V) -> &mut Problem {
        self.body.insert(
            key.to_string(),
            serde_json::to_value(value).expect("data must be JSON serializable"),
        );
        self
    }

    pub fn insert_str(&mut self, key: impl ToString, value: impl ToString) -> &mut Problem {
        self.body
            .insert(key.to_string(), Value::String(value.to_string()));
        self
    }

    pub fn append(&mut self, value: Map<String, Value>) -> &mut Problem {
        self.body.append(&mut value.clone());
        self
    }

    pub fn append_serialized(&mut self, data: impl Serialize) -> &mut Problem {
        let body = serde_json::to_value(data).expect("data must be JSON serializable");

        match body {
            Value::Object(mut map) => self.body.append(&mut map),
            _ => panic!("appended data must be an object when serialized."),
        }

        self
    }
}

impl Display for Problem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.status, self.title)
    }
}

impl std::error::Error for Problem {}

impl<'r> Responder<'r, 'static> for Problem {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let mut body = self.body.clone();

        // Following are required by rfc7807
        body.insert(String::from("type"), serde_json::Value::from(self.type_uri));
        body.insert(String::from("title"), serde_json::Value::from(self.title));

        // Optional parameters as specified by rfc7807
        if self.detail.is_some() {
            body.insert(
                String::from("detail"),
                serde_json::Value::from(self.detail.unwrap()),
            );
        }
        body.insert(
            String::from("status"),
            serde_json::Value::from(self.status.code),
        );
        if self.instance_uri.is_some() {
            body.insert(
                String::from("instance"),
                serde_json::Value::from(self.instance_uri.unwrap()),
            );
        }

        let body_string = serde_json::to_string(&body)
            .expect("JSON map keys and values must be JSON serializable");

        Response::build()
            .status(self.status)
            .header(ContentType::new("application", "problem+json"))
            .raw_header(CONTENT_LANGUAGE.as_str(), "en")
            .sized_body(body_string.len(), Cursor::new(body_string))
            .ok()
    }
}

pub mod problems {
    use crate::resp::problem::Problem;
    use rocket::http::Status;

    #[inline]
    pub fn parse_problem() -> Problem {
        Problem::new_untyped(
            Status::BadRequest,
            "There was a problem parsing part of the request.",
        )
    }
}

#[allow(dead_code)]
impl From<mongodb::error::Error> for Problem {
    fn from(e: mongodb::error::Error) -> Self {
        use mongodb::error::ErrorKind;

        fn mongodb_problem() -> Problem {
            Problem::new_untyped(
                Status::InternalServerError,
                "MongoDB failed while processing request.",
            )
        }

        fn access_problem() -> Problem {
            Problem::new_untyped(
                Status::InternalServerError,
                "Server was unable to access MongoDB.",
            )
        }

        fn bad_db_request() -> Problem {
            Problem::new_untyped(
                Status::InternalServerError,
                "MongoDB was unable to process bad server request.",
            )
        }

        fn bson_problem() -> Problem {
            Problem::new_untyped(
                Status::InternalServerError,
                "There was a problem with handling MongoDB bson.",
            )
        }

        fn timeout_problem() -> Problem {
            Problem::new_untyped(
                Status::InternalServerError,
                "A timeout occurred while accessing MongoDB.",
            )
        }

        match e.kind.as_ref() {
            ErrorKind::InvalidArgument { .. } => bad_db_request(),
            ErrorKind::Authentication { .. } => access_problem(),
            ErrorKind::BsonDeserialization(_) => bson_problem(),
            ErrorKind::BsonSerialization(_) => bson_problem(),
            ErrorKind::BulkWrite(_) => bad_db_request(),
            ErrorKind::Command(_) => bad_db_request(),
            ErrorKind::DnsResolve { .. } => access_problem(),
            ErrorKind::Internal { .. } => mongodb_problem(),
            ErrorKind::Io(_) => mongodb_problem()
                .detail("An IO error occurred. Submitted data might not be properly stored.")
                .clone(),
            ErrorKind::ConnectionPoolCleared { .. } => mongodb_problem(),
            ErrorKind::InvalidResponse { .. } => mongodb_problem(),
            ErrorKind::ServerSelection { .. } => access_problem(),
            ErrorKind::SessionsNotSupported => mongodb_problem(),
            ErrorKind::InvalidTlsConfig { .. } => access_problem(),
            ErrorKind::Write(_) => mongodb_problem()
                .detail("A write error occurred. Submitted data might not be properly stored.")
                .clone(),
            ErrorKind::Transaction { .. } => mongodb_problem(),
            ErrorKind::IncompatibleServer { .. } => access_problem(),
            _ => mongodb_problem(),
        }
    }
}

impl From<bson::de::Error> for Problem {
    fn from(_: bson::de::Error) -> Self {
        Problem::new_untyped(
            Status::InternalServerError,
            "An error occurred while processing BSON data.",
        )
    }
}

impl From<serde_json::Error> for Problem {
    fn from(_: serde_json::Error) -> Self {
        Problem::new_untyped(
            Status::InternalServerError,
            "An error occurred while processing JSON data.",
        )
    }
}

impl From<jsonwebtoken::errors::Error> for Problem {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;

        match e.into_kind() {
            ErrorKind::ExpiredSignature => {
                Problem::new_untyped(Status::Unauthorized, "Expired JWT signature.")
            }
            _ => Problem::new_untyped(Status::Unauthorized, "Error while handling JWT."),
        }
    }
}

impl From<std::io::Error> for Problem {
    fn from(_: std::io::Error) -> Self {
        Problem::new_untyped(Status::InternalServerError, "Server IO error")
    }
}
