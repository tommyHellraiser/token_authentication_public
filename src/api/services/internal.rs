use actix_web::web;
use crate::modules;

pub fn services(cfg: &mut web::ServiceConfig) {
    cfg.service(modules::users::services::create_user)
        .service(modules::users::services::delete_user_internal)
        .service(modules::users::services::undo_delete_user)
        .service(modules::users::services::change_user_level);
        
}