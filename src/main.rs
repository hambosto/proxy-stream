use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncWriteExt;
use std::error::Error;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value_t = 8888)]
    listen_port: u16,

    #[arg(long, default_value_t = 110)]
    destination_port: u16,

    #[arg(long, default_value = "127.0.0.1")]
    destination_host: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let addr = format!("0.0.0.0:{}", args.listen_port);
    let listener = TcpListener::bind(&addr).await?;
    println!("[INFO] - Server started on port: {}", args.listen_port);
    println!("[INFO] - Redirecting requests to: {} at port {}", args.destination_host, args.destination_port);

    while let Ok((inbound, _)) = listener.accept().await {
        let destination_host = args.destination_host.clone();
        let destination_port = args.destination_port;
        tokio::spawn(async move {
            if let Err(e) = handle_connection(inbound, &destination_host, destination_port).await {
                eprintln!("[ERROR] - {}", e);
            }
        });
    }

    Ok(())
}

async fn handle_connection(mut inbound: TcpStream, destination_host: &str, destination_port: u16) -> Result<(), Box<dyn Error + Send + Sync>> {
    let peer_addr = inbound.peer_addr()?;
    println!("[INFO] - Connection received from {}", peer_addr);

    inbound.write_all(b"HTTP/1.1 101 Switching Protocols\r\nContent-Length: 1048576000000\r\n\r\n").await?;

    let mut outbound = TcpStream::connect(format!("{}:{}", destination_host, destination_port)).await?;

    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    let client_to_server = async {
        tokio::io::copy(&mut ri, &mut wo).await?;
        wo.shutdown().await
    };

    let server_to_client = async {
        tokio::io::copy(&mut ro, &mut wi).await?;
        wi.shutdown().await
    };

    tokio::try_join!(client_to_server, server_to_client)?;

    println!("[INFO] - Connection terminated for {}", peer_addr);

    Ok(())
}