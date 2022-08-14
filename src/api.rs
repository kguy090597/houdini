// SPDX-License-Identifier: Apache-2.0
//
// Houdini  A container escape artist
// Copyright (c) 2022  William Findlay
//
// February 25, 2022  William Findlay  Created this.
//

//! The Houdini API.

pub mod client;

mod middleware;
mod uds;
mod vsock;

use std::path::Path;
use std::str;

use anyhow::{Context as _, Result};
use axum::{
    debug_handler,
    handler::Handler,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use hyper::StatusCode;
use tokio::net::UnixListener;
use tower::ServiceBuilder;

use crate::{
    tricks::{report::TrickReport, Trick},
    CONFIG,
};

use tokio_vsock::VsockListener;
use tokio_vsock::VsockStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use futures::StreamExt as _;

pub async fn serve(socket: Option<&Path>) -> Result<()> {
    let socket = if let Some(socket) = socket {
        socket
    } else {
        &CONFIG.api.socket
    };

    let _ = tokio::fs::remove_file(socket).await;
    if let Some(parent) = &CONFIG.api.socket.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("failed to create parent directory for Houdini socket")?
    }

    let uds = UnixListener::bind(socket).context("failed to bind to Houdini socket")?;

    // Add routes
    let app = Router::new()
        .route("/", get(ping))
        .route("/ping", get(ping))
        .route("/trick", post(run_trick));

    // Add fallback handler
    let app = app.fallback(not_found.into_service());

    // Add middleware
    let app = app.route_layer(
        ServiceBuilder::new().layer(axum::middleware::from_fn(middleware::log_connection)),
    );

    tracing::info!("server listening on {:?}...", socket);
    axum::Server::builder(uds::ServerAccept { uds })
        .serve(app.into_make_service_with_connect_info::<uds::UdsConnectInfo>())
        .await
        .context("failed to start Houdini API server")
}

//https://github.com/rust-vsock/tokio-vsock/blob/master/tests/vsock.rs
pub async fn vsock_client(cid: u32, port: u32) -> Result<()> {
    let string = String::from("hello");
    //let mut rng = rand::thread_rng();
    //let mut blob: Vec<u8> = vec![];
    let mut blob: &[u8]= string.as_bytes();
    let test_blob_size: usize = blob.len();
    let test_block_size: usize = blob.len();
    let mut rx_blob = vec![];
    let mut tx_pos = 0;

    rx_blob.resize(test_blob_size, 0);
    //rng.fill_bytes(&mut blob);

    let mut stream = VsockStream::connect(cid, port)
        .await
        .expect("connection failed");

    while tx_pos < test_blob_size {
        let written_bytes = stream
            .write(&blob[tx_pos..tx_pos + test_block_size])
            .await
            .expect("write failed");
        if written_bytes == 0 {
            panic!("stream unexpectedly closed");
        }

        let mut rx_pos = tx_pos;
        while rx_pos < (tx_pos + written_bytes) {
            let read_bytes = stream
                .read(&mut rx_blob[rx_pos..])
                .await
                .expect("read failed");
            if read_bytes == 0 {
                panic!("stream unexpectedly closed");
            }
            rx_pos += read_bytes;
            let s = match str::from_utf8(&rx_blob) {
                Ok(v) => v,
                Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
            };
            println!("Recieved: {:?}", rx_blob);
            println!("Recieved: {:?}", s);
        }

        tx_pos += written_bytes;
    }
    Ok(())
}

//https://github.com/rust-vsock/tokio-vsock/blob/master/test_server/src/main.rs
pub async fn vsock_server(cid: u32, port: u32) -> Result<()> {

    let listener = VsockListener::bind(cid, port)
        .expect("unable to bind virtio listener");
        println!("Listening for connections on port: {}", port);
    let mut incoming = listener.incoming();
    while let Some(result) = incoming.next().await {
        match result {
            Ok(mut stream) => {
                println!("Got connection ============");
                tokio::spawn(async move {
                    loop {
                        let mut buf = vec![0u8; 5000];
                        let len = stream.read(&mut buf).await.unwrap();
                        if len == 0 {
                            break;
                        }

                        println!("Got data: {:?}", &buf);
                        buf.resize(len+1, 0);
                        println!("Responding with: {:?}", &buf);
                        println!("Responding to: {:?}", stream.peer_addr());
                        stream.write_all(&buf).await.unwrap();
                        println!("Finished Writing");

                        //logic goes here
                        //parse request/response and send appropriate request/response

                    }
                    println!("Out of loop");
                });
                println!("done here");
            }
            Err(e) => {
                println!("Got error: {:?}", e);
                break;
            }
        }
        println!("done there");
    }
    Ok(())


    /*let virtio_sock = VsockListener::bind(cid, port)
    .expect("unable to bind virtio listener");

    println!("Listening for connections on cid: {}", cid);
    println!("Listening for connections on port: {}", port);

    // Add routes
    let app = Router::new()
        .route("/", get(ping))
        .route("/ping", get(ping))
        .route("/trick", post(run_trick));

    // Add fallback handler
    let app = app.fallback(not_found.into_service());

    // Add middleware
    let app = app.route_layer(
        ServiceBuilder::new().layer(axum::middleware::from_fn(middleware::log_connection)),
    );
    

    tracing::info!("server listening on {:?}...", virtio_sock);
    axum::Server::builder(vsock::ServerAccept { virtio_sock })
        .serve(app.into_make_service_with_connect_info::<vsock::VsockConnectInfo>())
        .await
        .context("failed to start Houdini API server")*/
}

async fn ping() -> &'static str {
    "pong"
}

#[debug_handler]
async fn run_trick(
    Json(trick): Json<Trick>,
) -> Result<Json<TrickReport>, (StatusCode, &'static str)> {
    let report = trick.run().await;
    Ok(Json(report))
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "bad endpoint")
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use super::*;
    use serial_test::serial;
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    #[serial]
    async fn test_api_server_runs_smoke() {
        let path = tempfile::NamedTempFile::new()
            .unwrap()
            .into_temp_path()
            .to_path_buf();

        let jh =
            tokio::spawn(async move { serve(Some(&path)).await.expect("server should serve") });
        tokio::time::sleep(Duration::from_secs(1)).await;

        assert!(!jh.is_finished());
        let _ = jh.abort();
    }

    #[tokio::test]
    #[traced_test]
    #[serial]
    async fn test_api_ping() {
        let path = Arc::new(
            tempfile::NamedTempFile::new()
                .unwrap()
                .into_temp_path()
                .to_path_buf(),
        );

        let p = path.clone();
        let jh = tokio::spawn(async move { serve(Some(&p)).await.expect("server should serve") });
        tokio::time::sleep(Duration::from_secs(1)).await;

        let client = client::HoudiniClient::new(Some(&path)).expect("client should connect");
        client.ping().await.expect("ping should succeed");

        assert!(!jh.is_finished());
        let _ = jh.abort();
    }

    #[tokio::test]
    #[traced_test]
    #[serial]
    async fn test_api_trick() {
        let path = Arc::new(
            tempfile::NamedTempFile::new()
                .unwrap()
                .into_temp_path()
                .to_path_buf(),
        );

        let p = path.clone();
        let jh = tokio::spawn(async move { serve(Some(&p)).await.expect("server should serve") });
        tokio::time::sleep(Duration::from_secs(1)).await;

        let client = client::HoudiniClient::new(Some(&path)).expect("client should connect");

        let yaml = r#"
            name: foo
            steps: []
            "#;
        let trick = serde_yaml::from_str(yaml).expect("trick should deserialize");

        let report = client.trick(&trick).await.expect("trick should succeed");
        assert_eq!(report.name, "foo");
        assert_eq!(report.steps.len(), 0);

        assert!(!jh.is_finished());
        let _ = jh.abort();
    }
}
