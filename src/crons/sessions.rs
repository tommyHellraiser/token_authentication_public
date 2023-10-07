use std::time::Duration;
use error_mapper::{map_to_new_error, TheResult};
use lazy_static::lazy_static;
use mysql_async::prelude::Queryable;
use tokio::sync::broadcast::Receiver;
use crate::api::StopMethod;
use crate::database::db_conn::get_conn;
use crate::{modules};
use crate::modules::users::user::User;
use crate::modules::users::users_sessions::{SessionData, SessionStatus};
use crate::modules::users::UsersSessions;

//  Cron to close expired sessions
pub(super) async fn close_expired_sessions(mut stopper: Receiver<StopMethod>) {

    lazy_static!{
        static ref DB_USAGE: tokio::sync::Mutex<bool> = tokio::sync::Mutex::new(false);
    }

    //  tokio select between the cron loop and the stopper reception
    let cron_loop = async {
        loop {

            //  Main cron loop
            //  Acquire mutex to work on DB
            *DB_USAGE.lock().await = true;

            //  Select all sessions from db and revoke the expired ones
            if let Err(e) = validate_db_sessions_status().await {
                println!("Error checking sessions status: {}", e);
            };

            //  Release mutex
            *DB_USAGE.lock().await = false;

            //  Sleep for 1 minute
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    };

    let stopper_reception = async {

        let _ = stopper.recv().await;
        //  Any stop message received here should kill the cron
        {
            while *DB_USAGE.lock().await {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    };

    //  Execute the two tasks and return when the first one is completed. The first one will always
    // be stopper_reception when a stop signal is received
    tokio::select!{
        _ = cron_loop => {},
        _ = stopper_reception => {}
    }
}

async fn validate_db_sessions_status() -> TheResult<()> {

    let conn = &mut get_conn().await?;

    let sessions = conn.query::<SessionData, _>(
        "SELECT users_ID, creation, expiry FROM users_sessions"
    ).await.map_err(|e| map_to_new_error!(e))?;

    for session in sessions {
        let user;
        if let Some(user_fetched) = User::select_by_id(session.get_user_id()).await? {
            user = user_fetched
        } else {
            //  If no user was found, need to make sure he's logged out and no active sessions are prsent in db 
            modules::users::users_sessions::delete_logins_session(session.get_user_id()).await?;
            UsersSessions::instance().delete_user_entry(session.get_user_id()).await;
            continue;
        };
        match session.get_session_status() {
            SessionStatus::Expired | SessionStatus::SessionError => {
                //  If session is expired or has any error, delete it from DB and logout from runtime
                modules::users::users_sessions::delete_logins_session(session.get_user_id()).await?;
                UsersSessions::instance().logout_user(&user).await;
            },
            SessionStatus::Active => {
                //  If active, do nothing, session is ok
                //  Ensure session is active in runtime
                UsersSessions::instance().login_user(&user).await;
            }
        }
    }

    Ok(())
}