use std::fmt;
use std::future::{ready, Ready};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::task::{Context, Poll};

use actix_web::{dev::{Service, ServiceRequest, ServiceResponse, Transform}, Error, HttpResponse, ResponseError};
use actix_web::http::StatusCode;
use futures_util::future::LocalBoxFuture;
use crate::modules::users::user::{Level, User};
use crate::modules::users::UsersSessions;

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.


pub struct UserAuthentication {
    level: Level
}

impl UserAuthentication {
    pub fn new(level: Level) -> Self {
        UserAuthentication { level }
    }
}

// Middleware factory is `Transform` trait
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for UserAuthentication
    where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
        S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = UserAuthenticationMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(UserAuthenticationMiddleware {
            service: Arc::new(Mutex::new(service)),
            level: Arc::new(Mutex::new(self.level))
        }))
    }
}

pub struct UserAuthenticationMiddleware<S> {
    service: Arc<Mutex<S>>,
    level: Arc<Mutex<Level>>
}

impl<S, B> Service<ServiceRequest> for UserAuthenticationMiddleware<S>
    where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
        S::Future: 'static
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&self, mut req: ServiceRequest) -> Self::Future {

        let inner = Arc::clone(&self.service);
        let level = Arc::clone(&self.level);

        Box::pin(async move {
            if let Some(auth_error) = user_authentication_validation(&mut req, level).await {
                return Err(actix_web::Error::from(auth_error));
            }
            let service = inner.lock().await;
            service.call(req).await
        })
    }
}

async fn user_authentication_validation(req: &mut ServiceRequest, level: Arc<Mutex<Level>>) -> Option<TheHttpResponse> {

    //  Attempt to fetch username from headers
    let username = match req.headers().get("username") {
        Some(username) => {
            match username.to_str() {
                Ok(username) => username,
                Err(_) => {
                    return Some(
                        TheHttpResponse::status_code(StatusCode::FORBIDDEN)
                            .with_body("Invalid username".to_string())
                    )
                }
            }
        },
        None => {
            return Some(
                TheHttpResponse::status_code(StatusCode::FORBIDDEN)
                    .with_body("No username provided or found".to_string())
            )
        }
    };

    //  Attempt to fetch token from headers
    let token = match req.headers().get("token") {
        Some(token) => {
            match token.to_str() {
                Ok(token) => token,
                Err(_) => {
                    return Some(
                        TheHttpResponse::status_code(StatusCode::FORBIDDEN)
                            .with_body("Invalid session token".to_string())
                    )
                }
            }
        },
        None => {
            return Some(
                TheHttpResponse::status_code(StatusCode::FORBIDDEN)
                    .with_body("No session token received".to_string())
            )
        }
    };

    //  Fetch user data from database
    let user = match User::select_by_username(username).await {
        Ok(Some(user)) => user,
        Ok(None) => return Some(
            TheHttpResponse::status_code(StatusCode::FORBIDDEN)
                .with_body("User not found".to_string())
        ),
        Err(_) => {
            return Some(
                TheHttpResponse::status_code(StatusCode::INTERNAL_SERVER_ERROR)
                    .with_body("Failed to fetch user data".to_string())
            )
        }
    };

    //  Check if user is logged in
    if !UsersSessions::instance().is_user_logged_in(user.get_id()).await {
        return Some(
            TheHttpResponse::status_code(StatusCode::UNAUTHORIZED)
                .with_body("User not logged in".to_string())
        )
    };

    //  Validate user level
    if *user.get_level() < *level.lock().await {
        return Some(
            TheHttpResponse::status_code(StatusCode::FORBIDDEN)
                .with_body("User level below required privileges".to_string())
        )
    }

    //  Validate session token
    match super::modules::users::users_sessions::validate_session_token(&user, token).await {
        Ok(true) => {},
        Ok(false) => {
            return Some(
                TheHttpResponse::status_code(StatusCode::FORBIDDEN)
                    .with_body("Invalid session token".to_string())
            )
        },
        Err(_) => {
            return Some(
                TheHttpResponse::status_code(StatusCode::INTERNAL_SERVER_ERROR)
                    .with_body("Failed to validate session token".to_string())
            )
        }
    }

    //  If all validation was ok, return None to proceed
    None
}

#[derive(Debug)]
struct TheHttpResponse {
    status_code: StatusCode,
    body: Option<String>,
}

impl TheHttpResponse {
    fn status_code(status_code: StatusCode) -> Self {
        TheHttpResponse {
            status_code,
            body: None,
        }
    }

    fn with_body(mut self, body: String) -> Self {
        self.body = Some(body);
        self
    }
}

impl fmt::Display for TheHttpResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.status_code, f)
    }
}

impl ResponseError for TheHttpResponse {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code)
            .body(self.body.clone().unwrap_or_default())
    }
}
