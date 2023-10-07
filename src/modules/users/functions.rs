use actix_web::HttpRequest;
use error_mapper::{map_to_new_error, TheResult};
use mysql_async::prelude::Queryable;
use crate::auth;
use crate::database::db_conn::get_conn;
use crate::general::types::UsersIdType;
use crate::modules::users::user::User;
use crate::modules::users::{users_sessions, UsersSessions};

pub async fn create_default_super_user() -> TheResult<()> {

    let conn = &mut get_conn().await?;

    let first_id = conn.query_first::<Option<UsersIdType>, _>(
        "SELECT ID FROM users WHERE ID = 1"
    )
        .await
        .map_err(|e| map_to_new_error!(e))?
        .unwrap_or_default();

    if first_id.is_some() {
        return Ok(())
    }

    let mut user = User::create_super_user();
    let password = "asdfgqwert1234567890";
    let string_to_hash = user.build_string_to_hash(password);

    let hashed_pass = auth::crypt::generate_hash(
        string_to_hash.as_str()
    );

    user.set_hashed_pass(hashed_pass);

    user.insert().await?;

    Ok(())
}

pub fn get_username_from_request(request: HttpRequest) -> Option<String> {

    //  Attempt to get username from headers
    return match request.headers().get("username") {
        Some(username) => {
            return match username.to_str() {
                Ok(username) => Some(username.to_string()),
                Err(_) =>  None
            }
        },
        None => None
    }
}

pub fn get_session_token_from_request(request: HttpRequest) -> Option<String> {

    //  Attempt to get session token from request
    return match request.headers().get("token") {
        Some(token) => {
            return match token.to_str() {
                Ok(token) => Some(token.to_string()),
                Err(_) => None
            }
        },
        None => None
    }
}

pub async fn get_user_from_headers(username: Option<String>, token: Option<String>) -> TheResult<Option<User>> {

    let user;
    if username.is_none() || token.is_none() {
        Ok(None)
    } else {
        //  At this point, both username and session_token are validated to be a Some.
        let username = username.unwrap();
        let token = token.unwrap();
        // We fetch the user from database
        user = match User::select_by_username(username.as_str()).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                return Ok(None)
            },
            Err(e) => return Err(e)
        };

        //  Validating user is online
        if !UsersSessions::instance().is_user_logged_in(user.get_id()).await {
            return Ok(None)
        }

        //  Validating the session token
        match users_sessions::validate_session_token(&user, token.as_str()).await {
            Ok(true) => {
                Ok(Some(user))
            },
            Ok(false) => {
                Ok(None)
            },
            Err(e) => {
                Err(e)
            }
        }
    }
}
