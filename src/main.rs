use std::env;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::thread;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut target_host = String::from("127.0.0.1");
    let mut target_port = String::from("109");
    let mut listen_port = String::from("30001");
    let mut packets_to_skip = 0;

    // Parse command line arguments
    for i in 0..args.len() {
        match args[i].as_str() {
            "-skip" => packets_to_skip = args[i + 1].parse().unwrap_or(0),
            "--target-host" => target_host = args[i + 1].clone(),
            "--target-port" => target_port = args[i + 1].clone(),
            "--listen-port" => listen_port = args[i + 1].clone(),
            _ => {}
        }
    }

    println!("[INFO] - Server started on port: {}", listen_port);
    println!("[INFO] - Redirecting requests to: {} at port {}", target_host, target_port);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", listen_port))?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let target_host = target_host.clone();
                let target_port = target_port.clone();
                let packets_to_skip = packets_to_skip;

                thread::spawn(move || {
                    handle_client(stream, &target_host, &target_port, packets_to_skip).unwrap_or_else(|error| {
                        eprintln!("[ERROR] - Failed to handle client: {}", error);
                    });
                });
            }
            Err(e) => {
                eprintln!("[ERROR] - Failed to accept connection: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_client(mut client: TcpStream, target_host: &str, target_port: &str, packets_to_skip: usize) -> std::io::Result<()> {
    let client_addr = client.peer_addr()?;
    println!("[INFO] - Connection received from {}:{}", client_addr.ip(), client_addr.port());

    client.write_all(b"HTTP/1.1 101 Switching Protocols\r\nContent-Length: 1048576000000\r\n\r\n")?;

    let mut server = TcpStream::connect(format!("{}:{}", target_host, target_port))?;

    let mut packet_count = 0;
    let mut client_clone = client.try_clone()?;
    let mut server_clone = server.try_clone()?;

    let client_to_server = thread::spawn(move || {
        let mut buffer = [0; 4096];
        loop {
            match client_clone.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    if packet_count < packets_to_skip {
                        packet_count += 1;
                    } else if packet_count == packets_to_skip {
                        if let Err(e) = server_clone.write_all(&buffer[..n]) {
                            eprintln!("[ERROR] - Failed to write to server: {}", e);
                            break;
                        }
                    }
                    if packet_count > packets_to_skip {
                        packet_count = packets_to_skip;
                    }
                }
                Err(e) => {
                    eprintln!("[ERROR] - Failed to read from client: {}", e);
                    break;
                }
            }
        }
    });

    let server_to_client = thread::spawn(move || {
        let mut buffer = [0; 4096];
        loop {
            match server.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    if let Err(e) = client.write_all(&buffer[..n]) {
                        eprintln!("[ERROR] - Failed to write to client: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("[ERROR] - Failed to read from server: {}", e);
                    break;
                }
            }
        }
    });

    client_to_server.join().unwrap();
    server_to_client.join().unwrap();

    println!("[INFO] - Connection terminated for {}:{}", client_addr.ip(), client_addr.port());

    Ok(())
}