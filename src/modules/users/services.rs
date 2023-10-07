
use actix_web::{get, HttpRequest, HttpResponse, post, put, web};
use actix_web::http::StatusCode;
use serde::{Deserialize, Serialize};
use crate::{auth, general};
use crate::general::http_req_res::{json_response, plain_text_response};
use crate::general::types::UsersIdType;
use crate::modules::users::{functions, user, users_sessions};
use crate::modules::users::user::{Level, User};
use crate::modules::users::users_sessions::{SessionStatus};

#[derive(Deserialize, Debug, Clone)]
struct PostUser {
    username: String,
    password: String,
    email: String,
    level: Option<u8>
}

#[derive(Serialize)]
struct UserCreated {
    user_id: UsersIdType,
    session_token: String
}

#[derive(Deserialize, Debug)]
struct UserLoginData {
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String
}

#[derive(Deserialize, Debug, Clone)]
struct UserDelete {
    user_id: Option<UsersIdType>,
    username: Option<String>
}

#[derive(Deserialize, Debug, Clone)]
struct ValidatePassword {
    password: String
}

#[derive(Deserialize, Debug, Clone)]
struct ChangePassword {
    old_password: String,
    new_password: String
}

#[derive(Deserialize, Debug, Clone)]
struct UndoDeleteUser {
    user_id: Option<UsersIdType>,
    username: Option<String>
}

#[derive(Deserialize, Debug, Clone)]
struct ChangeUserLevel {
    user_id: Option<UsersIdType>,
    username: Option<String>,
    level: u8
}


/// ##  Endpoint login
/// POST {UTAUrl}:{UTAPort}/users/login
///
/// #### Required Body fields
/// - username: ans-20 max
/// - password: ans-30 max
#[post("/login")]
async fn user_login(body: web::Json<UserLoginData>) -> HttpResponse {

    let user_login_data = body.into_inner();
    let (username, password) = (
        user_login_data.username.as_str(), user_login_data.password.as_str()
    );

    //  Get user data from db
    let user = match User::select_by_username(username).await {
        Ok(Some(user)) => user,
        Ok(None) => return json_response(StatusCode::BAD_REQUEST, "Invalid username or password".to_string()),
        Err(_) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error logging in".to_string())
    };

    //  Check password and execute login
    //  Password is the value received from the request. self.hashed_pass is the value fetched from db
    if !user.validate_hashed_password(password) {
        return json_response(StatusCode::UNAUTHORIZED, "Invalid username or password".to_string())
    };

    //  Check if user has an active session
    match users_sessions::check_user_active_session(user.get_id()).await {
        Ok(SessionStatus::Active) => {
            return match users_sessions::extend_user_session(&user).await {
                Ok(_) => {
                    //  Fetch existing token from db
                    let token = match users_sessions::fetch_session_token(user.get_id()).await {
                        Ok(token) => token,
                        Err(_) => {
                            return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error logging in".to_string())
                        }
                    };

                    plain_text_response(StatusCode::OK, token)
                },
                Err(_) => json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error logging in".to_string())
            }
        },
        //  If session is expired, the user gets logged in next
        Ok(SessionStatus::Expired) => {},
        Ok(SessionStatus::SessionError) | Err(_) => {
            return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error checking user session. Session error".to_string())
        }
    }

    //  Generate new token to login user
    let token = match auth::crypt::generate_session_token() {
        Ok(token) => token,
        Err(_) => {
            return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error processing login".to_string())
        }
    };

    //  If user has no active session, execute log in
    match users_sessions::activate_user_session(&user, &token).await {
        Ok(_) => {
            plain_text_response(StatusCode::OK, token)
        },
        Err(_) => {
            json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error logging in".to_string())
        }
    }
}

/// ##  Endpoint logout
/// POST {UTAUrl}:{UTAPort}/users/logout
///
/// #### Required Headers
/// - username: ans-20 max
/// - token: ans-50 max
#[post("/logout")]
async fn user_logout(request: HttpRequest) -> HttpResponse {

    let username = functions::get_username_from_request(request.clone());
    let session_token = functions::get_session_token_from_request(request.clone());

    let user = match functions::get_user_from_headers(username, session_token.clone()).await {
        Ok(Some(user)) => user,
        Ok(None) => return json_response(StatusCode::UNAUTHORIZED, "Invalid username or session token".to_string()),
        Err(_) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error changing password".to_string())
    };

    //  Check if user has an active session
    match users_sessions::check_user_active_session(user.get_id()).await {
        Ok(SessionStatus::Active) => {
            //  If the session is active, proceed to logout
        },
        Ok(SessionStatus::Expired) => {
            return json_response(StatusCode::UNAUTHORIZED, "Session expired".to_string())
        },
        Ok(SessionStatus::SessionError) | Err(_) => {
            return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error checking user session".to_string())
        }
    }

    match users_sessions::terminate_user_session(&user).await {
        Ok(_) => {
            json_response(StatusCode::OK, "Successfully logged out".to_string())
        },
        Err(_) => {
            json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error logging out".to_string())
        }
    }
}

/// ##  Endpoint create user
/// POST {UTAUrl}:{UTAPort}/users/create_user (public)
/// POST {UTAUrl}:{UTAPort}/internal/create_user (private)
///
/// #### Required Body
/// - username: ans-20 max string
/// - password: ans-30 max string
/// - email: ans-50 max string
#[post("/create_user")]
async fn create_user(request: HttpRequest, body: web::Json<PostUser>) -> HttpResponse {

    //  First of all check if username is available, to avoid unnecessary computations
    match user::username_available(body.username.as_str()).await {
        Ok(true) => {},
        Ok(false) => {
            return json_response(StatusCode::BAD_REQUEST, "Username not available".to_string())
        },
        Err(_) => {
            return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error creating user".to_string())
        }
    }

    //  Attempt to get username from headers
    let username = functions::get_username_from_request(request.clone());

    //  Attempt to get session token from request
    let session_token = functions::get_session_token_from_request(request.clone());

    //  Check availability of user to create
    match User::select_by_username(body.username.as_str()).await {
        Ok(Some(_)) => return json_response(StatusCode::BAD_REQUEST, "Username not available".to_string()),
        Ok(None) => {},
        Err(_) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error creating user".to_string())
    }

    //  Validate password
    let errors = User::validate_password(&body.password);
    if !errors.is_empty() {
        return json_response(StatusCode::BAD_REQUEST, errors.join("\n"));
    }

    //  If username and session token could be retrieved from headers, validate level to create an
    // account one level below that one
    let mut account_level = Level::Low;
    if username.is_some() && session_token.is_some() {
        //  At this point username and session_token are validated to be a Some
        let user = match User::select_by_username(username.unwrap().as_str()).await {
            Ok(Some(user)) => user,
            Ok(None) => return json_response(StatusCode::BAD_REQUEST, "Invalid username or session token".to_string()),
            Err(_) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error fetching user data".to_string())
        };

        match users_sessions::validate_session_token(&user, session_token.unwrap().as_str()).await {
            Ok(true) => {
                //  Attempts to fetch the Level sent in the request body
                if let Some(level_u8) = body.level {
                    let level = level_u8.into();
                    if level > user.get_level().one_level_below() {
                        return json_response(
                            StatusCode::BAD_REQUEST,
                            "User level must be at least one level below the requesting account's".to_string()
                        )
                    } else {
                        account_level = level;
                    }
                } else {
                    //  If not possible to fetch, it'll create a user with one level below the requesting user
                    account_level = user.get_level().one_level_below();
                }
            },
            _ => {
                return json_response(StatusCode::UNAUTHORIZED, "Invalid username or session token".to_string())
            }
        };
    }

    //  Create a user account with the data from the body, and the level fetched above
    let (user_id, token) = match User::create_user(
        &body.username,
        &body.password,
        &body.email,
        &account_level
    ).await {
        Ok((user, token)) => (user, token),
        Err(_) => {
            return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error creating user".to_string())
        }
    };

    let user_created = UserCreated {
        user_id,
        session_token: token
    };

    let body = match general::http_req_res::serialize_into_json(&user_created) {
        Ok(body) => body,
        Err(_) => {
            return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error creating user".to_string())
        }
    };

    json_response(StatusCode::CREATED, body)
}

/// ##  Endpoint change password
/// PUT {UTAUrl}:{UTAPort}/users/manage/change_password
///
/// #### Required Body
/// - old_password: ans-50 max String
/// - new_password: ans-50 max String
#[put("/change_password")]
async fn change_password(request: HttpRequest, body: web::Json<ChangePassword>) -> HttpResponse {

    let username = functions::get_username_from_request(request.clone());
    let session_token = functions::get_session_token_from_request(request.clone());

    let user = match functions::get_user_from_headers(username, session_token).await {
        Ok(Some(user)) => user,
        Ok(None) => return json_response(StatusCode::UNAUTHORIZED, "Invalid username or session token".to_string()),
        Err(_) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error changing password".to_string())
    };

    //  Validating old password
    if user.validate_hashed_password(body.old_password.as_str()) {
        //  Validate password
        User::validate_password(&body.new_password);

        //  Changing password
        match user.change_password(body.new_password.as_str()).await {
            Ok(_) => HttpResponse::Ok().finish(),
            Err(_) => json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error changing password".to_string())
        }
    } else {
        json_response(StatusCode::BAD_REQUEST, "Old password is incorrect".to_string())
    }
}

/// ## Endpoint check password
/// GET {UTAUrl}:{UTAPort}/users/manage/check_password (public)
///
/// #### Required Body
/// - password: ans-50 max String
///
/// #### Description
/// Endpoint used to simply validate if a password matches with the user's stored password.
/// Usage examples are when a website asks to re-enter the password to confirm it's the user and not
/// someone who got a hold to the user's account
///
/// ### Warning
/// User needs to be logged in for this endpoint to work, otherwise it will return an unauthorized
///
/// #### Response:
/// - 201 if Ok. No need for extra content
/// - 400 if invalid password. An empty bad request http response message is enough for this case
#[get("/check_password")]
async fn check_password(request: HttpRequest, body: web::Json<ValidatePassword>) -> HttpResponse {

    //  Fetch username and token from headers
    let username = functions::get_username_from_request(request.clone());
    let session_token = functions::get_session_token_from_request(request.clone());

    let user = match functions::get_user_from_headers(username, session_token).await {
        Ok(Some(user)) => user,
        Ok(None) => return json_response(StatusCode::UNAUTHORIZED, "Invalid username or session token".to_string()),
        Err(_) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error validating password".to_string())
    };

    //  Validating password
    if user.validate_hashed_password(body.password.as_str()) {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::BadRequest().finish()
    }
}

/// ##  Endpoint delete user
/// PUT {UTAUrl}:{UTAPort}/users/manage/delete_user (public)
///
/// #### Required Headers
/// - username (required): ans-20 max string
/// - token (required): session token provided by the app in login
#[put("/delete_user")]
async fn delete_user(request: HttpRequest) -> HttpResponse {

    //  Fetch username and token from headers
    let username = functions::get_username_from_request(request.clone());
    let session_token = functions::get_session_token_from_request(request.clone());

    let user = match functions::get_user_from_headers(username, session_token).await {
        Ok(Some(user)) => user,
        Ok(None) => return json_response(StatusCode::UNAUTHORIZED, "Invalid username or session token".to_string()),
        Err(_) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error deleting user".to_string())
    };

    //  Deleting account (own account in this endpoint, user does not have permission to delete another user's account)
    match user.delete_account().await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(_) => json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error deleting user".to_string())
    }
}

/// ##  Endpoint delete user internal
/// PUT {UTAUrl}:{UTAPort}/internal/delete_user (private)
///
/// #### Required Body
/// - username: ans-20 max string
/// - password: ans-25 max string
///
/// ### Description
/// Deletes an account sent in the body of the request. This endpoint is only accessible to super
/// and admins (high). The account to delete should be the one included in the request body
#[put("/delete_user")]
async fn delete_user_internal(request: HttpRequest, body: web::Json<UserDelete>) -> HttpResponse {

    //  Fetch username and token from headers
    let username = functions::get_username_from_request(request.clone());
    let session_token = functions::get_session_token_from_request(request.clone());

    let user = match functions::get_user_from_headers(username, session_token).await {
        Ok(Some(user)) => user,
        Ok(None) => return json_response(StatusCode::UNAUTHORIZED, "Invalid username or session token".to_string()),
        Err(_) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error deleting user".to_string())
    };

    //  Fetching user to be deleted
    let user_to_delete;
    if let Some(user_id) = body.user_id {
        user_to_delete = match User::select_by_id(&user_id).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                return json_response(StatusCode::BAD_REQUEST, "Invalid user id".to_string())
            },
            Err(_) => {
                return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error deleting user".to_string())
            }
        }
    } else if let Some(username) = body.username.clone() {
        user_to_delete = match User::select_by_username(username.as_str()).await {
            Ok(Some(user)) => user,
            Ok(None) => return json_response(StatusCode::BAD_REQUEST, "Invalid username or password".to_string()),
            Err(_) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error logging in".to_string())
        };
    } else {
        return json_response(StatusCode::BAD_REQUEST, "Invalid user id and username".to_string())
    }

    //  Checking that level of user to delete is one lower than user that is deleting
    if user.get_level().one_level_below() <= *user_to_delete.get_level() {
        return json_response(StatusCode::UNAUTHORIZED, "User does not have permission to delete this account".to_string())
    }


    match user_to_delete.delete_account().await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(_) => json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error deleting user".to_string())
    }
}

/// ##  Endpoint undo delete account
/// PUT {UTAUrl}:{UTAPort}/internal/undo_delete_account (private)
///
/// #### Required Body
/// One of the optional parameters must be present in request body
/// - user_id (optional): optional u32
/// - username (optional): optional ans-20 max string
#[put("/undo_delete_user")]
async fn undo_delete_user(request: HttpRequest, body: web::Json<UndoDeleteUser>) -> HttpResponse {

    //  Fetch username and token from headers
    let username = functions::get_username_from_request(request.clone());
    let session_token = functions::get_session_token_from_request(request.clone());

    let user = match functions::get_user_from_headers(username, session_token).await {
        Ok(Some(user)) => user,
        Ok(None) => return json_response(StatusCode::UNAUTHORIZED, "Invalid username or session token".to_string()),
        Err(_) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error restoring user".to_string())
    };

    //  Validate the user has privileges to restore an account
    if user.get_level() < &Level::High {
        return json_response(
            StatusCode::UNAUTHORIZED,
            "User does not have permission to restore this account".to_string()
        )
    }

    match User::restore_user(body.user_id, body.username.clone()).await {
        Ok(Some(true)) => json_response(StatusCode::OK, "User restored".to_string()),
        Ok(Some(false)) => {
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "User account not restored".to_string()
            )
        },
        Ok(None) => {
            json_response(
                StatusCode::BAD_REQUEST,
                "User id and username not received in request".to_string()
            )
        },
        Err(_) => {
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error restoring user account".to_string()
            )
        }
    }
}

/// ##  Endpoint change user level
/// PUT {UTAUrl}:{UTAPort}/internal/change_user_level (private)
///
/// #### Required Body
/// One of the optional parameters must be present in the request body
/// - user_id (optional): optional u32
/// - username (optional): optional ans-20 max string
/// - level (required): from 0 to 3 u8
#[put("/change_user_level")]
async fn change_user_level(request: HttpRequest, body: web::Json<ChangeUserLevel>) -> HttpResponse {

    let target_user = body.into_inner();

    //  Fetch username and token from headers
    let username = functions::get_username_from_request(request.clone());
    let session_token = functions::get_session_token_from_request(request.clone());

    let user = match functions::get_user_from_headers(username, session_token).await {
        Ok(Some(user)) => user,
        Ok(None) => return json_response(StatusCode::UNAUTHORIZED, "Invalid username or session token".to_string()),
        Err(_) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error restoring user".to_string())
    };

    //  Validate user has privileges to change an account's level
    if user.get_level() < &Level::High {
        return json_response(
            StatusCode::FORBIDDEN,
            "User lacks the privileges to perform the required operation".to_string()
        )
    }

    let target_level = target_user.level.into();
    if user.get_level().one_level_below() < target_level {
        return json_response(
            StatusCode::FORBIDDEN,
            "User lacks privileges to perform required operation".to_string()
        )
    }

    let user_id = if let Some(user) = target_user.user_id {
        match User::select_by_id(&user).await {
            Ok(Some(user)) => {
                *user.get_id()
            }
            Ok(None) => {
                return json_response(
                    StatusCode::BAD_REQUEST,
                    "Invalid user id".to_string()
                )
            },
            Err(_) => {
                return json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error changing user level".to_string()
                )
            }
        }
    } else if let Some(username) = target_user.username {
        match User::select_by_username(username.as_str()).await {
            Ok(Some(user)) => {
                *user.get_id()
            },
            Ok(None) => {
                return json_response(
                    StatusCode::BAD_REQUEST,
                    "Invalid username".to_string()
                )
            },
            Err(_) => {
                return json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error changing user level".to_string()
                )
            }
        }
    } else {
        return json_response(
            StatusCode::BAD_REQUEST,
            "Invalid user id and username".to_string()
        )
    };

    //  Change the level
    match User::change_user_level(&user_id, &target_level).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => {
            json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error changing user level".to_string()
            )
        }
    }
}
