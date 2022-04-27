//! Faucet server implementation.

use actix_cors::Cors;
use actix_web::http::{header, StatusCode};
use actix_web::web::{get, post, Bytes};
use actix_web::{App, HttpResponse, HttpServer, Responder};
use eyre::Result;
use tracing::{error, info};

use crate::{active_requests, config, erc20_tokens, id, neon_token};

/// Starts the server in listening mode.
pub async fn start(rpc_bind: &str, rpc_port: u16, workers: usize) -> Result<()> {
    info!("{} Bind {}:{}", id::default(), rpc_bind, rpc_port);

    HttpServer::new(|| {
        let mut cors = Cors::default();
        let allowed_origins = config::allowed_origins();
        if !allowed_origins.is_empty() {
            cors = cors
                .allowed_methods(vec!["GET", "POST"])
                .allowed_header(header::CONTENT_TYPE)
                .max_age(3600);
            for origin in &allowed_origins {
                cors = cors.allowed_origin(origin);
            }
        }

        App::new()
            .wrap(cors)
            .route("/request_ping", get().to(handle_request_ping))
            .route("/request_version", get().to(handle_request_version))
            .route(
                "/request_neon_in_galans",
                post().to(handle_request_neon_in_galans),
            )
            .route("/request_neon", post().to(handle_request_neon))
            .route("/request_erc20_list", get().to(handle_request_erc20_list))
            .route("/request_erc20", post().to(handle_request_erc20))
    })
    .bind((rpc_bind, rpc_port))?
    .workers(workers)
    .run()
    .await?;

    Ok(())
}

/// Handles a ping request.
async fn handle_request_ping(body: Bytes) -> impl Responder {
    let id = id::generate();
    let counter = active_requests::increment();

    println!();
    info!("{} Handling ping...", id);
    info!("{} Active requests: {}", id, counter);

    let input = String::from_utf8(body.to_vec());
    if let Err(err) = input {
        error!("{} BadRequest (body): {}", id, err);
        return HttpResponse::with_body(StatusCode::BAD_REQUEST, err.to_string());
    }

    let ping = input.unwrap();
    info!("{} Ping '{}'", id, ping);

    HttpResponse::with_body(StatusCode::OK, ping)
}

/// Handles a version request.
async fn handle_request_version() -> impl Responder {
    let id = id::generate();
    let counter = active_requests::increment();

    println!();
    info!("{} Handling version request...", id);
    info!("{} Active requests: {}", id, counter);

    let version = crate::version::display!();
    info!("{} Faucet {}", id, version);

    version
}

/// Handles a request for NEON airdrop in galans (1 galan = 10E-9 NEON).
async fn handle_request_neon_in_galans(body: Bytes) -> impl Responder {
    let id = id::generate();
    let counter = active_requests::increment();

    println!();
    info!("{} Handling request for NEON (in galans) Airdrop...", id);
    info!("{} Active requests: {}", id, counter);

    let input = String::from_utf8(body.to_vec());
    if let Err(err) = input {
        error!("{} BadRequest (body): {}", id, err);
        return HttpResponse::with_body(StatusCode::BAD_REQUEST, err.to_string());
    }

    let input = input.unwrap();
    let airdrop = serde_json::from_str::<neon_token::Airdrop>(&input);
    if let Err(err) = airdrop {
        error!("{} BadRequest (json): {} in '{}'", id, err, input);
        return HttpResponse::with_body(StatusCode::BAD_REQUEST, err.to_string());
    }

    let mut airdrop = airdrop.unwrap();
    airdrop.in_fractions = true;
    if let Err(err) = neon_token::airdrop(&id, airdrop).await {
        error!("{} InternalServerError: {}", id, err);
        return HttpResponse::with_body(StatusCode::INTERNAL_SERVER_ERROR, err.to_string());
    }

    HttpResponse::with_body(StatusCode::OK, String::default())
}

/// Handles a request for NEON airdrop.
async fn handle_request_neon(body: Bytes) -> impl Responder {
    let id = id::generate();
    let counter = active_requests::increment();

    println!();
    info!("{} Handling request for NEON Airdrop...", id);
    info!("{} Active requests: {}", id, counter);

    let input = String::from_utf8(body.to_vec());
    if let Err(err) = input {
        error!("{} BadRequest (body): {}", id, err);
        return HttpResponse::with_body(StatusCode::BAD_REQUEST, err.to_string());
    }

    let input = input.unwrap();
    let airdrop = serde_json::from_str::<neon_token::Airdrop>(&input);
    if let Err(err) = airdrop {
        error!("{} BadRequest (json): {} in '{}'", id, err, input);
        return HttpResponse::with_body(StatusCode::BAD_REQUEST, err.to_string());
    }

    if let Err(err) = neon_token::airdrop(&id, airdrop.unwrap()).await {
        error!("{} InternalServerError: {}", id, err);
        return HttpResponse::with_body(StatusCode::INTERNAL_SERVER_ERROR, err.to_string());
    }

    HttpResponse::with_body(StatusCode::OK, String::default())
}

/// Handles a request for list of available ERC20 tokens.
async fn handle_request_erc20_list() -> impl Responder {
    let id = id::generate();
    let counter = active_requests::increment();

    println!();
    info!("{} Handling request for list of ERC20...", id);
    info!("{} Active requests: {}", id, counter);

    let mut list = String::from("[");
    for t in config::tokens() {
        list.push('"');
        list.push_str(&t);
        list.push('"');
        list.push(',');
    }
    if list.ends_with(',') {
        list.pop();
    }
    list.push(']');

    list
}

/// Handles a request for ERC20 tokens airdrop.
async fn handle_request_erc20(body: Bytes) -> impl Responder {
    let id = id::generate();
    let counter = active_requests::increment();

    println!();
    info!("{} Handling request for ERC20 Airdrop...", id);
    info!("{} Active requests: {}", id, counter);

    let input = String::from_utf8(body.to_vec());
    if let Err(err) = input {
        error!("{} BadRequest (body): {}", id, err);
        return HttpResponse::with_body(StatusCode::BAD_REQUEST, err.to_string());
    }

    let input = input.unwrap();
    let airdrop = serde_json::from_str::<erc20_tokens::Airdrop>(&input);
    if let Err(err) = airdrop {
        error!("{} BadRequest (json): {} in '{}'", id, err, input);
        return HttpResponse::with_body(StatusCode::BAD_REQUEST, err.to_string());
    }

    if let Err(err) = erc20_tokens::airdrop(&id, airdrop.unwrap()).await {
        error!("{} InternalServerError: {}", id, err);
        return HttpResponse::with_body(StatusCode::INTERNAL_SERVER_ERROR, err.to_string());
    }

    HttpResponse::with_body(StatusCode::OK, String::default())
}

/// Handles a request for graceful shutdown.
#[allow(unused)]
async fn handle_request_stop(body: Bytes) -> impl Responder {
    #[derive(serde::Deserialize)]
    struct Stop {
        /// Milliseconds to wait before shutdown.
        delay: u64,
    }

    use nix::sys::signal;
    use nix::unistd::Pid;
    use tokio::time::Duration;

    let id = id::generate();
    let counter = active_requests::increment();

    info!("{} Shutting down...", id);
    info!("{} Active requests: {}", id, counter);

    let input = String::from_utf8(body.to_vec());
    if let Err(err) = input {
        error!("{} BadRequest (body): {}", id, err);
        return HttpResponse::with_body(StatusCode::BAD_REQUEST, err.to_string());
    }

    let input = input.unwrap();
    let stop = serde_json::from_str::<Stop>(&input);
    if let Err(err) = stop {
        error!("{} BadRequest (json): {} in '{}'", id, err, input);
        return HttpResponse::with_body(StatusCode::BAD_REQUEST, err.to_string());
    }

    let delay = stop.unwrap().delay;
    if delay > 0 {
        info!("{} Sleeping {} millis...", id, delay);
        tokio::time::sleep(Duration::from_millis(delay)).await;
    }

    let terminate = signal::kill(Pid::this(), signal::SIGTERM);
    if let Err(err) = terminate {
        error!("{} BadRequest (terminate): {}", id, err);
        return HttpResponse::with_body(StatusCode::BAD_REQUEST, err.to_string());
    }

    HttpResponse::with_body(StatusCode::OK, String::default())
}
