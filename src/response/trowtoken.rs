use frank_jwt::{decode, encode, Algorithm};
use rocket::http::ContentType;
use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};
use rocket::response::{Responder, Response};
use rocket::Outcome;
use serde_json::json;
use std::io::Cursor;
use std::ops::Add;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const AUTHORISATION_SECRET: &str = "Bob Marley Rastafaria";
const TOKEN_DURATION: u64 = 3600;

pub struct ValidBasicToken {
    user: String,
}

impl<'a, 'r> FromRequest<'a, 'r> for ValidBasicToken {
    type Error = ();
    fn from_request(req: &'a Request<'r>) -> request::Outcome<ValidBasicToken, ()> {
       
        //As Authorization is a standard header, we should be able to use standard code here
        //But Rocket doesn't seem to support it, despite exporting Hyper Headers
        let auth_val = match req.headers().get_one("Authorization") {
            Some(a) => a,
            None => return Outcome::Failure((Status::Unauthorized, ())),
        };

        // The value of the header is the type of the auth (Basic or Bearer), followed by an
        // encoded string, separate by whitespace.
        let auth_strings: Vec<String> = auth_val.split_whitespace().map(String::from).collect();
        if auth_strings.len() != 2 {
            //TODO: Should this be BadRequest?
            return Outcome::Failure((Status::Unauthorized, ()));
        }
        // We're looking for a Basic token
        if auth_strings[0] != "Basic" {
            //TODO: This probably isn't right, maybe check if bearer?
            return Outcome::Failure((Status::Unauthorized, ()));
        }

        let outcome = match base64::decode(&auth_strings[1]) {
            Ok(userpass) => {

                // Hard-coded credential for testing
                if userpass == b"admin:test" {
                    Outcome::Success(ValidBasicToken {
                        user: "admin".to_owned(),
                    })
                } else {
                    Outcome::Failure((Status::Unauthorized, ()))
                }
            }
            Err(_) => Outcome::Failure((Status::Unauthorized, ())),
        };

        outcome
    }
}

#[derive(Debug, Serialize, RustcEncodable, RustcDecodable)]
pub struct TrowToken {
    pub user: String,
    pub token: String,
}

//Just using the default token claim stuff
//Could add scope stuff (which repos, what rights), but could also keep this in DB
//Mirroring Docker format would allow resuse of existing token server implementations
#[derive(Clone, Debug, Serialize, Deserialize)]
struct TokenClaim {
    iss: String, // (Issuer) The issuer of the token, typically the fqdn of the authorization server.
    sub: String, // (Subject)The subject of the token; the name or id of the client which requested it. This should be empty if the client did not authenticate.
    aud: String, // (Audience) The intended audience of the token; the name or id of the service which will verify the token to authorize the client/subject.
    exp: u64, // (Expiration) The token should only be considered valid up to this specified date and time.
    nbf: u64, // (Not Before) The token should not be considered valid before this specified date and time.
    iat: u64, // (Issued At) Specifies the date and time which the Authorization server generated this token.
    jti: String, // (JWT ID) A unique identifier for this token. Can be used by the intended audience to prevent replays of the token.
}
/*
 * Create new jsonwebtoken.
 * Token consists of a string with 3 comma separated fields header, payload, signature
 */
pub fn new(vbt: ValidBasicToken) -> Result<TrowToken, frank_jwt::Error> {
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    // build token from structure and return token string
    let token_claim = TokenClaim {
        iss: "trow.local".to_owned(), // TODO: get this from somewhere, should be fqdn
        sub: vbt.user.clone(),
        aud: "Trow Registry".to_owned(),
        exp: current_time.add(Duration::new(TOKEN_DURATION, 0)).as_secs(),
        nbf: current_time.as_secs(),
        iat: current_time.as_secs(),
        jti: Uuid::new_v4().to_string(),
    };

    let header = json!({});
    let payload = serde_json::to_value(&token_claim)?;

    let token = encode(
        header,
        &AUTHORISATION_SECRET.to_string(),
        &payload,
        Algorithm::HS256,
    )?;

    Ok(TrowToken {
        user: vbt.user,
        token: token.to_string(),
    })
}
/*
 * Responder returns token as JSON body
 */
impl<'r> Responder<'r> for TrowToken {
    fn respond_to(self, _: &Request) -> Result<Response<'r>, Status> {
        let formatted_body = Cursor::new(self.token);
        Response::build()
            .status(Status::Ok)
            .header(ContentType::JSON)
            .sized_body(formatted_body)
            .ok()
    }
}
/*
 *
 */
impl<'a, 'r> FromRequest<'a, 'r> for TrowToken {
    type Error = ();
    fn from_request(req: &'a Request<'r>) -> request::Outcome<TrowToken, ()> {
        // Look in headers for an Authorization header
        /*
                let keys: Vec<_> = req.headers().get("Authorization").collect();
                if keys.len() != 1 {
                    // no key return false in auth structure
                    return Outcome::Failure((Status::Unauthorized, ()));
                }
        */
        let auth_val = match req.headers().get_one("Authorization") {
            Some(a) => a,
            None => return Outcome::Failure((Status::Unauthorized, ())),
        };

        //Check header handling - isn't there a next?
        // split the header on white space
        let auth_strings: Vec<String> = auth_val.split_whitespace().map(String::from).collect();
        if auth_strings.len() != 2 {
            //TODO: Should this be BadRequest?
            return Outcome::Failure((Status::Unauthorized, ()));
        }
        // We're looking for a Bearer token
        //TODO: Maybe should forward or something on Basic
        if auth_strings[0] != "Bearer" {
            return Outcome::Failure((Status::Unauthorized, ()));
        }

        // parse for bearer token
        // TODO: frank_jwt is meant to verify iat, nbf etc, but doesn't.
        let dec_token = match decode(
            &auth_strings[1],
            &AUTHORISATION_SECRET.to_string(),
            Algorithm::HS256,
        ) {
            Ok((_, payload)) => payload,
            Err(_) => {
                warn!("Failed to decode user token");
                return Outcome::Failure((Status::Unauthorized, ()));
            }
        };

        let ttoken = TrowToken {
            user: dec_token["sub"].to_string(),
            token: auth_strings[1].clone(),
        };

        Outcome::Success(ttoken)
    }
}

#[cfg(test)]
mod test {
    use response::trowtoken::{self, ValidBasicToken};
    use rocket::http::Status;

    use response::test_helper::test_route;

    #[test]
    fn token_ok() {
        let user = ValidBasicToken {
            user: "admin".to_string(),
        };
        let response = test_route(trowtoken::new(user));
        assert_eq!(response.status(), Status::Ok);
    }
}
