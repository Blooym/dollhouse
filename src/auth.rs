use crate::AppState;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};

#[derive(Debug)]
pub struct AuthProvider {
    valid_tokens: Vec<String>,
}

#[derive(PartialEq, Eq)]
pub enum AuthState {
    Valid,
    Invalid,
}

impl AuthProvider {
    pub fn new(valid_tokens: Vec<String>) -> Self {
        Self { valid_tokens }
    }

    /// Get the [`AuthState`] for the provided token.
    pub fn state_for_token(&self, token: &str) -> AuthState {
        match self.valid_tokens.iter().any(|f| f == token) {
            true => AuthState::Valid,
            false => AuthState::Invalid,
        }
    }

    /// Middleware that will ensure that the request's [`TypedHeader<Authorization<Bearer>>`] contains a
    /// token that resolves as [`AuthState::Valid`].
    pub async fn valid_auth_middleware(
        State(state): State<AppState>,
        TypedHeader(authorization): TypedHeader<Authorization<Bearer>>,
        request: Request,
        next: Next,
    ) -> Result<Response, StatusCode> {
        if state.auth_provider.state_for_token(authorization.token()) != AuthState::Valid {
            return Err(StatusCode::UNAUTHORIZED);
        }
        Ok(next.run(request).await)
    }
}
