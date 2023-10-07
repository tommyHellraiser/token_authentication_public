use actix_web::web;

use crate::modules;
use crate::modules::users::user::Level;

pub fn services(cfg: &mut web::ServiceConfig) {
    cfg.service(modules::users::services::user_login)
        .service(modules::users::services::user_logout)
        .service(modules::users::services::create_user)
        .service(
            web::scope("manage")
                .service(modules::users::services::change_password)
                .service(modules::users::services::delete_user)
                .service(modules::users::services::check_password)
                .wrap(crate::api::UserAuthentication::new(Level::Low))
        );
}
