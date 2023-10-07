use std::ops::Add;
use chrono::NaiveDateTime;
use error_mapper::{map_to_new_error, SystemErrorCodes, TheError, TheResult};
use mysql_async::prelude::{FromRow, Queryable};
use mysql_async::{FromRowError};
use crate::{database, row_to_data, row_to_naive_datetime};
use crate::database::db_conn::get_conn;
use crate::general::types::UsersIdType;
use crate::modules::users::user::User;
use crate::modules::users::UsersSessions;

#[derive(Default, Debug, Clone, PartialEq, Copy)]
#[allow(dead_code)]
pub enum SessionStatus {
    Active,
    #[default]
    Expired,
    SessionError
}

#[derive(Clone, Debug, Copy)]
pub struct SessionData {
    users_id: UsersIdType,
    session_status: SessionStatus
}

pub(super) async fn check_user_active_session(user_id: &UsersIdType) -> TheResult<SessionStatus> {

    match select_expiry_from_user_session(user_id).await {
        Ok(expiry) => {
            if expiry < chrono::Utc::now().naive_utc() {
                //  At this point, if the session is expired, 
                return Ok(SessionStatus::Expired)
            }
            Ok(SessionStatus::Active)
        },
        Err(_) => {
            Ok(SessionStatus::SessionError)
        }
    }
}

pub(super) async fn extend_user_session(user: &User) -> TheResult<SessionStatus> {
    //  If a session was found, then the user is logged in. Make sure the static sessions data is updated
    UsersSessions::instance().login_user(user).await;
    if update_login_session(user.get_id()).await.is_err() {
        //  If the session couldn't be updated, delete the session from database
        if let Err(e) = delete_logins_session(user.get_id()).await {
            //  TODO remove when logger is implemented
            println!("Error deleting user {} session: {}", user.get_id(), e);
            return Ok(SessionStatus::SessionError)
        };
    };

    Ok(SessionStatus::Active)
}

pub(super) async fn terminate_user_session(user: &User) -> TheResult<()> {

    //  Delete session data from database
    delete_logins_session(user.get_id()).await?;

    //  Close session from session data
    UsersSessions::instance().logout_user(user).await;

    Ok(())
}

pub(super) async fn activate_user_session(user: &User, token: &String) -> TheResult<()> {

    insert_login_session(user.get_id(), token).await?;

    UsersSessions::instance().login_user(user).await;

    Ok(())
}

pub(super) async fn select_expiry_from_user_session(
    user_id: &UsersIdType
) -> TheResult<NaiveDateTime> {

    let conn = &mut get_conn().await?;

    //  If no session is active, creation won't be found, and if it is found, expiry value will tell
    // whether the session is active or not
    let expiry = conn.query_first::<Option<String>,_>(
        format!(
            "SELECT expiry FROM users_sessions \
            WHERE users_ID = {}",
            user_id
        )
    ).await.map_err(|e| map_to_new_error!(e))?;

    //  Try expiry into NaiveDateTime
    let expiry = if let Some(Some(string)) = expiry {
        NaiveDateTime::parse_from_str(string.as_str(), database::DATETIME_FORMAT)
            .map_err(|e| map_to_new_error!(e))?
    } else {
        //  If it's default, the session's expiry datetime will return as EPOCH time, so it's expired
        return Ok(NaiveDateTime::default())
    };

    Ok(expiry)
}

pub(super) async fn insert_login_session(user_id: &UsersIdType, token: &String) -> TheResult<()> {

    let conn = &mut get_conn().await?;

    let now = chrono::Utc::now();

    let expiry = now.add(chrono::Duration::minutes(30))
        .format(database::DATETIME_FORMAT)
        .to_string();

    let creation = now.format(database::DATETIME_FORMAT).to_string();

    conn.query_drop(
        format!(
            "INSERT INTO users_sessions (users_ID, token, creation, expiry) \
            VALUES ({}, '{}', '{}', '{}')",
            user_id,
            token,
            creation,
            expiry
        )
    ).await.map_err(|e| map_to_new_error!(e))?;

    Ok(())
}

pub(super) async fn update_login_session(user_id: &UsersIdType) -> TheResult<()> {

    let conn = &mut get_conn().await?;

    let now = chrono::Utc::now();

    let expiry = now.add(chrono::Duration::minutes(30))
        .format(database::DATETIME_FORMAT)
        .to_string();

    let creation = now.format(database::DATETIME_FORMAT).to_string();

    conn.query_drop(
        format!(
            "UPDATE users_sessions SET creation = '{}', expiry = '{}' \
            WHERE users_ID = {}",
            creation,
            expiry,
            user_id
        )
    ).await.map_err(|e| map_to_new_error!(e))?;

    Ok(())
}

pub async fn delete_logins_session(user_id: &UsersIdType) -> TheResult<()> {

    let conn = &mut get_conn().await?;

    conn.query_drop(
        format!(
            "DELETE FROM users_sessions WHERE users_ID = {}",
            user_id
        )
    ).await.map_err(|e| map_to_new_error!(e))?;

    Ok(())
}

pub async fn validate_session_token(user: &User, user_token: &str) -> TheResult<bool> {

    let conn = &mut get_conn().await?;

    let token = conn.query_first::<Option<String>, _>(
        format!(
            "SELECT token FROM users_sessions WHERE users_ID = {}",
            user.get_id()
        )
    ).await.map_err(|e| map_to_new_error!(e))?;

    if let Some(Some(db_token)) = token {
        if db_token.as_str() == user_token {
            return Ok(true)
        }
    }

    Ok(false)
}

pub(super) async fn fetch_session_token(user_id: &UsersIdType) -> TheResult<String> {

    let conn = &mut get_conn().await?;

    let token = conn.query_first::<Option<String>, _>(
        format!(
            "SELECT token FROM users_sessions WHERE users_ID = {}",
            user_id
        )
    ).await.map_err(|e| map_to_new_error!(e))?;

    if let Some(Some(token)) = token {
        return Ok(token)
    }

    Err(
        map_to_new_error!(
            TheError::new(
                SystemErrorCodes::NotFound,
                format!(
                    "User {} session not found",
                    user_id
                )
            )
        )
    )
}

impl SessionData {
    pub async fn get_all_user_sessions() -> TheResult<Vec<Self>> {

        let conn = &mut get_conn().await?;

        let sessions = conn.query::<Self, _>(
            "SELECT users_ID, creation, expiry FROM users_sessions"
        ).await.map_err(|e| map_to_new_error!(e))?;

        Ok(sessions)
    }

    pub fn get_user_id(&self) -> &UsersIdType {
        &self.users_id
    }

    pub fn get_session_status(&self) -> &SessionStatus {
        &self.session_status
    }
}

impl FromRow for SessionData {
    fn from_row(row: mysql_async::Row) -> Self {
        let creation = row_to_naive_datetime!(row, "creation", "users_sessions");
        let expiry = row_to_naive_datetime!(row, "expiry", "users_sessions");

        Self {
            users_id: row_to_data!(row, "users_ID", "users_sessions", UsersIdType),
            session_status: {
                if creation > chrono::Utc::now().naive_utc() {
                    SessionStatus::SessionError
                } else if expiry < chrono::Utc::now().naive_utc() {
                    SessionStatus::Expired
                } else {
                    SessionStatus::Active
                }
            }
        }
    }

    fn from_row_opt(_: mysql_async::Row) -> Result<Self, FromRowError> where Self: Sized {
        unimplemented!()
    }
}