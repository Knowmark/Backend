#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct PageState {
    pub page_length: u32,
    pub page: u32,
}

impl Default for PageState {
    fn default() -> Self {
        PageState {
            page_length: 20,
            page: 0,
        }
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for PageState {
    type Error = Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let length: Option<u32> = request
            .query_value("len")
            .map(|it| it.ok())
            .flatten()
            .or_else(|| request.query_value("l").map(|it| it.ok()).flatten());

        let page: Option<u32> = request
            .query_value("page")
            .map(|it| it.ok())
            .flatten()
            .or_else(|| request.query_value("p").map(|it| it.ok()).flatten());

        if let Some(p) = page {
            Outcome::Success(PageState {
                page_length: length.unwrap_or(20),
                page: p,
            })
        } else {
            Outcome::Success(Default::default())
        }
    }
}
