use actix_web::{get, HttpRequest, HttpResponse, put, web};
use actix_web::http::StatusCode;
use chrono::{Local};
use crate::{StopMethod};
use crate::api::AppData;
use crate::config::shutdown::Shutdown;
use crate::general::http_req_res::json_response;
use crate::modules::users::functions;
use crate::modules::users::user::Level;

pub fn alive_service(cfg: &mut web::ServiceConfig) {
    cfg.service(alive);
}

pub fn internal(cfg: &mut web::ServiceConfig) {
    cfg.service(alive)
        .service(stop)
        .service(stop_now);
}

/// ## Endpoint alive
/// GET {UTAUrl}:{UTAPort}/api/alive (public)
/// GET {UTAUrl}:{UTAPort}/api/internal/alive (public)
///
/// ### Description
/// Alive endpoint. Responds with alive data when service is running
///
/// #### Information
/// - Available in public and private modes for testing purposes
/// - Private mode will require user to to be authenticated
/// - Any user level will be able to request the alive service in private mode
#[get("alive")]
async fn alive() -> HttpResponse {

    if Shutdown::instance().is_shutting_down().await {
        return HttpResponse::Ok().json("Service is shutting down");
    }

    HttpResponse::Ok().json(
        format!(
            "Service is alive at: {} {}",
            Local::now().date_naive().format("%Y-%m-%d"),
            Local::now().time().format("%H:%M:%S")
        )
    )
}
/// ## Endpoint stop
/// GET {UTAUrl}:{UTAPort}/api/stop (public)
/// GET {UTAUrl}:{UTAPort}/api/internal/stop (public)
///
/// ### Description
/// Stop endpoint that kills the Http server gracefully
///
/// #### Information
/// User needs to be authenticated to perform this action with level High or Super
#[put("stop")]
async fn stop(request: HttpRequest, data: web::Data<AppData>) -> HttpResponse {

    //  Fetch username and token from headers
    let username = functions::get_username_from_request(request.clone());
    let session_token = functions::get_session_token_from_request(request.clone());

    let user = match functions::get_user_from_headers(username, session_token).await {
        Ok(Some(user)) => user,
        Ok(None) => return json_response(StatusCode::UNAUTHORIZED, "Invalid username or session token".to_string()),
        Err(_) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error restoring user".to_string())
    };

    if user.get_level() < &Level::High {
        return json_response(
            StatusCode::FORBIDDEN,
            "User lacks the privileges to perform this operation".to_string()
        )
    }

    if let Err(e) = data.sender.send(StopMethod::Graceful) {
        return HttpResponse::InternalServerError().json(format!("Failed to send stop signal: {}", e));
    };

    HttpResponse::Ok().json("Service is stopping")
}

/// ## Endpoint stop now
/// GET {UTAUrl}:{UTAPort}/api/stop_now (public)
/// GET {UTAUrl}:{UTAPort}/api/internal/stop_now (public)
///
/// ### Description
/// Stop endpoint that kills the Http server immediately without waiting for other processes to end
///
/// #### Information
/// User needs to be authenticated to perform this action with level Super
#[put("stop_now")]
async fn stop_now(request: HttpRequest, data: web::Data<AppData>) -> HttpResponse {

    //  Fetch username and token from headers
    let username = functions::get_username_from_request(request.clone());
    let session_token = functions::get_session_token_from_request(request.clone());

    let user = match functions::get_user_from_headers(username, session_token).await {
        Ok(Some(user)) => user,
        Ok(None) => return json_response(StatusCode::UNAUTHORIZED, "Invalid username or session token".to_string()),
        Err(_) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, "Error restoring user".to_string())
    };

    if user.get_level() < &Level::Super {
        return json_response(
            StatusCode::FORBIDDEN,
            "User lacks the privileges to perform this operation".to_string()
        )
    }

    if let Err(e) = data.sender.send(StopMethod::Immediate) {
        return HttpResponse::InternalServerError().json(format!("Failed to send stop signal: {}", e));
    };

    HttpResponse::Ok().json("Service is stopping al tiro")
}
