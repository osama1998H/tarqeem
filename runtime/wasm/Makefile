# Tarqeem WebAssembly Runtime Library Makefile
#
# Builds the WASM runtime library for Tarqeem programs compiled to WebAssembly.
# Requires clang with WebAssembly target support.

# Compiler settings
CC ?= clang
AR ?= llvm-ar

# WASM target triple
WASM_TARGET = wasm32-unknown-unknown

# Compiler flags for WASM
CFLAGS = -Wall -Wextra -std=c11 -O2
CFLAGS += --target=$(WASM_TARGET)
CFLAGS += -nostdlib
CFLAGS += -fno-builtin
CFLAGS += -ffreestanding
CFLAGS += -fvisibility=hidden
CFLAGS += -D__wasm__

# Debug build
ifdef DEBUG
    CFLAGS += -g -O0 -DDEBUG
endif

# Source files
SRCS = runtime_wasm.c
OBJS = $(SRCS:.c=.o)

# Output
LIB = libtrq_wasm.a

# Default target
all: $(LIB)

# Static library
$(LIB): $(OBJS)
	$(AR) rcs $@ $^
	@echo "Built WASM runtime library: $@"

# Object files
%.o: %.c tarqeem_wasm.h
	$(CC) $(CFLAGS) -c $< -o $@

# Clean
clean:
	rm -f $(OBJS) $(LIB)

# Check if WASM compilation is available
check:
	@echo "Checking for WASM support..."
	@$(CC) --version | head -1
	@$(CC) --print-targets 2>/dev/null | grep -q wasm && \
		echo "WASM target: OK" || \
		echo "WASM target: NOT FOUND (install LLVM with WebAssembly support)"
	@which wasm-ld >/dev/null 2>&1 && \
		echo "wasm-ld: OK" || \
		echo "wasm-ld: NOT FOUND"

# Help
help:
	@echo "Tarqeem WASM Runtime Build System"
	@echo ""
	@echo "Targets:"
	@echo "  all     - Build the WASM runtime library (default)"
	@echo "  clean   - Remove built files"
	@echo "  check   - Check for WASM compilation support"
	@echo "  help    - Show this help message"
	@echo ""
	@echo "Variables:"
	@echo "  CC      - C compiler (default: clang)"
	@echo "  AR      - Archiver (default: llvm-ar)"
	@echo "  DEBUG   - Enable debug build (set to any value)"

.PHONY: all clean check help
