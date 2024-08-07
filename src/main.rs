use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncWriteExt, copy_bidirectional};
use std::error::Error;
use clap::Parser;
use std::sync::Arc;
use tokio::time::{timeout, Duration};

/// Command line arguments for the TCP proxy
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port number on which the proxy will listen
    #[arg(long, default_value_t = 8888)]
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
}

/// Main function to run the TCP proxy
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse command line arguments
    let args = Arc::new(Args::parse());

    // Construct the address to bind the listener
    let addr = format!("0.0.0.0:{}", args.listen_port);
    let listener = TcpListener::bind(&addr).await?;
    
    println!("[INFO] - Server started on port: {}", args.listen_port);
    println!("[INFO] - Redirecting requests to: {} at port {}", args.destination_host, args.destination_port);

    // Main server loop
    loop {
        // Wait for a client to connect
        let (inbound, _) = listener.accept().await?;
        let args = Arc::clone(&args);
        
        // Spawn a new task to handle the connection
        tokio::spawn(async move {
            if let Err(e) = handle_connection(inbound, &args).await {
                eprintln!("[ERROR] - {}", e);
            }
        });
    }
}

/// Handles an individual client connection
///
/// # Arguments
///
/// * `inbound` - The incoming TCP stream from the client
/// * `args` - Reference to the command line arguments
///
/// # Returns
///
/// A Result indicating success or containing an error
async fn handle_connection(mut inbound: TcpStream, args: &Args) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Get the peer address for logging
    let peer_addr = inbound.peer_addr()?;
    println!("[INFO] - Connection received from {}", peer_addr);

    // Send initial response to the client
    inbound.write_all(b"HTTP/1.1 101 Switching Protocols\r\nContent-Length: 1048576000000\r\n\r\n").await?;

    // Connect to the destination server
    let mut outbound = TcpStream::connect(format!("{}:{}", args.destination_host, args.destination_port)).await?;

    // Set up the timeout duration
    let timeout_duration = Duration::from_secs(args.timeout_seconds);

    // Use copy_bidirectional with a timeout to handle data transfer
    let (bytes_copied_to_server, bytes_copied_to_client) = match timeout(
        timeout_duration,
        copy_bidirectional(&mut inbound, &mut outbound)
    ).await {
        Ok(result) => result?,
        Err(_) => {
            println!("[INFO] - Connection timed out for {}", peer_addr);
            return Ok(());
        }
    };

    // Log connection termination and bytes transferred
    println!("[INFO] - Connection terminated for {}. Bytes to server: {}, Bytes to client: {}", 
             peer_addr, bytes_copied_to_server, bytes_copied_to_client);

    Ok(())
}