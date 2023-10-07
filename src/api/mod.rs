
use actix_web::{App, HttpServer, web};
use error_mapper::{map_to_new_error, TheResult};
use openssl::ssl::SslAcceptorBuilder;
use tokio::sync::broadcast::{Receiver, Sender};
use crate::{config, modules};
use crate::api::authentication::UserAuthentication;
use crate::config::environment::EnvironmentConfig;
use crate::modules::users::user::Level;

pub mod services;
pub mod authentication;

#[derive(Debug, Clone)]
pub enum StopMethod {
    Graceful,
    Immediate
}

#[derive(Debug, Clone)]
pub struct AppData {
    pub(crate) sender: Sender<StopMethod>
}

pub async fn start_api(
    (builder, (sender, receiver)): (SslAcceptorBuilder, (Sender<StopMethod>, Receiver<StopMethod>))
) -> TheResult<()> {

    let service_api_bind = format!(
        "{}:{}",
        EnvironmentConfig::instance().get_service_url().await,
        EnvironmentConfig::instance().get_service_port().await
    );

    let server = HttpServer::new(move || {
        let sender_api = sender.clone();

        App::new()
            .service(
                web::scope("api")
                    .service(
                    web::scope("public").configure(services::api::alive_service)
                    ).service(
                    web::scope("internal")
                        .configure(services::api::internal)
                        .app_data(web::Data::new(AppData { sender: sender_api.clone() }))
                        .wrap(UserAuthentication::new(Level::High))
                )
            )
            .service(
                web::scope("users")
                    .configure(services::users::services)
            )
            .service(
                web::scope("internal")
                    .configure(services::internal::services)
                    .wrap(UserAuthentication::new(Level::High))
            )
    })
        .workers(32)
        //  Kills main thread if it fails to open the Http server
        .bind_openssl(service_api_bind, builder)
        .map_err(|e| map_to_new_error!(e))?
        .run();

    tokio::spawn(config::shutdown::stop_server(server.handle(), receiver.resubscribe()));

    server.await.map_err(|e| map_to_new_error!(e))?; // Blocks main thread until server is stopped

    Ok(())
}

/*
API SCHEMA:
    login
    logout
    create account
    delete account
    change password
    alive
    stop
    stop now

    api
        alive
        internal
            alive internal
            stop
            stop now

    users
        login
        logout
        create account (up to level 1)
        manage
            change password
            delete account

    internal
        create account (up to level 3. Only 1 level 4 account)
        delete account (up to level 3. Can delete an account of up to 1 level below)
 */