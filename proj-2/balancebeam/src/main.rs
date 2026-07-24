mod request;
mod response;

use clap::Clap;
use core::{fmt, time};
use log::log;
use rand::seq::IteratorRandom;
use rand::{Rng, SeedableRng};
use std::collections::{HashMap, HashSet};
use std::io;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;
use tokio::net::{TcpListener, TcpStream};

/// Contains information parsed from the command-line invocation of balancebeam. The Clap macros
/// provide a fancy way to automatically construct a command-line argument parser.
#[derive(Clap, Debug)]
#[clap(about = "Fun with load balancing")]
struct CmdOptions {
    #[clap(
        short,
        long,
        about = "IP/port to bind to",
        default_value = "0.0.0.0:1100"
    )]
    bind: String,
    #[clap(short, long, about = "Upstream host to forward requests to")]
    upstream: Vec<String>,
    #[clap(
        long,
        about = "Perform active health checks on this interval (in seconds)",
        default_value = "10"
    )]
    active_health_check_interval: usize,
    #[clap(
        long,
        about = "Path to send request to for active health checks",
        default_value = "/"
    )]
    active_health_check_path: String,
    #[clap(
        long,
        about = "Maximum number of requests to accept per IP per minute (0 = unlimited)",
        default_value = "0"
    )]
    max_requests_per_minute: usize,
}

enum ProxyError {
    NoUpstream,
}

impl fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "No avaliable upstream")
    }
}

struct Upstream {
    addresses: Vec<String>,
    valid_indexes: HashSet<usize>,
}

impl Upstream {
    fn new(addresses: Vec<String>) -> Self {
        let upstream_number = addresses.len();
        Upstream {
            addresses,
            valid_indexes: ((0..upstream_number).into_iter().collect()),
        }
    }
    fn get_valid(&self) -> Result<(String, usize), ()> {
        let mut rng = rand::rngs::StdRng::from_entropy();
        let upstream_idx = self.valid_indexes.iter().choose(&mut rng);
        match upstream_idx {
            Some(upstream_idx) => Ok((self.addresses[*upstream_idx].clone(), *upstream_idx)),
            None => Err(()),
        }
    }

    fn set_valid(&mut self, index: usize, state: bool) {
        if state == true {
            self.valid_indexes.insert(index);
        } else {
            self.valid_indexes.remove(&index);
        }
    }
}

/// Contains information about the state of balancebeam (e.g. what servers we are currently proxying
/// to, what servers have failed, rate limiting counts, etc.)
///
/// You should add fields to this struct in later milestones.
struct ProxyState {
    /// How frequently we check whether upstream servers are alive (Milestone 4)
    #[allow(dead_code)]
    active_health_check_interval: usize,
    /// Where we should send requests when doing active health checks (Milestone 4)
    #[allow(dead_code)]
    active_health_check_path: String,
    /// Maximum number of requests an individual IP can make in a minute (Milestone 5)
    #[allow(dead_code)]
    max_requests_per_minute: usize,
    // Addresses and State of servers that we are proxying to
    upstream: Upstream,
}
#[tokio::main]
async fn main() -> io::Result<()> {
    // Initialize the logging library. You can print log messages using the `log` macros:
    // https://docs.rs/log/0.4.8/log/ You are welcome to continue using print! statements; this
    // just looks a little prettier.
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "debug");
    }
    pretty_env_logger::init();

    // Parse the command line arguments passed to this program
    let options = CmdOptions::parse();
    if options.upstream.len() < 1 {
        log::error!("At least one upstream server must be specified using the --upstream option.");
        std::process::exit(1);
    }

    // Start listening for connections
    let mut listener = match TcpListener::bind(&options.bind).await {
        Ok(listener) => listener,
        Err(err) => {
            log::error!("Could not bind to {}: {}", options.bind, err);
            std::process::exit(1);
        }
    };
    log::info!("Listening for requests on {}", options.bind);

    // Handle incoming connections

    let state = ProxyState {
        upstream: Upstream::new(options.upstream),
        active_health_check_interval: options.active_health_check_interval,
        active_health_check_path: options.active_health_check_path,
        max_requests_per_minute: options.max_requests_per_minute,
    };
    let state_ref = Arc::new(RwLock::new(state));
    // check healthy interval Future
    let state_ref_copy = state_ref.clone();
    let request_times: Arc<Mutex<HashMap<String, (usize, std::time::Instant)>>> =
        Arc::new(Mutex::new(HashMap::new()));
    tokio::spawn(async move {
        loop {
            let active_health_check_interval =
                { state_ref_copy.read().unwrap().active_health_check_interval };
            // delay for interval
            tokio::time::delay_for(tokio::time::Duration::from_secs(
                active_health_check_interval as u64,
            ))
            .await;
            // check every upstream
            let (active_health_check_path, upstream_addresses) = {
                let lock = state_ref_copy.read().unwrap();
                (
                    lock.active_health_check_path.clone(),
                    lock.upstream.addresses.clone(),
                )
            };
            for (index, addr) in upstream_addresses.iter().enumerate() {
                // check
                let health_check_result =
                    request::health_check(&addr, &active_health_check_path).await;
                let upstream = { &mut state_ref_copy.write().unwrap().upstream };
                if health_check_result == true {
                    upstream.set_valid(index, true);
                } else {
                    upstream.set_valid(index, false);
                }
            }
        }
    });
    loop {
        let (stream, _) = listener.accept().await?;
        let state_ref = state_ref.clone();
        let request_times = request_times.clone();
        tokio::spawn(async move {
            handle_connection(stream, state_ref, request_times).await;
        });
    }
}

async fn connect_to_upstream(state: Arc<RwLock<ProxyState>>) -> Result<TcpStream, ProxyError> {
    loop {
        let upstream = {
            let lock = &state.read().unwrap();
            let upstream = &lock.upstream;
            upstream.get_valid()
        };
        match upstream {
            Ok((upstream, index)) => {
                let upstream = TcpStream::connect(&upstream)
                    .await
                    .or_else(|err: io::Error| {
                        log::error!("Failed to connect to upstream {}: {}", upstream, err);
                        Err(err)
                    });
                match upstream {
                    Ok(upstream) => {
                        return Ok(upstream);
                    }
                    Err(_) => {
                        state.write().unwrap().upstream.set_valid(index, false);
                        continue;
                    }
                }
            }
            Err(_) => {
                return Err(ProxyError::NoUpstream);
            }
        }
    }
}

async fn send_response(client_conn: &mut TcpStream, response: &http::Response<Vec<u8>>) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!(
        "{} <- {}",
        client_ip,
        response::format_response_line(&response)
    );
    if let Err(error) = response::write_to_stream(&response, client_conn).await {
        log::warn!("Failed to send response to client: {}", error);
        return;
    }
}

async fn handle_connection(
    mut client_conn: TcpStream,
    state: Arc<RwLock<ProxyState>>,
    request_times: Arc<Mutex<HashMap<String, (usize, std::time::Instant)>>>,
) {
    let max_requests_per_minute = { state.read().unwrap().max_requests_per_minute };
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();

    log::info!("Connection received from {}", client_ip);

    // Open a connection to a random destination server
    let mut upstream_conn = match connect_to_upstream(state.clone()).await {
        Ok(stream) => stream,
        Err(_error) => {
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            return;
        }
    };
    let upstream_ip: String = client_conn.peer_addr().unwrap().ip().to_string();
    {
        let mut lock = request_times.lock().unwrap();
        *lock
            .entry(upstream_ip.clone())
            .or_insert((0, std::time::Instant::now()));
    }
    // The client may now send us one or more requests. Keep trying to read requests until the
    // client hangs up or we get an error.
    loop {
        // Read a request from the client
        let mut request = match request::read_from_stream(&mut client_conn).await {
            Ok(request) => request,
            // Handle case where client closed connection and is no longer sending requests
            Err(request::Error::IncompleteRequest(0)) => {
                log::debug!("Client finished sending requests. Shutting down connection");
                return;
            }
            // Handle I/O error in reading from the client
            Err(request::Error::ConnectionError(io_err)) => {
                log::info!("Error reading request from client stream: {}", io_err);
                return;
            }
            Err(error) => {
                log::debug!("Error parsing request: {:?}", error);
                let response = response::make_http_error(match error {
                    request::Error::IncompleteRequest(_)
                    | request::Error::MalformedRequest(_)
                    | request::Error::InvalidContentLength
                    | request::Error::ContentLengthMismatch => http::StatusCode::BAD_REQUEST,
                    request::Error::RequestBodyTooLarge => http::StatusCode::PAYLOAD_TOO_LARGE,
                    request::Error::ConnectionError(_) => http::StatusCode::SERVICE_UNAVAILABLE,
                });
                send_response(&mut client_conn, &response).await;
                continue;
            }
        };

        let restrict_flag = {
            let lock = &mut request_times.lock().unwrap();
            let (request_times, mut started) = &mut *lock.get_mut(&upstream_ip).unwrap();

            if started.elapsed() > std::time::Duration::from_secs(60) {
                *request_times += 1;
                *request_times = 0;
                started = Instant::now();
            }
            *request_times += 1;
            if max_requests_per_minute != 0 {
                *request_times > max_requests_per_minute
            } else {
                false
            }
        };
        if restrict_flag {
            let response = response::make_http_error(http::StatusCode::TOO_MANY_REQUESTS);
            response::write_to_stream(&response, &mut client_conn).await;
            continue;
        }
        log::info!(
            "{} -> {}: {}",
            client_ip,
            upstream_ip,
            request::format_request_line(&request)
        );

        // Add X-Forwarded-For header so that the upstream server knows the client's IP address.
        // (We're the ones connecting directly to the upstream server, so without this header, the
        // upstream server will only know our IP, not the client's.)
        request::extend_header_value(&mut request, "x-forwarded-for", &client_ip);
        // Forward the request to the server
        if let Err(error) = request::write_to_stream(&request, &mut upstream_conn).await {
            log::error!(
                "Failed to send request to upstream {}: {}",
                upstream_ip,
                error
            );
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response).await;
            return;
        }
        log::debug!("Forwarded request to server");

        // Read the server's response
        let response = match response::read_from_stream(&mut upstream_conn, request.method()).await
        {
            Ok(response) => response,
            Err(error) => {
                log::error!("Error reading response from server: {:?}", error);
                let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
                send_response(&mut client_conn, &response).await;
                return;
            }
        };
        // Forward the response to the client
        send_response(&mut client_conn, &response).await;
        log::debug!("Forwarded response to client");
    }
}
