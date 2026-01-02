# Tarqeem Makefile
# ترقيم - أول لغة برمجة عربية 
#
# Usage:
#   make              # Build release
#   make install      # Install to ~/.tarqeem
#   make uninstall    # Remove installation
#   make clean        # Clean build artifacts

.PHONY: all build build-debug runtime runtime-rs install uninstall clean test fmt clippy

# Installation directory (override with: make install PREFIX=/custom/path)
PREFIX ?= $(HOME)/.tarqeem

# Detect OS
UNAME_S := $(shell uname -s)

all: build

# Build release binary
build: runtime
	@echo "Building Tarqeem compiler (release)..."
	cargo build --release
	@echo "Build complete: target/release/tarqeem"

# Build debug binary
build-debug: runtime
	@echo "Building Tarqeem compiler (debug)..."
	cargo build
	@echo "Build complete: target/debug/tarqeem"

# Build C runtime library
runtime:
	@echo "Building C runtime..."
	$(MAKE) -C runtime

# Build Rust runtime library (Phase 1+)
runtime-rs:
	@echo "Building Rust runtime..."
	cargo build --release --package tarqeem-runtime
	@echo "Rust runtime built: target/release/libtrq.a"

# Install to PREFIX
install: build
	@echo "Installing Tarqeem to $(PREFIX)..."
	@mkdir -p $(PREFIX)/bin
	@mkdir -p $(PREFIX)/lib
	@mkdir -p $(PREFIX)/stdlib_trq
	@cp target/release/tarqeem $(PREFIX)/bin/
	@chmod +x $(PREFIX)/bin/tarqeem
	@cp runtime/libtrq.a $(PREFIX)/lib/
	@if [ -f runtime/wasm/libtrq_wasm.a ]; then cp runtime/wasm/libtrq_wasm.a $(PREFIX)/lib/; fi
	@cp -r stdlib_trq/* $(PREFIX)/stdlib_trq/
	@grep -m1 'version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/' > $(PREFIX)/VERSION
	@echo ""
	@echo "Installation complete!"
	@echo ""
	@echo "Add to your shell profile:"
	@echo ""
	@echo '  export TARQEEM_HOME="$(PREFIX)"'
	@echo '  export PATH="$$TARQEEM_HOME/bin:$$PATH"'
	@echo ""

# Uninstall
uninstall:
	@echo "Removing Tarqeem from $(PREFIX)..."
	@rm -rf $(PREFIX)
	@echo "Uninstall complete."
	@echo ""
	@echo "Remember to remove TARQEEM_HOME and PATH entries from your shell profile."

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	$(MAKE) -C runtime clean
	cargo clean
	@echo "Clean complete."

# Run tests
test:
	@echo "Running tests..."
	cargo test

# Format code
fmt:
	@echo "Formatting code..."
	cargo fmt

# Run clippy
clippy:
	@echo "Running clippy..."
	cargo clippy -- -D warnings

# Check everything
check: fmt clippy test
	@echo "All checks passed!"
