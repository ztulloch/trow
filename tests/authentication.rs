extern crate crypto;
extern crate environment;
#[macro_use]
extern crate hyper;
extern crate rand;
extern crate reqwest;
extern crate serde_json;
extern crate trow;
extern crate base64;
extern crate trow_server;

mod common;

#[cfg(test)]
mod authentication_tests {

    use environment::Environment;

    use common;
    use reqwest::StatusCode;
    use reqwest::header::LOCATION;
    use reqwest;
    use serde_json;
    use base64::encode;
    use std::fs::{self, File};
    use std::io::Read;
    use std::process::Child;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;
    use trow::types::{RepoCatalog, RepoName, TagList};
    use trow_server::manifest;

    const TROW_ADDRESS: &str = "https://trow.test:8443";

    const DIST_API_HEADER: &str = "Docker-Distribution-API-Version";
    const UPLOAD_HEADER: &str = "Docker-Upload-Uuid";
    const AUTHN_HEADER: &str = "www-authenticate";
    const AUTHZ_HEADER: &str = "Authorization";

    struct TrowInstance {
        pid: Child,
    }
    /// Call out to cargo to start trow.
    /// Seriously considering moving to docker run.

    fn start_trow() -> TrowInstance {
        let mut child = Command::new("cargo")
            //.current_dir("../../")
            .arg("run")
            .env_clear()
            .envs(Environment::inherit().compile())
            .spawn()
            .expect("failed to start");

        let mut timeout = 20;

        let mut buf = Vec::new();
        File::open("./certs/ca.crt")
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap();
        let cert = reqwest::Certificate::from_pem(&buf).unwrap();
        // get a client builder
        let client = reqwest::Client::builder()
            .add_root_certificate(cert)
            .build()
            .unwrap();

        let mut response = client.get(TROW_ADDRESS).send();
        while timeout > 0 && (response.is_err() || (response.unwrap().status() != StatusCode::OK)) {
            thread::sleep(Duration::from_millis(100));
            response = client.get(TROW_ADDRESS).send();
            timeout -= 1;
        }
        if timeout == 0 {
            child.kill().unwrap();
            panic!("Failed to start Trow");
        }
        TrowInstance { pid: child }
    }

    impl Drop for TrowInstance {
        fn drop(&mut self) {
            //Y U NO HV STOP?
            self.pid.kill().unwrap();
        }
    }

    fn test_auth_redir(cl: &reqwest::Client) {
        let resp = cl.get(&(TROW_ADDRESS.to_owned() + "/test-auth")).send().unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        //Test get redir header
        assert_eq!(
            resp.headers().get(AUTHN_HEADER).unwrap(), 
            "Bearer realm=\"https://0.0.0.0:8443/login\",service=\"trow_registry\",scope=\"push/pull\""
        );
    }

    fn test_login(cl: &reqwest::Client) {
        let bytes = encode(b"admin:test");
        let resp = cl.get(&(TROW_ADDRESS.to_owned() +"/login")).header(
            AUTHZ_HEADER, format!("Basic {}", bytes)).send().unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    fn test_login_fail(cl: &reqwest::Client) {
        let resp = cl.get(&(TROW_ADDRESS.to_owned() +"/login")).header(AUTHZ_HEADER, "Basic thisstringwillfail").send().unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_runner() {
        //Need to start with empty repo
        fs::remove_dir_all("./data").unwrap_or(());

        //Had issues with stopping and starting trow causing test fails.
        //It might be possible to improve things with a thread_local
        let _trow = start_trow();

        let mut buf = Vec::new();
        File::open("./certs/ca.crt")
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap();
        let cert = reqwest::Certificate::from_pem(&buf).unwrap();
        // get a client builder
        let client = reqwest::Client::builder()
            .add_root_certificate(cert)
            .build()
            .unwrap();

        println!("Running test_auth_redir()");
        test_auth_redir(&client);
        println!("Running test_login()");
        test_login(&client);
        println!("Running test_login_fail()");
        test_login_fail(&client);

    }
}
