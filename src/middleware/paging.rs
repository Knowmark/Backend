use std::convert::TryFrom;
use std::marker::PhantomData;

use bson::{from_document, Document};
use mongodb::{Cursor, Database};
use rocket::form::FromForm;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::resp::problem::Problem;

mod sealed {
    pub struct HasContext<'k> {
        pub collection: &'k str,
        pub key: &'k str,
    }
    pub struct NoContext;

    pub trait PagingContext {}
    impl PagingContext for HasContext<'_> {}
    impl PagingContext for NoContext {}
}
use sealed::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct PageState<'r, T: FromForm<'r> + Serialize, Ctx: PagingContext = NoContext> {
    pub length: u8,
    pub from: Option<T>,
    context: Ctx,
    _phantom: PhantomData<&'r ()>,
}

impl<'r, T: FromForm<'r> + Serialize, Ctx: PagingContext> PageState<'r, T, Ctx> {
    pub fn page_over<'k>(
        self,
        collection: &'k str,
        key: &'k str,
    ) -> PageState<'r, T, HasContext<'k>> {
        PageState {
            length: self.length,
            from: self.from,
            context: HasContext { collection, key },
            _phantom: self._phantom,
        }
    }
}

impl<'r, 'k, T: FromForm<'r> + Serialize> PageState<'r, T, HasContext<'k>> {
    pub async fn entries<R: DeserializeOwned>(
        &'r self,
        db: &'r Database,
    ) -> Result<Vec<R>, Problem> {
        let mut position: Cursor<R> = db
            .collection(self.context.collection)
            .find(
                self.from.as_ref().map(|it| {
                    let mut doc = Document::new();
                    let val = bson::to_bson(it).expect("unable to serialize 'from' to bson");
                    doc.insert(self.context.key, val);
                    doc
                }),
                None,
            )
            .await
            .map_err(|e| Problem::from(e))?;

        let mut result = Vec::with_capacity(self.length as usize);
        for _ in 0..self.length {
            let raw = position.current();
            match Document::try_from(raw) {
                Ok(entry) => {
                    let entry_identifier = entry.get(self.context.key).map(|it| it.to_string());

                    match from_document(entry) {
                        Ok(value) => result.push(value),
                        Err(err) => {
                            tracing::warn!(
                                "Can't read document as {} in '{}' collection ({}: {:?}); {}",
                                std::any::type_name::<T>(),
                                self.context.collection,
                                self.context.key,
                                entry_identifier,
                                err
                            );
                        }
                    }
                }
                Err(err) => {
                    let kv = raw
                        .get(self.context.key)
                        .ok()
                        .flatten()
                        .map(|it| it.to_raw_bson());
                    tracing::error!(
                        "Can't read document in '{}' collection ({}: {:?}); {}",
                        self.context.collection,
                        self.context.key,
                        kv,
                        err
                    );
                }
            }

            match position.advance().await {
                Ok(true) => {}
                Ok(false) => break,
                Err(e) => return Err(Problem::from(e)),
            }
        }

        Ok(result)
    }
}

const DEFAULT_LENGTH: u8 = 20;

impl<'r, T: FromForm<'r> + Serialize> Default for PageState<'r, T, NoContext> {
    fn default() -> Self {
        PageState {
            length: DEFAULT_LENGTH,
            from: None,
            context: NoContext,
            _phantom: PhantomData,
        }
    }
}

#[rocket::async_trait]
impl<'r, T: FromForm<'r> + Serialize> FromRequest<'r> for PageState<'r, T, NoContext> {
    type Error = rocket::form::Errors<'r>;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let length: u8 = match request.query_value("length") {
            Some(Ok(it)) => it,
            None => DEFAULT_LENGTH,
            Some(Err(e)) => return Outcome::Failure((Status::BadRequest, e)),
        };

        let from: Option<T> = match request.query_value("from") {
            Some(Ok(it)) => Some(it),
            None => None,
            Some(Err(e)) => return Outcome::Failure((Status::BadRequest, e)),
        };

        Outcome::Success(PageState {
            length,
            from,
            context: NoContext,
            _phantom: PhantomData,
        })
    }
}
