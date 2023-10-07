use std::collections::HashMap;
use error_mapper::TheResult;
use lazy_static::lazy_static;
use tokio::sync::RwLock;
use crate::general::types::UsersIdType;
use crate::modules::users::user::User;
use crate::modules::users::users_sessions::SessionStatus;

pub mod services;
pub mod functions;
pub mod queries;
pub mod user;
pub mod users_sessions;


lazy_static!{
    /// Users sessions and users serve the purpose of being an abstraction layer between
    /// authentication from API and database
    ///
    /// If we're not careful with abstraction, users might attempt to execute SQL
    /// injections and get unexpected data
    pub static ref USERS_SESSIONS: UsersSessions = UsersSessions::new();
}


#[derive(Debug)]
pub struct UsersSessions {
    inner: RwLock<UsersSessionsInner>
}

#[derive(Debug)]
struct UsersSessionsInner {
    //  User ID, is logged in
    sessions: HashMap<UsersIdType, UserSessionData>
}

#[derive(PartialEq, Debug, Clone, Default)]
struct UserSessionData {
    username: String,
    email: String,
    session_status: SessionStatus
}

impl UsersSessions {
    fn new() -> Self {
        Self {
            inner: RwLock::new(UsersSessionsInner {
                sessions: HashMap::new()
            })
        }
    }

    pub fn instance() -> &'static Self {
        &USERS_SESSIONS
    }

    pub async fn is_user_logged_in(&self, user_id: &UsersIdType) -> bool {
        if let Some(session_data) = self.inner.read().await.sessions.get(user_id) {
            if session_data.session_status == SessionStatus::Active {
                return true
            }
        }
        false
    }

    pub async fn login_user(&self, user: &User) {
        //  Looks for a key. If not found, it inserts the login value, otherwise, it sets the logged in status as true
        self.inner.write().await.sessions
            .entry(*user.get_id())
            .and_modify(|session_data| {session_data.session_status = SessionStatus::Active})
            .or_insert_with(|| {
                //  This point will be reached when a new user is created and is logged in for the first time
                UserSessionData{
                    username: user.get_username().to_string(),
                    email: user.get_email().to_string(),
                    session_status: SessionStatus::Active
                }
            });
    }

    pub async fn logout_user(&self, user: &User) {
        self.inner.write().await.sessions.entry(*user.get_id())
            .and_modify(|session_data| {session_data.session_status = SessionStatus::Expired})
            .or_insert_with(|| {
                //  This point will be reached when a new user is created and is logged in for the first time
                UserSessionData{
                    username: user.get_username().to_string(),
                    email: user.get_email().to_string(),
                    session_status: SessionStatus::Expired
                }
            });
    }
    
    pub async fn delete_user_entry(&self, user_id: &UsersIdType) {
        //  If the user exists, it'll get deleted. If not, there was no user to start with. No need to check
        self.inner.write().await.sessions.remove(user_id);
    }

    pub async fn register_users_in_runtime(&self, users: &[User]) -> TheResult<()> {

        //  Registering users in runtime session data
        for user in users {
            self.inner.write().await.sessions.insert(
                *user.get_id(),
                UserSessionData {
                    username: user.get_username().to_string(),
                    email: user.get_email().to_string(),
                    session_status: SessionStatus::Expired
                }
            );
        }

        //  Updating the session status reading user sessions from database
        let sessions = users_sessions::SessionData::get_all_user_sessions().await?;

        for session in sessions {
            self.inner.write().await.sessions.entry(*session.get_user_id())
                .and_modify(|session_data| {
                    session_data.session_status = *session.get_session_status()
                })
                .or_insert_with(||{
                    //  If we get here, it means that there's a user session registered in database
                    // but the origin user doesn't exist. Big problem
                    //  TODO remove when logger is implemented
                    println!("User {} not found in runtime sessions", session.get_user_id());
                    UserSessionData::default()
                });
        }

        Ok(())
    }
}
