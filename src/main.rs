use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncWriteExt;
use std::error::Error;
use clap::Parser;

/// Command-line arguments for the proxy server.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port on which the proxy server listens.
    #[arg(long, default_value_t = 8888)]
    listen_port: u16,

    /// Port to which the incoming connections are redirected.
    #[arg(long, default_value_t = 110)]
    destination_port: u16,

    /// Host to which the incoming connections are redirected.
    #[arg(long, default_value = "127.0.0.1")]
    destination_host: String,
}

/// Entry point of the application.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Format the address to bind the listener to.
    let addr = format!("0.0.0.0:{}", args.listen_port);
    let listener = TcpListener::bind(&addr).await?;
    println!("[INFO] - Server started on port: {}", args.listen_port);
    println!("[INFO] - Redirecting requests to: {} at port {}", args.destination_host, args.destination_port);

    // Accept incoming connections in a loop.
    while let Ok((inbound, _)) = listener.accept().await {
        let destination_host = args.destination_host.clone();
        let destination_port = args.destination_port;
        
        // Spawn a new task to handle each connection.
        tokio::spawn(async move {
            if let Err(e) = handle_connection(inbound, &destination_host, destination_port).await {
                eprintln!("[ERROR] - {}", e);
            }
        });
    }

    Ok(())
}

/// Handles a single TCP connection, forwarding data between the client and destination server.
async fn handle_connection(mut inbound: TcpStream, destination_host: &str, destination_port: u16) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Log the address of the client that connected.
    let peer_addr = inbound.peer_addr()?;
    println!("[INFO] - Connection received from {}", peer_addr);

    // Send an HTTP response to initiate protocol switching.
    inbound.write_all(b"HTTP/1.1 101 Switching Protocols\r\nContent-Length: 1048576000000\r\n\r\n").await?;

    // Connect to the destination server.
    let mut outbound = TcpStream::connect(format!("{}:{}", destination_host, destination_port)).await?;

    // Split the inbound and outbound streams into readers and writers.
    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    // Create tasks to copy data between the client and server.
    let client_to_server = async {
        tokio::io::copy(&mut ri, &mut wo).await?;
        wo.shutdown().await
    };

    let server_to_client = async {
        tokio::io::copy(&mut ro, &mut wi).await?;
        wi.shutdown().await
    };

    // Run both tasks concurrently and wait for them to complete.
    tokio::try_join!(client_to_server, server_to_client)?;

    // Log when the connection is terminated.
    println!("[INFO] - Connection terminated for {}", peer_addr);

    Ok(())
}
