
use error_mapper::{TheResult};
use openssl::ssl::SslAcceptorBuilder;
use tokio::sync::broadcast::{Receiver, Sender};
use crate::api::StopMethod;
use crate::config::environment::EnvironmentConfig;
use crate::config::shutdown::Shutdown;
use crate::modules::users::user::User;
use crate::modules::users::UsersSessions;

mod database;
mod config;
mod web_local;
mod modules;
mod api;
mod general;
mod auth;
mod crons;

#[tokio::main]
async fn main() -> TheResult<()> {

    //  TODO Check error logging from the error. Logging is weird, it originates in the error_mapper crate
    //  2023-10-01 T05:25:31.605599     @ C:\Users\Nacho\.cargo\registry\src\index.crates.io-6f17d22bba15001f\error_mapper-0.3.6\src\errors\the_error.rs 42|33 =>       NotFound: Username "super" not found

    //  TODO check for a disconnect method from db

    //  TODO s:
    //   -Superuser should be able to change other people's passwords
    //   -Validation to eliminate by cron any other superuser created manually in the database
    //   -Make logic to re-trigger the session manager cron if it fails for some reason
    //   -Hot reload for the config.json file. Easy implementation, just need to do it lol


    let (builder, (sender, receiver)) = setup_service().await?;

    setup_initial_env(receiver.resubscribe()).await?;

    if let Err(e) = api::start_api(
        (builder, (sender, receiver))
    ).await {
        panic!("Failed to start api services: {}", e);
    }

    Ok(())
}

pub async fn is_shutting_down() -> bool {
    Shutdown::instance().is_shutting_down().await
}

async fn setup_service() -> TheResult<(SslAcceptorBuilder, (Sender<StopMethod>, Receiver<StopMethod>))> {

    let builder = config::openssl::create_openssl_builder()?;

    let (sender, receiver) = tokio::sync::broadcast::channel::<StopMethod>(4);

    Shutdown::instance().set_shutdown_state_to_false().await;

    Ok((builder, (sender, receiver)))
}

async fn setup_initial_env(stopper: Receiver<StopMethod>) -> TheResult<()> {

    if EnvironmentConfig::instance().reset_db().await {
        if let Err(e) = database::reset_db().await {
            println!("There was an error resetting database: {}", e);
        };
    }

    modules::users::functions::create_default_super_user().await?;

    let users = User::select_all().await?;

    UsersSessions::instance().register_users_in_runtime(users.as_slice()).await?;

    //  Start session cron(s)
    tokio::spawn(crons::run_crons(stopper));

    Ok(())
}