# proxy-stream

A lightweight, high-performance TCP proxy server written in Rust using the Tokio asynchronous runtime.

## Features

- Configurable listen port
- Customizable destination host and port
- Asynchronous handling of multiple connections
- Simple command-line interface

## Usage

```
proxy-stream [OPTIONS]
```

Options:
- `--listen-port <PORT>`: Set the listening port (default: 8888)
- `--destination-port <PORT>`: Set the destination port (default: 110)
- `--destination-host <HOST>`: Set the destination host (default: 127.0.0.1)

## Building

To build the project, ensure you have Rust and Cargo installed, then run:

```
cargo build --release
```

## Running

After building, you can run the proxy server with:

```
./target/release/proxy-stream
```

Or with custom options:

```
./target/release/proxy-stream --listen-port 9000 --destination-port 80 --destination-host 127.0.0.1
```

## Dependencies

- tokio
- clap

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.