//! This file should peferably be empty.
//! The use of this file is unclear, but it does at the very least
//! clean out other sections of code.
use std::net::ToSocketAddrs;

use failure::Error;
use rocket;
use tokio_core::reactor;
use tokio_io::AsyncRead;
use futures::Future;

use config;
use errors;

// --- Channel Helpers ---
pub fn send(tx: config::SendSock, msg: config::BackendMessage) -> Result<(), Error> {
    tx.send(msg).or_else(|e| {
        warn!("Error: {}", e);
        Err(e.into())
    })
}

pub fn recv(rx: &config::RecvSock) -> Result<config::BackendMessage, Error> {
    use std::time;
    let duration = time::Duration::from_secs(10);
    rx.recv_timeout(duration).or_else(|e| {
        warn!("{}", e);
        Err(e.into())
    })
}
