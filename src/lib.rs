#![feature(proc_macro_hygiene, decl_macro)]
#![feature(plugin)]

#[macro_use]
extern crate failure;
extern crate futures;
extern crate grpcio;
extern crate hostname;
extern crate frank_jwt;
extern crate orset;
extern crate protobuf;
extern crate base64;
#[macro_use]
extern crate rocket;
extern crate rocket_contrib;
extern crate rustc_serialize;
extern crate serde;
extern crate serde_json;
extern crate uuid;
#[macro_use]
extern crate display_derive;

extern crate trow_protobuf;
extern crate trow_server;

extern crate env_logger;
extern crate crypto;
extern crate chrono;

use log::{LogLevelFilter, LogRecord, SetLoggerError};
#[macro_use]
extern crate failure_derive;
#[macro_use(log, warn, info, debug)]
extern crate log;
#[macro_use]
extern crate serde_derive;

#[cfg(test)]
extern crate quickcheck;

use failure::Error;
use std::env;
use std::fs;
use std::path::Path;
use std::thread;

use grpcio::{ChannelBuilder, EnvBuilder};
use rocket::fairing;
use std::sync::Arc;

mod client_interface;
pub mod response;
mod routes;
pub mod types;

use client_interface::{BackendClient, ClientInterface};

//TODO: Make this take a cause or description
#[derive(Fail, Debug)]
#[fail(display = "invalid data directory")]
pub struct ConfigError {}

#[derive(Clone, Debug)]
pub struct NetAddr {
    pub host: String,
    pub port: u16,
}

/*
 * Configuration for Trow. This isn't direct fields on the builder so that we can pass it
 * to Rocket to manage.
 */
#[derive(Clone, Debug)]
pub struct TrowConfig {
    data_dir: String,
    addr: NetAddr,
    tls: Option<TlsConfig>,
    grpc: GrpcConfig,
    host_names: Vec<String>,
    allow_prefixes: Vec<String>,
    allow_images: Vec<String>,
    deny_prefixes: Vec<String>,
    deny_images: Vec<String>,
    dry_run: bool,
}

#[derive(Clone, Debug)]
struct GrpcConfig {
    listen: NetAddr,
}

#[derive(Clone, Debug)]
struct TlsConfig {
    cert_file: String,
    key_file: String,
}

fn init_trow_server(config: TrowConfig) -> Result<std::thread::JoinHandle<()>, Error> {
    debug!("Starting Trow server");

    //Could pass full config here.
    //Pros: less work, new args added automatically
    //-s: ties frontend to backend, some uneeded/unwanted vars

    let ts = trow_server::build_server(
        &config.data_dir,
        &config.grpc.listen.host,
        config.grpc.listen.port,
        config.allow_prefixes,
        config.allow_images,
        config.deny_prefixes,
        config.deny_images,
    );
    //TODO: probably shouldn't be reusing this cert
    let ts = if let Some(tls) = config.tls {
        ts.add_tls(fs::read(tls.cert_file)?, fs::read(tls.key_file)?)
    } else {
        ts
    };

    Ok(thread::spawn(move || {
        ts.start_sync();
    }))
}

/// Build the logging agent with formatting.
fn init_logger() -> Result<(), SetLoggerError> {
    let mut builder = env_logger::LogBuilder::new();
    builder
        .format(|record: &LogRecord| {
            format!("{}[{}] {}", record.target(), record.level(), record.args(),)
        })
        .filter(None, LogLevelFilter::Error);

    if env::var("RUST_LOG").is_ok() {
        builder.parse(&env::var("RUST_LOG").unwrap());
    }

    builder.init()
}

pub struct TrowBuilder {
    config: TrowConfig,
}

impl TrowBuilder {
    pub fn new(
        data_dir: String,
        addr: NetAddr,
        listen: NetAddr,
        host_names: Vec<String>,
        allow_prefixes: Vec<String>,
        allow_images: Vec<String>,
        deny_prefixes: Vec<String>,
        deny_images: Vec<String>,
        dry_run: bool,
    ) -> TrowBuilder {
        let config = TrowConfig {
            data_dir,
            addr,
            tls: None,
            grpc: GrpcConfig { listen },
            host_names,
            allow_prefixes,
            allow_images,
            deny_prefixes,
            deny_images,
            dry_run,
        };
        TrowBuilder { config }
    }

    pub fn with_tls(&mut self, cert_file: String, key_file: String) -> &mut TrowBuilder {
        let cfg = TlsConfig {
            cert_file,
            key_file,
        };
        self.config.tls = Some(cfg);
        self
    }

    fn build_rocket_config(&self) -> Result<rocket::config::Config, Error> {
        let mut cfg = rocket::config::Config::build(rocket::config::Environment::Production)
            .address(self.config.addr.host.clone())
            .port(self.config.addr.port)
            .keep_alive(60)
            .workers(256);

        if let Some(ref tls) = self.config.tls {
            if !(Path::new(&tls.cert_file).is_file() && Path::new(&tls.key_file).is_file()) {
                return  Err(format_err!("Trow requires a TLS certificate and key, but failed to find them. \nExpected to find TLS certificate at {} and key at {}", tls.cert_file, tls.key_file));
            }
            cfg = cfg.tls(tls.cert_file.clone(), tls.key_file.clone());
        }
        let cfg = cfg.finalize()?;
        Ok(cfg)
    }

    pub fn start(&self) -> Result<(), Error> {
        init_logger()?;
        // GRPC Backend thread.
        let _grpc_thread = init_trow_server(self.config.clone())?;

        //TODO: shouldn't need to clone rocket config
        let rocket_config = &self.build_rocket_config()?;
        println!(
            "Starting trow on {}:{}",
            self.config.addr.host, self.config.addr.port
        );
        println!("\n**Validation callback configuration\n");

        println!("  By default all remote images are denied, and all local images present in the repository are allowed\n");

        println!(
            "  These host names will considered local (refer to this regsitry): {:?}",
            self.config.host_names
        );
        println!(
            "  Images with these prefixes are explicitly allowed: {:?}",
            self.config.allow_prefixes
        );
        println!(
            "  Images with these names are explicitly allowed: {:?}",
            self.config.allow_images
        );
        println!(
            "  Local images with these prefixes are explicitly denied: {:?}",
            self.config.deny_prefixes
        );
        println!(
            "  Local images with these names are explicitly denied: {:?}\n",
            self.config.deny_images
        );
        if self.config.dry_run {
            println!("Dry run, exiting.");
            std::process::exit(0);
        }
        rocket::custom(rocket_config.clone())
            .manage(build_handlers(
                &self.config.grpc.listen.host,
                self.config.grpc.listen.port,
            ))
            .manage(self.config.clone())
            .attach(fairing::AdHoc::on_attach(
                "SIGTERM handler",
                |r| match attach_sigterm() {
                    Ok(_) => Ok(r),
                    Err(_) => Err(r),
                },
            ))
            .attach(fairing::AdHoc::on_response(
                "Set API Version Header",
                |_, resp| {
                    //Only serve v2. If we also decide to support older clients, this will to be dropped on some paths
                    resp.set_raw_header("Docker-Distribution-API-Version", "registry/2.0");
                },
            ))
            .attach(fairing::AdHoc::on_launch("Launch Message", |_| {
                println!("Trow is up and running!");
            }))
            .mount("/", routes::routes())
            .register(routes::catchers())
            .launch();
        Ok(())
    }
}

fn attach_sigterm() -> Result<(), Error> {
    ctrlc::set_handler(|| {
        info!("SIGTERM caught, shutting down...");
        std::process::exit(0);
    })
    .map_err(|e| e.into())
}

pub fn build_handlers(listen_host: &str, listen_port: u16) -> ClientInterface {
    debug!("Connecting to backend: {}:{}", listen_host, listen_port);
    let env = Arc::new(EnvBuilder::new().build());
    let ch = ChannelBuilder::new(env).connect(&format!("{}:{}", listen_host, listen_port));
    let client = BackendClient::new(ch);
    ClientInterface::new(client)
}
