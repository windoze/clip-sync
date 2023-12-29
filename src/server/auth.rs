use poem::{
    http::StatusCode,
    web::headers::{self, authorization::Bearer, HeaderMapExt},
    Endpoint, Middleware, Request,
};

pub struct ApiKeyAuth {
    api_key: Option<String>,
}

impl ApiKeyAuth {
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key }
    }
}

impl<E: Endpoint> Middleware<E> for ApiKeyAuth {
    type Output = ApiKeyAuthEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        ApiKeyAuthEndpoint {
            ep,
            api_key: self.api_key.clone(),
        }
    }
}

pub struct ApiKeyAuthEndpoint<E> {
    ep: E,
    api_key: Option<String>,
}

#[poem::async_trait]
impl<E: Endpoint> Endpoint for ApiKeyAuthEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, req: Request) -> poem::Result<Self::Output> {
        // Skip auth if no api key is set
        if self.api_key.is_none() {
            return self.ep.call(req).await;
        }
        if let Some(auth) = req.headers().typed_get::<headers::Authorization<Bearer>>() {
            if auth.0.token() == self.api_key.as_ref().unwrap() {
                return self.ep.call(req).await;
            }
        }
        Err(poem::Error::from_status(StatusCode::UNAUTHORIZED))
    }
}
