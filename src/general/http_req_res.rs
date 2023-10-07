use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use error_mapper::{map_to_new_error, TheResult};
use serde::Serialize;

pub fn json_response(status_code: StatusCode, msg: String) -> HttpResponse {
    HttpResponse::build(status_code).content_type("application/json").body(msg.clone())
}

pub fn plain_text_response(status_code: StatusCode, text: String) -> HttpResponse {
    HttpResponse::build(status_code).body(text)
}

pub fn serialize_into_json<T: Serialize>(struct_to_serialize: &T) -> TheResult<String> {
    match serde_json::to_string(struct_to_serialize) {
        Ok(serialized_struct) => Ok(serialized_struct),
        Err(e) => Err(map_to_new_error!(e))
    }
}
