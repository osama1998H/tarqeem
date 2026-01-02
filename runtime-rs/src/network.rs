//! Network Operations for Tarqeem Runtime
//!
//! This module implements TCP, UDP, HTTP, DNS, and URL encoding operations.
//!
//! # ABI Compatibility
//!
//! All exported functions use the C calling convention (`extern "C"`) and are
//! marked with `#[no_mangle]` to ensure they can be linked with compiled
//! Tarqeem programs, JIT-compiled code, and the interpreter.
//!
//! # Handle Management
//!
//! Network handles are managed using thread-local storage with separate
//! HashMaps for TCP streams, TCP listeners, and UDP sockets. A single
//! atomic counter generates unique handle IDs across all socket types.

use crate::string::trq_string_new;
use crate::types::{TrqArray, TrqHttpResponse, TrqString, TrqTcpInfo};

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

// ============================================================================
// Handle Management
// ============================================================================

static NEXT_NETWORK_HANDLE: AtomicI64 = AtomicI64::new(1);

struct TcpStreamWrapper {
    stream: TcpStream,
}

struct TcpListenerWrapper {
    listener: TcpListener,
}

struct UdpSocketWrapper {
    socket: UdpSocket,
    last_sender: Option<SocketAddr>,
}

thread_local! {
    static TCP_STREAMS: RefCell<HashMap<i64, TcpStreamWrapper>> = RefCell::new(HashMap::new());
    static TCP_LISTENERS: RefCell<HashMap<i64, TcpListenerWrapper>> = RefCell::new(HashMap::new());
    static UDP_SOCKETS: RefCell<HashMap<i64, UdpSocketWrapper>> = RefCell::new(HashMap::new());
}

fn get_next_handle() -> i64 {
    NEXT_NETWORK_HANDLE.fetch_add(1, Ordering::SeqCst)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract a Rust String from a TrqString pointer
fn extract_string(s: *const TrqString) -> Option<String> {
    if s.is_null() {
        return None;
    }
    unsafe {
        if (*s).data.is_null() || (*s).len <= 0 {
            return Some(String::new());
        }
        let slice = std::slice::from_raw_parts((*s).data, (*s).len as usize);
        String::from_utf8(slice.to_vec()).ok()
    }
}

/// Convert milliseconds to Duration, None for infinite/invalid
fn duration_from_ms(timeout_ms: i64) -> Option<Duration> {
    if timeout_ms <= 0 {
        None
    } else {
        Some(Duration::from_millis(timeout_ms as u64))
    }
}

/// Create a TrqString from a Rust string
fn string_to_trq(s: &str) -> *mut TrqString {
    let bytes = s.as_bytes();
    trq_string_new(bytes.as_ptr(), bytes.len() as i64)
}

/// Create an empty TrqString
fn empty_string() -> *mut TrqString {
    trq_string_new(std::ptr::null(), 0)
}

// ============================================================================
// TCP Functions (13)
// ============================================================================

/// Connect to a TCP server with timeout.
///
/// # Arguments
/// * `address` - Server address (hostname or IP)
/// * `port` - Server port (1-65535)
/// * `timeout_ms` - Connection timeout in milliseconds (0 or negative for blocking)
///
/// # Returns
/// Socket handle on success, -1 on error.
#[no_mangle]
pub extern "C" fn trq_tcp_connect(address: *const TrqString, port: i64, timeout_ms: i64) -> i64 {
    let addr_str = match extract_string(address) {
        Some(s) => s,
        None => return -1,
    };

    if port < 1 || port > 65535 {
        return -1;
    }

    let addr = format!("{}:{}", addr_str, port);

    // Resolve address
    let socket_addr = match addr.to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(a) => a,
            None => return -1,
        },
        Err(_) => return -1,
    };

    // Connect with or without timeout
    let stream = if let Some(timeout) = duration_from_ms(timeout_ms) {
        match TcpStream::connect_timeout(&socket_addr, timeout) {
            Ok(s) => s,
            Err(_) => return -1,
        }
    } else {
        match TcpStream::connect(&socket_addr) {
            Ok(s) => s,
            Err(_) => return -1,
        }
    };

    // Store and return handle
    let handle = get_next_handle();
    TCP_STREAMS.with(|streams| {
        streams
            .borrow_mut()
            .insert(handle, TcpStreamWrapper { stream });
    });
    handle
}

/// Close a TCP connection or listener.
#[no_mangle]
pub extern "C" fn trq_tcp_close(handle: i64) {
    // Try removing from streams first
    let removed_stream = TCP_STREAMS.with(|streams| streams.borrow_mut().remove(&handle));
    if removed_stream.is_some() {
        return;
    }

    // Try removing from listeners
    TCP_LISTENERS.with(|listeners| {
        listeners.borrow_mut().remove(&handle);
    });
}

/// Send string data over TCP.
///
/// # Returns
/// `true` on success, `false` on error.
#[no_mangle]
pub extern "C" fn trq_tcp_send(handle: i64, data: *const TrqString) -> bool {
    if data.is_null() {
        return false;
    }

    let data_slice = unsafe {
        if (*data).data.is_null() || (*data).len <= 0 {
            return true; // Empty data is success
        }
        std::slice::from_raw_parts((*data).data, (*data).len as usize)
    };

    TCP_STREAMS.with(|streams| {
        let mut streams = streams.borrow_mut();
        if let Some(wrapper) = streams.get_mut(&handle) {
            wrapper.stream.write_all(data_slice).is_ok()
        } else {
            false
        }
    })
}

/// Send byte array over TCP.
///
/// # Returns
/// `true` on success, `false` on error.
#[no_mangle]
pub extern "C" fn trq_tcp_send_bytes(handle: i64, data: *const TrqArray) -> bool {
    if data.is_null() {
        return false;
    }

    let data_slice = unsafe {
        if (*data).data.is_null() || (*data).len <= 0 {
            return true; // Empty data is success
        }
        std::slice::from_raw_parts((*data).data, (*data).len as usize)
    };

    TCP_STREAMS.with(|streams| {
        let mut streams = streams.borrow_mut();
        if let Some(wrapper) = streams.get_mut(&handle) {
            wrapper.stream.write_all(data_slice).is_ok()
        } else {
            false
        }
    })
}

/// Receive string data from TCP with timeout.
///
/// # Arguments
/// * `handle` - Socket handle
/// * `timeout_ms` - Read timeout in milliseconds (0 or negative for blocking)
///
/// # Returns
/// Received data as TrqString, or empty string on error/timeout.
#[no_mangle]
pub extern "C" fn trq_tcp_receive(handle: i64, timeout_ms: i64) -> *mut TrqString {
    TCP_STREAMS.with(|streams| {
        let mut streams = streams.borrow_mut();
        if let Some(wrapper) = streams.get_mut(&handle) {
            // Set timeout
            let timeout = duration_from_ms(timeout_ms);
            if wrapper.stream.set_read_timeout(timeout).is_err() {
                return empty_string();
            }

            // Read into buffer (8KB like C version)
            let mut buffer = vec![0u8; 8192];
            match wrapper.stream.read(&mut buffer) {
                Ok(0) => empty_string(), // Connection closed
                Ok(n) => trq_string_new(buffer.as_ptr(), n as i64),
                Err(_) => empty_string(),
            }
        } else {
            empty_string()
        }
    })
}

/// Receive exact number of bytes from TCP.
///
/// # Returns
/// Received data as TrqArray, or null on error.
#[no_mangle]
pub extern "C" fn trq_tcp_receive_bytes(handle: i64, size: i64, timeout_ms: i64) -> *mut TrqArray {
    if size <= 0 {
        return std::ptr::null_mut();
    }

    TCP_STREAMS.with(|streams| {
        let mut streams = streams.borrow_mut();
        if let Some(wrapper) = streams.get_mut(&handle) {
            // Set timeout
            let timeout = duration_from_ms(timeout_ms);
            if wrapper.stream.set_read_timeout(timeout).is_err() {
                return std::ptr::null_mut();
            }

            // Read exact bytes
            let mut buffer = vec![0u8; size as usize];
            match wrapper.stream.read_exact(&mut buffer) {
                Ok(()) => {
                    // Create array - allocate via our memory system
                    let arr = unsafe {
                        let arr_ptr =
                            crate::memory::trq_alloc(std::mem::size_of::<TrqArray>() as i64)
                                as *mut TrqArray;
                        if arr_ptr.is_null() {
                            return std::ptr::null_mut();
                        }

                        let data_ptr = libc::malloc(size as usize) as *mut u8;
                        if data_ptr.is_null() {
                            crate::memory::trq_free(arr_ptr as *mut u8);
                            return std::ptr::null_mut();
                        }

                        std::ptr::copy_nonoverlapping(buffer.as_ptr(), data_ptr, size as usize);

                        (*arr_ptr).len = size;
                        (*arr_ptr).cap = size;
                        (*arr_ptr).elem_size = 1;
                        (*arr_ptr).data = data_ptr;

                        arr_ptr
                    };
                    arr
                }
                Err(_) => std::ptr::null_mut(),
            }
        } else {
            std::ptr::null_mut()
        }
    })
}

/// Receive data until delimiter is found.
///
/// # Returns
/// Data up to (but not including) delimiter, or empty string on error.
#[no_mangle]
pub extern "C" fn trq_tcp_receive_until(
    handle: i64,
    delimiter: *const TrqString,
    timeout_ms: i64,
) -> *mut TrqString {
    let delim = match extract_string(delimiter) {
        Some(d) if !d.is_empty() => d,
        _ => return empty_string(),
    };

    TCP_STREAMS.with(|streams| {
        let mut streams = streams.borrow_mut();
        if let Some(wrapper) = streams.get_mut(&handle) {
            // Set timeout
            let timeout = duration_from_ms(timeout_ms);
            if wrapper.stream.set_read_timeout(timeout).is_err() {
                return empty_string();
            }

            let delim_bytes = delim.as_bytes();
            let mut buffer = Vec::new();

            // Read byte by byte directly from stream until delimiter found
            // Note: We read directly from wrapper.stream to avoid data loss
            // that would occur with try_clone() (which shares the socket buffer)
            loop {
                let mut byte = [0u8; 1];
                match wrapper.stream.read_exact(&mut byte) {
                    Ok(()) => {
                        buffer.push(byte[0]);
                        // Check if buffer ends with delimiter
                        if buffer.len() >= delim_bytes.len()
                            && &buffer[buffer.len() - delim_bytes.len()..] == delim_bytes
                        {
                            // Remove delimiter from result
                            buffer.truncate(buffer.len() - delim_bytes.len());
                            break;
                        }
                    }
                    Err(_) => {
                        // Return what we have so far
                        break;
                    }
                }
            }

            let result = String::from_utf8_lossy(&buffer);
            string_to_trq(&result)
        } else {
            empty_string()
        }
    })
}

/// Check if data is available to read (non-blocking).
#[no_mangle]
pub extern "C" fn trq_tcp_available(handle: i64) -> bool {
    TCP_STREAMS.with(|streams| {
        let mut streams = streams.borrow_mut();
        if let Some(wrapper) = streams.get_mut(&handle) {
            // Set non-blocking temporarily
            if wrapper.stream.set_nonblocking(true).is_err() {
                return false;
            }

            // Try to peek
            let mut buf = [0u8; 1];
            let available = match wrapper.stream.peek(&mut buf) {
                Ok(n) => n > 0,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => false,
                Err(_) => false,
            };

            // Restore blocking mode
            let _ = wrapper.stream.set_nonblocking(false);

            available
        } else {
            false
        }
    })
}

/// Create a TCP server socket (listen).
///
/// # Arguments
/// * `address` - Bind address (e.g., "0.0.0.0" or "127.0.0.1")
/// * `port` - Bind port
/// * `backlog` - Connection backlog (unused in Rust, kept for API compatibility)
///
/// # Returns
/// Listener handle on success, -1 on error.
#[no_mangle]
pub extern "C" fn trq_tcp_listen(address: *const TrqString, port: i64, _backlog: i64) -> i64 {
    let addr_str = match extract_string(address) {
        Some(s) => s,
        None => return -1,
    };

    if port < 1 || port > 65535 {
        return -1;
    }

    let addr = format!("{}:{}", addr_str, port);

    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(_) => return -1,
    };

    let handle = get_next_handle();
    TCP_LISTENERS.with(|listeners| {
        listeners
            .borrow_mut()
            .insert(handle, TcpListenerWrapper { listener });
    });
    handle
}

/// Accept an incoming TCP connection (blocking).
///
/// # Returns
/// TrqTcpInfo with client info and handle, or null on error.
#[no_mangle]
pub extern "C" fn trq_tcp_accept(handle: i64) -> *mut TrqTcpInfo {
    TCP_LISTENERS.with(|listeners| {
        let listeners = listeners.borrow();
        if let Some(wrapper) = listeners.get(&handle) {
            match wrapper.listener.accept() {
                Ok((stream, addr)) => {
                    // Create TrqTcpInfo first before storing the stream
                    let info_ptr = crate::memory::trq_alloc(std::mem::size_of::<TrqTcpInfo>() as i64)
                        as *mut TrqTcpInfo;
                    if info_ptr.is_null() {
                        // Don't store the stream if allocation fails
                        return std::ptr::null_mut();
                    }

                    // Now store stream (only after successful allocation)
                    let stream_handle = get_next_handle();
                    TCP_STREAMS.with(|streams| {
                        streams
                            .borrow_mut()
                            .insert(stream_handle, TcpStreamWrapper { stream });
                    });

                    // Fill in TrqTcpInfo
                    unsafe {
                        (*info_ptr).address = string_to_trq(&addr.ip().to_string());
                        (*info_ptr).port = addr.port() as i64;
                        (*info_ptr).handle = stream_handle;
                    }

                    info_ptr
                }
                Err(_) => std::ptr::null_mut(),
            }
        } else {
            std::ptr::null_mut()
        }
    })
}

/// Accept an incoming TCP connection with timeout.
///
/// # Returns
/// TrqTcpInfo with handle=-1 on timeout, or client info on success.
#[no_mangle]
pub extern "C" fn trq_tcp_accept_timeout(handle: i64, timeout_ms: i64) -> *mut TrqTcpInfo {
    TCP_LISTENERS.with(|listeners| {
        let listeners = listeners.borrow();
        if let Some(wrapper) = listeners.get(&handle) {
            // Set non-blocking
            if wrapper.listener.set_nonblocking(true).is_err() {
                return std::ptr::null_mut();
            }

            let timeout = duration_from_ms(timeout_ms).unwrap_or(Duration::from_secs(30));
            let start = std::time::Instant::now();

            loop {
                match wrapper.listener.accept() {
                    Ok((stream, addr)) => {
                        // Restore blocking mode
                        let _ = wrapper.listener.set_nonblocking(false);

                        // Create TrqTcpInfo first before storing the stream
                        let info_ptr =
                            crate::memory::trq_alloc(std::mem::size_of::<TrqTcpInfo>() as i64)
                                as *mut TrqTcpInfo;
                        if info_ptr.is_null() {
                            // Don't store the stream if allocation fails
                            return std::ptr::null_mut();
                        }

                        // Now store stream (only after successful allocation)
                        let stream_handle = get_next_handle();
                        TCP_STREAMS.with(|streams| {
                            streams
                                .borrow_mut()
                                .insert(stream_handle, TcpStreamWrapper { stream });
                        });

                        // Fill in TrqTcpInfo
                        unsafe {
                            (*info_ptr).address = string_to_trq(&addr.ip().to_string());
                            (*info_ptr).port = addr.port() as i64;
                            (*info_ptr).handle = stream_handle;
                        }

                        return info_ptr;
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if start.elapsed() >= timeout {
                            // Restore blocking mode
                            let _ = wrapper.listener.set_nonblocking(false);

                            // Return timeout info
                            unsafe {
                                let info_ptr =
                                    crate::memory::trq_alloc(
                                        std::mem::size_of::<TrqTcpInfo>() as i64
                                    ) as *mut TrqTcpInfo;
                                if info_ptr.is_null() {
                                    return std::ptr::null_mut();
                                }

                                (*info_ptr).address = std::ptr::null_mut();
                                (*info_ptr).port = 0;
                                (*info_ptr).handle = -1;

                                return info_ptr;
                            }
                        }
                        // Sleep briefly before retry
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => {
                        let _ = wrapper.listener.set_nonblocking(false);
                        return std::ptr::null_mut();
                    }
                }
            }
        } else {
            std::ptr::null_mut()
        }
    })
}

/// Get local address of a socket.
#[no_mangle]
pub extern "C" fn trq_tcp_local_address(handle: i64) -> *mut TrqString {
    // Try streams first
    let result = TCP_STREAMS.with(|streams| {
        let streams = streams.borrow();
        if let Some(wrapper) = streams.get(&handle) {
            wrapper.stream.local_addr().ok().map(|a| a.ip().to_string())
        } else {
            None
        }
    });

    if let Some(addr) = result {
        return string_to_trq(&addr);
    }

    // Try listeners
    let result = TCP_LISTENERS.with(|listeners| {
        let listeners = listeners.borrow();
        if let Some(wrapper) = listeners.get(&handle) {
            wrapper
                .listener
                .local_addr()
                .ok()
                .map(|a| a.ip().to_string())
        } else {
            None
        }
    });

    match result {
        Some(addr) => string_to_trq(&addr),
        None => empty_string(),
    }
}

/// Get local port of a socket.
#[no_mangle]
pub extern "C" fn trq_tcp_local_port(handle: i64) -> i64 {
    // Try streams first
    let result = TCP_STREAMS.with(|streams| {
        let streams = streams.borrow();
        if let Some(wrapper) = streams.get(&handle) {
            wrapper.stream.local_addr().ok().map(|a| a.port() as i64)
        } else {
            None
        }
    });

    if let Some(port) = result {
        return port;
    }

    // Try listeners
    TCP_LISTENERS.with(|listeners| {
        let listeners = listeners.borrow();
        if let Some(wrapper) = listeners.get(&handle) {
            wrapper
                .listener
                .local_addr()
                .ok()
                .map(|a| a.port() as i64)
                .unwrap_or(-1)
        } else {
            -1
        }
    })
}

// ============================================================================
// UDP Functions (7)
// ============================================================================

/// Bind a UDP socket to a port.
///
/// # Returns
/// Socket handle on success, -1 on error.
#[no_mangle]
pub extern "C" fn trq_udp_bind(port: i64) -> i64 {
    if port < 0 || port > 65535 {
        return -1;
    }

    let addr = format!("0.0.0.0:{}", port);
    let socket = match UdpSocket::bind(&addr) {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let handle = get_next_handle();
    UDP_SOCKETS.with(|sockets| {
        sockets.borrow_mut().insert(
            handle,
            UdpSocketWrapper {
                socket,
                last_sender: None,
            },
        );
    });
    handle
}

/// Close a UDP socket.
#[no_mangle]
pub extern "C" fn trq_udp_close(handle: i64) {
    UDP_SOCKETS.with(|sockets| {
        sockets.borrow_mut().remove(&handle);
    });
}

/// Send a string datagram to an address.
#[no_mangle]
pub extern "C" fn trq_udp_send_to(
    handle: i64,
    address: *const TrqString,
    port: i64,
    data: *const TrqString,
) -> bool {
    let addr_str = match extract_string(address) {
        Some(s) => s,
        None => return false,
    };

    if port < 1 || port > 65535 {
        return false;
    }

    let data_slice = unsafe {
        if data.is_null() || (*data).data.is_null() || (*data).len <= 0 {
            return true; // Empty is success
        }
        std::slice::from_raw_parts((*data).data, (*data).len as usize)
    };

    let dest = format!("{}:{}", addr_str, port);

    UDP_SOCKETS.with(|sockets| {
        let sockets = sockets.borrow();
        if let Some(wrapper) = sockets.get(&handle) {
            wrapper.socket.send_to(data_slice, &dest).is_ok()
        } else {
            false
        }
    })
}

/// Send a byte array datagram to an address.
#[no_mangle]
pub extern "C" fn trq_udp_send_bytes_to(
    handle: i64,
    address: *const TrqString,
    port: i64,
    data: *const TrqArray,
) -> bool {
    let addr_str = match extract_string(address) {
        Some(s) => s,
        None => return false,
    };

    if port < 1 || port > 65535 {
        return false;
    }

    let data_slice = unsafe {
        if data.is_null() || (*data).data.is_null() || (*data).len <= 0 {
            return true; // Empty is success
        }
        std::slice::from_raw_parts((*data).data, (*data).len as usize)
    };

    let dest = format!("{}:{}", addr_str, port);

    UDP_SOCKETS.with(|sockets| {
        let sockets = sockets.borrow();
        if let Some(wrapper) = sockets.get(&handle) {
            wrapper.socket.send_to(data_slice, &dest).is_ok()
        } else {
            false
        }
    })
}

/// Receive a datagram with timeout.
///
/// # Returns
/// Received data as string, empty on timeout/error.
#[no_mangle]
pub extern "C" fn trq_udp_receive(handle: i64, timeout_ms: i64) -> *mut TrqString {
    UDP_SOCKETS.with(|sockets| {
        let mut sockets = sockets.borrow_mut();
        if let Some(wrapper) = sockets.get_mut(&handle) {
            // Set timeout
            let timeout = duration_from_ms(timeout_ms);
            if wrapper.socket.set_read_timeout(timeout).is_err() {
                return empty_string();
            }

            // 64KB buffer for UDP (max datagram size)
            let mut buffer = vec![0u8; 65536];
            match wrapper.socket.recv_from(&mut buffer) {
                Ok((n, addr)) => {
                    wrapper.last_sender = Some(addr);
                    trq_string_new(buffer.as_ptr(), n as i64)
                }
                Err(_) => empty_string(),
            }
        } else {
            empty_string()
        }
    })
}

/// Receive bytes with timeout.
///
/// # Returns
/// Received data as TrqArray, null on error.
#[no_mangle]
pub extern "C" fn trq_udp_receive_bytes(handle: i64, size: i64, timeout_ms: i64) -> *mut TrqArray {
    if size <= 0 {
        return std::ptr::null_mut();
    }

    UDP_SOCKETS.with(|sockets| {
        let mut sockets = sockets.borrow_mut();
        if let Some(wrapper) = sockets.get_mut(&handle) {
            // Set timeout
            let timeout = duration_from_ms(timeout_ms);
            if wrapper.socket.set_read_timeout(timeout).is_err() {
                return std::ptr::null_mut();
            }

            let mut buffer = vec![0u8; size as usize];
            match wrapper.socket.recv_from(&mut buffer) {
                Ok((n, addr)) => {
                    wrapper.last_sender = Some(addr);

                    // Create array
                    unsafe {
                        let arr_ptr =
                            crate::memory::trq_alloc(std::mem::size_of::<TrqArray>() as i64)
                                as *mut TrqArray;
                        if arr_ptr.is_null() {
                            return std::ptr::null_mut();
                        }

                        let data_ptr = libc::malloc(n) as *mut u8;
                        if data_ptr.is_null() {
                            crate::memory::trq_free(arr_ptr as *mut u8);
                            return std::ptr::null_mut();
                        }

                        std::ptr::copy_nonoverlapping(buffer.as_ptr(), data_ptr, n);

                        (*arr_ptr).len = n as i64;
                        (*arr_ptr).cap = n as i64;
                        (*arr_ptr).elem_size = 1;
                        (*arr_ptr).data = data_ptr;

                        arr_ptr
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        } else {
            std::ptr::null_mut()
        }
    })
}

/// Reply to the last sender.
///
/// # Returns
/// `true` on success, `false` if no last sender or error.
#[no_mangle]
pub extern "C" fn trq_udp_reply(handle: i64, data: *const TrqString) -> bool {
    let data_slice = unsafe {
        if data.is_null() || (*data).data.is_null() || (*data).len <= 0 {
            return false;
        }
        std::slice::from_raw_parts((*data).data, (*data).len as usize)
    };

    UDP_SOCKETS.with(|sockets| {
        let sockets = sockets.borrow();
        if let Some(wrapper) = sockets.get(&handle) {
            if let Some(addr) = wrapper.last_sender {
                wrapper.socket.send_to(data_slice, addr).is_ok()
            } else {
                false
            }
        } else {
            false
        }
    })
}

// ============================================================================
// DNS/Utility Functions (2)
// ============================================================================

/// Resolve a hostname to an IPv4 address.
///
/// # Returns
/// IP address as string, empty on error.
#[no_mangle]
pub extern "C" fn trq_resolve_hostname(hostname: *const TrqString) -> *mut TrqString {
    let host = match extract_string(hostname) {
        Some(h) => h,
        None => return empty_string(),
    };

    // Add port to make it a valid socket address for resolution
    let addr = format!("{}:0", host);

    match addr.to_socket_addrs() {
        Ok(mut addrs) => {
            // Find first IPv4 address
            for a in addrs.by_ref() {
                if let SocketAddr::V4(v4) = a {
                    return string_to_trq(&v4.ip().to_string());
                }
            }
            empty_string()
        }
        Err(_) => empty_string(),
    }
}

/// Get the local IP address.
///
/// # Returns
/// Local IP as string, or empty on error.
#[no_mangle]
pub extern "C" fn trq_get_local_ip() -> *mut TrqString {
    // Connect UDP to 8.8.8.8:80 (doesn't actually send data)
    // Then get the local address
    match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => {
            if socket.connect("8.8.8.8:80").is_err() {
                return empty_string();
            }
            match socket.local_addr() {
                Ok(addr) => string_to_trq(&addr.ip().to_string()),
                Err(_) => empty_string(),
            }
        }
        Err(_) => empty_string(),
    }
}

// ============================================================================
// URL Encoding Functions (2)
// ============================================================================

/// URL-encode a string (RFC 3986).
///
/// Unreserved characters (A-Z, a-z, 0-9, -, _, ., ~) are not encoded.
/// All other characters are percent-encoded.
#[no_mangle]
pub extern "C" fn trq_url_encode(s: *const TrqString) -> *mut TrqString {
    let input = match extract_string(s) {
        Some(i) => i,
        None => return empty_string(),
    };

    let mut encoded = String::with_capacity(input.len() * 3);

    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }

    string_to_trq(&encoded)
}

/// URL-decode a string.
///
/// Decodes %XX sequences and + as space.
#[no_mangle]
pub extern "C" fn trq_url_decode(s: *const TrqString) -> *mut TrqString {
    let input = match extract_string(s) {
        Some(i) => i,
        None => return empty_string(),
    };

    let mut decoded = Vec::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '%' => {
                // Try to read two hex digits
                let mut hex = String::new();
                for _ in 0..2 {
                    if let Some(&next) = chars.peek() {
                        if next.is_ascii_hexdigit() {
                            hex.push(chars.next().unwrap());
                        }
                    }
                }
                if hex.len() == 2 {
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        decoded.push(byte);
                    } else {
                        decoded.push(b'%');
                        decoded.extend(hex.bytes());
                    }
                } else {
                    decoded.push(b'%');
                    decoded.extend(hex.bytes());
                }
            }
            '+' => decoded.push(b' '),
            _ => {
                // Properly encode multi-byte UTF-8 characters
                let mut buf = [0u8; 4];
                let encoded = c.encode_utf8(&mut buf);
                decoded.extend_from_slice(encoded.as_bytes());
            }
        }
    }

    let result = String::from_utf8_lossy(&decoded);
    string_to_trq(&result)
}

// ============================================================================
// HTTP Functions (3)
// ============================================================================

/// Internal: Parse a URL into components
struct ParsedUrl {
    host: String,
    port: u16,
    path: String,
    is_https: bool,
}

fn parse_url(url: &str) -> Option<ParsedUrl> {
    let url = url.trim();

    // Check scheme
    let (is_https, rest) = if url.starts_with("https://") {
        (true, &url[8..])
    } else if url.starts_with("http://") {
        (false, &url[7..])
    } else {
        // Assume http
        (false, url)
    };

    // Split host and path
    let (host_port, path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, "/"),
    };

    // Parse host and port, handling IPv6 addresses in brackets
    let (host, port) = if host_port.starts_with('[') {
        // IPv6 address in brackets: [::1]:8080
        match host_port.find(']') {
            Some(bracket_idx) => {
                let h = &host_port[1..bracket_idx]; // Extract IPv6 without brackets
                let after_bracket = &host_port[bracket_idx + 1..];
                if after_bracket.starts_with(':') {
                    let p = after_bracket[1..].parse().ok()?;
                    (h.to_string(), p)
                } else if after_bracket.is_empty() {
                    let default_port = if is_https { 443 } else { 80 };
                    (h.to_string(), default_port)
                } else {
                    return None; // Invalid format
                }
            }
            None => return None, // Missing closing bracket
        }
    } else {
        // Regular IPv4 or hostname
        match host_port.rfind(':') {
            Some(idx) => {
                let h = &host_port[..idx];
                let p = host_port[idx + 1..].parse().ok()?;
                (h.to_string(), p)
            }
            None => {
                let default_port = if is_https { 443 } else { 80 };
                (host_port.to_string(), default_port)
            }
        }
    };

    if host.is_empty() {
        return None;
    }

    Some(ParsedUrl {
        host,
        port,
        path: path.to_string(),
        is_https,
    })
}

/// Perform an HTTP request.
///
/// # Arguments
/// * `method` - HTTP method (GET, POST, etc.)
/// * `url` - Request URL
/// * `headers` - Array of header strings (can be null)
/// * `body` - Request body (can be null)
/// * `timeout_ms` - Timeout in milliseconds
/// * `follow_redirects` - Whether to follow redirects (not implemented)
///
/// # Returns
/// TrqHttpResponse with status, headers, and body.
///
/// # Note
/// HTTPS is NOT supported. Returns error response for HTTPS URLs.
#[no_mangle]
pub extern "C" fn trq_http_request(
    method: *const TrqString,
    url: *const TrqString,
    headers: *const TrqArray,
    body: *const TrqString,
    timeout_ms: i64,
    _follow_redirects: bool,
) -> *mut TrqHttpResponse {
    let method_str = match extract_string(method) {
        Some(m) if !m.is_empty() => m,
        _ => {
            return create_error_response("خطأ: طريقة فارغة");
        }
    };

    let url_str = match extract_string(url) {
        Some(u) if !u.is_empty() => u,
        _ => {
            return create_error_response("خطأ: رابط فارغ");
        }
    };

    let parsed = match parse_url(&url_str) {
        Some(p) => p,
        None => {
            return create_error_response("خطأ: رابط غير صالح");
        }
    };

    // HTTPS not supported
    if parsed.is_https {
        return create_error_response("خطأ: HTTPS غير مدعوم حالياً");
    }

    // Connect
    let addr = format!("{}:{}", parsed.host, parsed.port);
    let mut stream = match TcpStream::connect_timeout(
        &match addr.to_socket_addrs() {
            Ok(mut a) => match a.next() {
                Some(a) => a,
                None => return create_error_response("خطأ: فشل حل العنوان"),
            },
            Err(_) => return create_error_response("خطأ: فشل حل العنوان"),
        },
        duration_from_ms(timeout_ms).unwrap_or(Duration::from_secs(30)),
    ) {
        Ok(s) => s,
        Err(_) => {
            return create_error_response("خطأ: فشل الاتصال بالخادم");
        }
    };

    // Set timeouts
    let timeout = duration_from_ms(timeout_ms);
    let _ = stream.set_read_timeout(timeout);
    let _ = stream.set_write_timeout(timeout);

    // Build request
    let body_str = extract_string(body).unwrap_or_default();
    let mut request = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
        method_str, parsed.path, parsed.host
    );

    // Add custom headers
    if !headers.is_null() {
        unsafe {
            let len = (*headers).len;
            let elem_size = (*headers).elem_size;
            let expected_size = std::mem::size_of::<*const TrqString>() as i64;

            // Validate elem_size matches pointer size and len is positive
            if elem_size == expected_size && len > 0 && !(*headers).data.is_null() {
                for i in 0..len {
                    let ptr_offset = (i * elem_size) as usize;
                    let header_ptr = (*headers).data.add(ptr_offset) as *const *const TrqString;
                    if let Some(header) = extract_string(*header_ptr) {
                        request.push_str(&header);
                        request.push_str("\r\n");
                    }
                }
            }
        }
    }

    // Add content length if body exists
    if !body_str.is_empty() {
        request.push_str(&format!("Content-Length: {}\r\n", body_str.len()));
    }

    request.push_str("\r\n");
    request.push_str(&body_str);

    // Send request
    if stream.write_all(request.as_bytes()).is_err() {
        return create_error_response("خطأ: فشل إرسال الطلب");
    }

    // Read response
    let mut response_data = Vec::new();
    if stream.read_to_end(&mut response_data).is_err() {
        return create_error_response("خطأ: لم يتم استلام رد");
    }

    // Parse response
    parse_http_response(&response_data)
}

/// Create an error HTTP response with Arabic message
fn create_error_response(message: &str) -> *mut TrqHttpResponse {
    unsafe {
        let resp_ptr = crate::memory::trq_alloc(std::mem::size_of::<TrqHttpResponse>() as i64)
            as *mut TrqHttpResponse;
        if resp_ptr.is_null() {
            return std::ptr::null_mut();
        }

        (*resp_ptr).status = -1;
        (*resp_ptr).status_text = string_to_trq(message);
        (*resp_ptr).body = empty_string();
        (*resp_ptr).headers = std::ptr::null_mut();

        resp_ptr
    }
}

/// Parse HTTP response data
fn parse_http_response(data: &[u8]) -> *mut TrqHttpResponse {
    let response_str = String::from_utf8_lossy(data);

    // Find headers/body split
    let (headers_part, body) = match response_str.find("\r\n\r\n") {
        Some(idx) => (&response_str[..idx], &response_str[idx + 4..]),
        None => {
            return create_error_response("خطأ: رد غير صالح");
        }
    };

    // Parse status line
    let mut lines = headers_part.lines();
    let status_line = match lines.next() {
        Some(l) => l,
        None => {
            return create_error_response("خطأ: رد غير صالح");
        }
    };

    // Parse "HTTP/1.1 200 OK"
    let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return create_error_response("خطأ: سطر حالة غير صالح");
    }

    let status_code: i64 = parts[1].parse().unwrap_or(-1);
    let status_text = if parts.len() >= 3 { parts[2] } else { "" };

    // Collect headers
    let header_lines: Vec<&str> = lines.collect();

    unsafe {
        let resp_ptr = crate::memory::trq_alloc(std::mem::size_of::<TrqHttpResponse>() as i64)
            as *mut TrqHttpResponse;
        if resp_ptr.is_null() {
            return std::ptr::null_mut();
        }

        (*resp_ptr).status = status_code;
        (*resp_ptr).status_text = string_to_trq(status_text);
        (*resp_ptr).body = string_to_trq(body);

        // Create headers array
        if !header_lines.is_empty() {
            let arr_ptr =
                crate::memory::trq_alloc(std::mem::size_of::<TrqArray>() as i64) as *mut TrqArray;
            if !arr_ptr.is_null() {
                let ptr_size = std::mem::size_of::<*mut TrqString>();
                let data_ptr = libc::malloc(header_lines.len() * ptr_size) as *mut *mut TrqString;
                if !data_ptr.is_null() {
                    for (i, header) in header_lines.iter().enumerate() {
                        *data_ptr.add(i) = string_to_trq(header);
                    }
                    (*arr_ptr).len = header_lines.len() as i64;
                    (*arr_ptr).cap = header_lines.len() as i64;
                    (*arr_ptr).elem_size = ptr_size as i64;
                    (*arr_ptr).data = data_ptr as *mut u8;
                    (*resp_ptr).headers = arr_ptr;
                } else {
                    crate::memory::trq_free(arr_ptr as *mut u8);
                    (*resp_ptr).headers = std::ptr::null_mut();
                }
            } else {
                (*resp_ptr).headers = std::ptr::null_mut();
            }
        } else {
            (*resp_ptr).headers = std::ptr::null_mut();
        }

        resp_ptr
    }
}

/// Simple HTTP GET request.
///
/// # Returns
/// Response body as string, empty on error.
#[no_mangle]
pub extern "C" fn trq_http_get(url: *const TrqString) -> *mut TrqString {
    let method = string_to_trq("GET");
    let response = trq_http_request(method, url, std::ptr::null(), std::ptr::null(), 30000, true);

    if response.is_null() {
        // Release method string to avoid memory leak
        crate::memory::trq_release(method as *mut u8);
        return empty_string();
    }

    unsafe {
        let body = (*response).body;
        // Don't free body, but free the response structure
        let status_text = (*response).status_text;
        let headers = (*response).headers;

        // Free status_text if present
        if !status_text.is_null() {
            crate::memory::trq_release(status_text as *mut u8);
        }

        // Free headers array if present
        if !headers.is_null() {
            // Free each header string
            let len = (*headers).len;
            for i in 0..len {
                let ptr = ((*headers).data as *mut *mut TrqString).add(i as usize);
                if !(*ptr).is_null() {
                    crate::memory::trq_release(*ptr as *mut u8);
                }
            }
            if !(*headers).data.is_null() {
                libc::free((*headers).data as *mut libc::c_void);
            }
            crate::memory::trq_free(headers as *mut u8);
        }

        crate::memory::trq_free(response as *mut u8);

        // Free method string
        crate::memory::trq_release(method as *mut u8);

        body
    }
}

/// Download a file from URL.
///
/// # Returns
/// `true` on success, `false` on error.
#[no_mangle]
pub extern "C" fn trq_http_download(url: *const TrqString, path: *const TrqString) -> bool {
    let path_str = match extract_string(path) {
        Some(p) => p,
        None => return false,
    };

    let method = string_to_trq("GET");
    let response = trq_http_request(method, url, std::ptr::null(), std::ptr::null(), 30000, true);

    if response.is_null() {
        crate::memory::trq_release(method as *mut u8);
        return false;
    }

    unsafe {
        let status = (*response).status;
        let body = (*response).body;

        let success = if status >= 200
            && status < 300
            && !body.is_null()
            && !(*body).data.is_null()
            && (*body).len > 0
        {
            let body_slice = std::slice::from_raw_parts((*body).data, (*body).len as usize);
            std::fs::write(&path_str, body_slice).is_ok()
        } else if status >= 200 && status < 300 && !body.is_null() && (*body).len == 0 {
            // Empty body is still success, write empty file
            std::fs::write(&path_str, &[]).is_ok()
        } else {
            false
        };

        // Free response
        if !(*response).status_text.is_null() {
            crate::memory::trq_release((*response).status_text as *mut u8);
        }
        if !body.is_null() {
            crate::memory::trq_release(body as *mut u8);
        }
        if !(*response).headers.is_null() {
            let headers = (*response).headers;
            let len = (*headers).len;
            for i in 0..len {
                let ptr = ((*headers).data as *mut *mut TrqString).add(i as usize);
                if !(*ptr).is_null() {
                    crate::memory::trq_release(*ptr as *mut u8);
                }
            }
            if !(*headers).data.is_null() {
                libc::free((*headers).data as *mut libc::c_void);
            }
            crate::memory::trq_free(headers as *mut u8);
        }
        crate::memory::trq_free(response as *mut u8);
        crate::memory::trq_release(method as *mut u8);

        success
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encode_basic() {
        let input = string_to_trq("hello world");
        let encoded = trq_url_encode(input);
        unsafe {
            let slice = std::slice::from_raw_parts((*encoded).data, (*encoded).len as usize);
            let result = std::str::from_utf8(slice).unwrap();
            assert_eq!(result, "hello%20world");
            crate::memory::trq_release(input as *mut u8);
            crate::memory::trq_release(encoded as *mut u8);
        }
    }

    #[test]
    fn test_url_encode_arabic() {
        let input = string_to_trq("مرحبا");
        let encoded = trq_url_encode(input);
        unsafe {
            // Arabic text should be percent-encoded
            assert!((*encoded).len > 0);
            crate::memory::trq_release(input as *mut u8);
            crate::memory::trq_release(encoded as *mut u8);
        }
    }

    #[test]
    fn test_url_decode_basic() {
        let input = string_to_trq("hello%20world");
        let decoded = trq_url_decode(input);
        unsafe {
            let slice = std::slice::from_raw_parts((*decoded).data, (*decoded).len as usize);
            let result = std::str::from_utf8(slice).unwrap();
            assert_eq!(result, "hello world");
            crate::memory::trq_release(input as *mut u8);
            crate::memory::trq_release(decoded as *mut u8);
        }
    }

    #[test]
    fn test_url_decode_plus_as_space() {
        let input = string_to_trq("hello+world");
        let decoded = trq_url_decode(input);
        unsafe {
            let slice = std::slice::from_raw_parts((*decoded).data, (*decoded).len as usize);
            let result = std::str::from_utf8(slice).unwrap();
            assert_eq!(result, "hello world");
            crate::memory::trq_release(input as *mut u8);
            crate::memory::trq_release(decoded as *mut u8);
        }
    }

    #[test]
    fn test_parse_url_simple() {
        let parsed = parse_url("http://example.com/path").unwrap();
        assert_eq!(parsed.host, "example.com");
        assert_eq!(parsed.port, 80);
        assert_eq!(parsed.path, "/path");
        assert!(!parsed.is_https);
    }

    #[test]
    fn test_parse_url_with_port() {
        let parsed = parse_url("http://example.com:8080/path").unwrap();
        assert_eq!(parsed.host, "example.com");
        assert_eq!(parsed.port, 8080);
        assert_eq!(parsed.path, "/path");
    }

    #[test]
    fn test_parse_url_https() {
        let parsed = parse_url("https://example.com/").unwrap();
        assert_eq!(parsed.host, "example.com");
        assert_eq!(parsed.port, 443);
        assert!(parsed.is_https);
    }

    #[test]
    fn test_tcp_connect_invalid_port() {
        let addr = string_to_trq("127.0.0.1");
        assert_eq!(trq_tcp_connect(addr, 0, 100), -1);
        assert_eq!(trq_tcp_connect(addr, 70000, 100), -1);
        unsafe {
            crate::memory::trq_release(addr as *mut u8);
        }
    }

    #[test]
    fn test_tcp_connect_null_address() {
        assert_eq!(trq_tcp_connect(std::ptr::null(), 80, 100), -1);
    }

    #[test]
    fn test_udp_bind_and_close() {
        // Use port 0 for automatic assignment
        let handle = trq_udp_bind(0);
        assert!(handle > 0);
        trq_udp_close(handle);
    }

    #[test]
    fn test_resolve_localhost() {
        let hostname = string_to_trq("localhost");
        let ip = trq_resolve_hostname(hostname);
        unsafe {
            // Should resolve to 127.0.0.1
            let slice = std::slice::from_raw_parts((*ip).data, (*ip).len as usize);
            let result = std::str::from_utf8(slice).unwrap();
            assert!(result == "127.0.0.1" || result.starts_with("::"));
            crate::memory::trq_release(hostname as *mut u8);
            crate::memory::trq_release(ip as *mut u8);
        }
    }

    #[test]
    fn test_get_local_ip() {
        let ip = trq_get_local_ip();
        unsafe {
            // Should return some IP address
            assert!((*ip).len > 0);
            crate::memory::trq_release(ip as *mut u8);
        }
    }
}
