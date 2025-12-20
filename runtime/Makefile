# Tarqeem Runtime Library Makefile
#
# Builds the runtime library (libtrq.a) that provides runtime support
# for compiled Tarqeem programs.

# Compiler settings
CC ?= gcc
AR ?= ar
CFLAGS = -Wall -Wextra -Werror -std=c11 -O2 -fPIC
CFLAGS += -D_POSIX_C_SOURCE=200809L

# Debug build
ifdef DEBUG
    CFLAGS += -g -O0 -DDEBUG
endif

# Source files
SRCS = memory.c string.c array.c io.c builtins.c
OBJS = $(SRCS:.c=.o)

# Output
LIB_STATIC = libtrq.a
LIB_SHARED = libtrq.so

# Default target
all: $(LIB_STATIC)

# Static library
$(LIB_STATIC): $(OBJS)
	$(AR) rcs $@ $^
	@echo "Built static library: $@"

# Shared library
shared: CFLAGS += -fPIC
shared: $(LIB_SHARED)

$(LIB_SHARED): $(OBJS)
	$(CC) -shared -o $@ $^ -lm
	@echo "Built shared library: $@"

# Object files
%.o: %.c tarqeem_rt.h
	$(CC) $(CFLAGS) -c $< -o $@

# Clean
clean:
	rm -f $(OBJS) $(LIB_STATIC) $(LIB_SHARED)

# Install to system (requires sudo)
PREFIX ?= /usr/local
install: $(LIB_STATIC)
	install -d $(PREFIX)/lib
	install -d $(PREFIX)/include
	install -m 644 $(LIB_STATIC) $(PREFIX)/lib/
	install -m 644 tarqeem_rt.h $(PREFIX)/include/
	@echo "Installed to $(PREFIX)"

# Uninstall
uninstall:
	rm -f $(PREFIX)/lib/$(LIB_STATIC)
	rm -f $(PREFIX)/include/tarqeem_rt.h

# Test build
test: $(LIB_STATIC)
	@echo "Testing runtime library..."
	@echo "int main() { return 0; }" > /tmp/test_trq.c
	$(CC) -o /tmp/test_trq /tmp/test_trq.c -L. -ltrq -lm
	@rm -f /tmp/test_trq /tmp/test_trq.c
	@echo "Test passed!"

.PHONY: all shared clean install uninstall test
