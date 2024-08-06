# Define variables
BINARY_NAME = proxy-stream
INSTALL_DIR = /usr/local/bin

# Build the project
build:
	cargo build --release

# Install the binary to /usr/local/bin
install: build
	cp target/release/$(BINARY_NAME) $(INSTALL_DIR)/

# Uninstall the binary from /usr/local/bin
uninstall:
	rm -f $(INSTALL_DIR)/$(BINARY_NAME)

# Run the project
run: build
	./target/release/$(BINARY_NAME)

# Clean up build artifacts
clean:
	cargo clean
