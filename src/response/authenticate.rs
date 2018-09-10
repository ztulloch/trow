use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{Responder, Response};
/*
WWW-Authenticate: Basic

WWW-Authenticate: Basic realm="Access to the staging site", charset="UTF-8"
*/
#[derive(Debug, Serialize)]
pub struct AuthenticateHeader;

impl<'r> Responder<'r> for Empty {
    fn respond_to(self, _: &Request) -> Result<Response<'r>, Status> {
        Response::build()
            .status(Status::Unauthorized)
            .header("WWW-Autheniticate")
            .header("Basic")
            .header("realm=trow.test")
            .ok()
    }
}

impl Responder<'static> for String {
    fn respond_to(self, _: &Request) -> Result<Response<'static>, Status> {
        Response::build()
            .header(ContentType::Plain)
            .sized_body(Cursor::new(self))
            .ok()
    }
}

#[cfg(test)]
mod test {
    use response::empty::Empty;
    use rocket::http::Status;

    use response::test_helper::test_route;

    #[test]
    fn empty_ok() {
        let response = test_route(Empty);
        assert_eq!(response.status(), Status::Unauthorized);
    }
}
