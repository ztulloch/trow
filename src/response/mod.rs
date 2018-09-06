use hostname;
use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{Responder, Response};
use serde;

pub mod accepted_upload;
pub mod blob_reader;
pub mod empty;
pub mod errors;
pub mod html;
pub mod manifest_reader;
pub mod repo_catalog;
pub mod tag_list;
mod test_helper;
pub mod upload_info;
pub mod verified_manifest;

pub fn json_response<T: serde::Serialize>(
    req: &Request,
    var: &T,
) -> Result<Response<'static>, Status> {
    use rocket_contrib;
    rocket_contrib::Json(var).respond_to(req)
}

/// Gets the base URL e.g. <http://registry:8000> using the HOST value from the request header.
/// Falls back to hostname if it doesn't exist.
///
/// Move this.
fn get_base_url(req: &Request) -> String {
    let host = match req.headers().get("HOST").next() {
        None => {
            hostname::get_hostname().expect("Server has no name; cannot give clients my address")
        }
        Some(shost) => shost.to_string(),
    };

    //TODO: Dynamically figure out whether to use http or https
    format!("https://{}", host)
}
