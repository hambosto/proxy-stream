use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use clap::Parser;
use std::sync::Arc;

/// Struct representing command-line arguments parsed using `clap`.
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The target host to which the incoming requests will be forwarded.
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    target_host: String,

    /// The port on the target host to which the incoming requests will be forwarded.
    #[arg(short = 'p', long, default_value = "8080")]
    target_port: u16,

    /// The port on which the server will listen for incoming connections.
    #[arg(short = 'm', long, default_value = "8888")]
    listen_port: u16,

    /// The number of packets to skip before starting to forward data to the target server.
    #[arg(short, long, default_value = "0")]
    skip: usize,
}

/// The main function, which serves as the entry point to the application.
///
/// This function initializes the server, binds it to the specified listen port,
/// and enters an infinite loop where it accepts and handles incoming connections.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments and wrap them in an `Arc` for shared ownership across threads.
    let args: Arc<Args> = Arc::new(Args::parse());

    // Print startup information.
    println!("[INFO] - Server started on port: {}", args.listen_port);
    println!("[INFO] - Redirecting requests to: {} at port {}", args.target_host, args.target_port);

    // Bind the listener to the specified `listen_port` to accept incoming TCP connections.
    let listener = TcpListener::bind(format!("0.0.0.0:{}", args.listen_port)).await?;

    // Enter an infinite loop to accept incoming connections.
    loop {
        // Accept a new client connection.
        let (client, _) = listener.accept().await?;
        let args: Arc<Args> = Arc::clone(&args);

        // Spawn a new task to handle the client connection.
        tokio::spawn(async move {
            // If handling the client fails, print an error message.
            if let Err(e) = handle_client(client, args).await {
                eprintln!("[ERROR] - Failed to handle client: {}", e);
            }
        });
    }
}

/// Handles an individual client connection.
///
/// This function manages the data transfer between the client and the target server.
/// It splits both the client and server connections into read and write halves
/// to allow concurrent reading from and writing to the connections.
async fn handle_client(mut client: TcpStream, args: Arc<Args>) -> Result<(), Box<dyn std::error::Error>> {
    // Get the client's address for logging purposes.
    let client_addr = client.peer_addr()?;
    println!("[INFO] - Connection received from {}:{}", client_addr.ip(), client_addr.port());

    // Send an initial HTTP response header to the client.
    // This can be useful for WebSocket or similar protocol upgrades.
    client.write_all(b"HTTP/1.1 101 Switching Protocols\r\nContent-Length: 1048576000000\r\n\r\n").await?;

    // Establish a connection to the target server.
    let server = TcpStream::connect(format!("{}:{}", args.target_host, args.target_port)).await?;

    // Split the client and server connections into read and write halves
    // to allow simultaneous reading and writing.
    let (mut client_read, mut client_write) = client.into_split();
    let (mut server_read, mut server_write) = server.into_split();

    // Clone the arguments to pass to the client-to-server forwarding task.
    let args_clone: Arc<Args> = Arc::clone(&args);

    // Spawn a task to handle data forwarding from the client to the server.
    let client_to_server: tokio::task::JoinHandle<()> = tokio::spawn(async move {
        let mut buffer: [u8; 4096] = [0; 4096]; // Buffer for reading data.
        let mut packet_count: usize = 0; // Counter for the number of packets processed.

        loop {
            match client_read.read(&mut buffer).await {
                // End of stream: break the loop.
                Ok(0) => break,
                // Read data from the client.
                Ok(n) => {
                    // Skip packets based on the `skip` argument.
                    if packet_count < args_clone.skip {
                        packet_count += 1;
                    } else if packet_count == args_clone.skip {
                        // Forward the packet to the server.
                        if let Err(e) = server_write.write_all(&buffer[..n]).await {
                            eprintln!("[ERROR] - Failed to write to server: {}", e);
                            break;
                        }
                    }
                    // Reset the packet count to avoid unnecessary increments.
                    if packet_count > args_clone.skip {
                        packet_count = args_clone.skip;
                    }
                }
                // If reading from the client fails, log the error and break the loop.
                Err(e) => {
                    eprintln!("[ERROR] - Failed to read from client: {}", e);
                    break;
                }
            }
        }
    });

    // Spawn a task to handle data forwarding from the server to the client.
    let server_to_client: tokio::task::JoinHandle<()> = tokio::spawn(async move {
        let mut buffer: [u8; 4096] = [0; 4096]; // Buffer for reading data.

        loop {
            match server_read.read(&mut buffer).await {
                // End of stream: break the loop.
                Ok(0) => break,
                // Read data from the server.
                Ok(n) => {
                    // Forward the packet to the client.
                    if let Err(e) = client_write.write_all(&buffer[..n]).await {
                        eprintln!("[ERROR] - Failed to write to client: {}", e);
                        break;
                    }
                }
                // If reading from the server fails, log the error and break the loop.
                Err(e) => {
                    eprintln!("[ERROR] - Failed to read from server: {}", e);
                    break;
                }
            }
        }
    });

    // Wait for both data forwarding tasks to complete.
    tokio::try_join!(client_to_server, server_to_client)?;

    // Log the termination of the connection.
    println!("[INFO] - Connection terminated for {}:{}", client_addr.ip(), client_addr.port());

    // Return Ok to indicate the connection was handled successfully.
    Ok(())
}
