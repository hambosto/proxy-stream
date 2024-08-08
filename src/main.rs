use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncWriteExt, copy_bidirectional};
use std::error::Error;
use clap::Parser;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use tokio::sync::Semaphore;
use tracing::{info, error};
use std::net::SocketAddr;

/// Command line arguments for the TCP proxy
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port number on which the proxy will listen
    #[arg(long, default_value_t = 8000)]
    listen_port: u16,

    /// Destination port to which traffic will be forwarded
    #[arg(long, default_value_t = 110)]
    destination_port: u16,

    /// Destination host to which traffic will be forwarded
    #[arg(long, default_value = "127.0.0.1")]
    destination_host: String,

    /// Timeout in seconds for idle connections
    #[arg(long, default_value_t = 30)]
    timeout_seconds: u64,

    /// Maximum number of concurrent connections
    #[arg(long, default_value_t = 1000)]
    max_connections: usize,
}

/// Main function to run the TCP proxy
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args = Arc::new(Args::parse());

    // Start the TCP proxy server
    start_proxy(args).await?;

    Ok(())
}

/// Starts the TCP proxy server
///
/// # Arguments
///
/// * `args` - Reference to the command line arguments
///
/// # Returns
///
/// A Result indicating success or containing an error
async fn start_proxy(args: Arc<Args>) -> Result<(), Box<dyn Error>> {
    let addr = format!("0.0.0.0:{}", args.listen_port);
    let listener = TcpListener::bind(&addr).await?;
    let semaphore = Arc::new(Semaphore::new(args.max_connections));

    info!("Server started on port: {}", args.listen_port);
    info!(
        "Redirecting requests to: {} at port {}",
        args.destination_host, args.destination_port
    );

    loop {
        let (inbound, peer_addr) = listener.accept().await?;
        let args = Arc::clone(&args);
        let permit = Arc::clone(&semaphore).acquire_owned().await?;

        tokio::spawn(async move {
            if let Err(e) = handle_client(inbound, args, peer_addr).await {
                error!("Connection handling error for {}: {}", peer_addr, e);
            }
            drop(permit); // Release the semaphore permit when done
        });
    }
}

/// Handles an individual client connection
///
/// # Arguments
///
/// * `inbound` - The incoming TCP stream from the client
/// * `args` - Reference to the command line arguments
/// * `peer_addr` - The address of the client
///
/// # Returns
///
/// A Result indicating success or containing an error
async fn handle_client(
    mut inbound: TcpStream,
    args: Arc<Args>,
    peer_addr: SocketAddr,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("Connection received from {}", peer_addr);

    if let Err(e) = send_initial_response(&mut inbound).await {
        error!("Failed to send initial response to {}: {}", peer_addr, e);
        return Err(e);
    }

    match handle_connection(&mut inbound, &args).await {
        Ok(_) => info!("Connection terminated for {}", peer_addr),
        Err(e) => error!("Connection handling error for {}: {}", peer_addr, e),
    }

    Ok(())
}

/// Sends an initial response to the client
///
/// # Arguments
///
/// * `inbound` - The incoming TCP stream from the client
///
/// # Returns
///
/// A Result indicating success or containing an error
async fn send_initial_response(inbound: &mut TcpStream) -> Result<(), Box<dyn Error + Send + Sync>> {
    inbound
        .write_all(b"HTTP/1.1 101 Switching Protocols\r\nContent-Length: 1048576000000\r\n\r\n")
        .await?;
    Ok(())
}

/// Handles bidirectional data transfer between client and server
///
/// # Arguments
///
/// * `inbound` - The incoming TCP stream from the client
/// * `args` - Reference to the command line arguments
///
/// # Returns
///
/// A Result indicating success or containing an error
async fn handle_connection(
    inbound: &mut TcpStream,
    args: &Args,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let peer_addr = inbound.peer_addr()?;
    let mut outbound = TcpStream::connect(format!("{}:{}", args.destination_host, args.destination_port)).await?;

    let timeout_duration = Duration::from_secs(args.timeout_seconds);

    let (bytes_to_server, bytes_to_client) = match timeout(timeout_duration, copy_bidirectional(inbound, &mut outbound)).await {
        Ok(transfer) => transfer?,
        Err(_) => {
            info!("Connection timed out for {}", peer_addr);
            return Ok(());
        }
    };

    info!(
        "Connection closed for {}. Bytes to server: {}, Bytes to client: {}",
        peer_addr, bytes_to_server, bytes_to_client
    );

    Ok(())
}
