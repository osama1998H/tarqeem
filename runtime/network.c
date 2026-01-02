/**
 * Tarqeem Runtime - Network Operations
 *
 * Implements TCP, UDP, and HTTP networking for the Tarqeem language.
 *
 * مكتبة الشبكة لوقت التشغيل - ترقيم
 */

#include "tarqeem_rt.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <errno.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/socket.h>
#include <sys/select.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <arpa/inet.h>
#include <netdb.h>
#include <poll.h>

/*============================================================================
 * Internal Helper Functions
 *============================================================================*/

/**
 * Set socket to non-blocking mode.
 */
static bool set_nonblocking(int fd) {
    int flags = fcntl(fd, F_GETFL, 0);
    if (flags == -1) return false;
    return fcntl(fd, F_SETFL, flags | O_NONBLOCK) != -1;
}

/**
 * Set socket to blocking mode.
 */
static bool set_blocking(int fd) {
    int flags = fcntl(fd, F_GETFL, 0);
    if (flags == -1) return false;
    return fcntl(fd, F_SETFL, flags & ~O_NONBLOCK) != -1;
}

/**
 * Wait for socket to be readable with timeout.
 */
static bool wait_readable(int fd, int64_t timeout_ms) {
    struct pollfd pfd;
    pfd.fd = fd;
    pfd.events = POLLIN;
    pfd.revents = 0;

    int result = poll(&pfd, 1, (int)timeout_ms);
    return result > 0 && (pfd.revents & POLLIN);
}

/**
 * Wait for socket to be writable with timeout.
 */
static bool wait_writable(int fd, int64_t timeout_ms) {
    struct pollfd pfd;
    pfd.fd = fd;
    pfd.events = POLLOUT;
    pfd.revents = 0;

    int result = poll(&pfd, 1, (int)timeout_ms);
    return result > 0 && (pfd.revents & POLLOUT);
}

/*============================================================================
 * TCP Operations
 *============================================================================*/

int64_t trq_tcp_connect(TrqString* address, int64_t port, int64_t timeout_ms) {
    if (!address || !address->data || port <= 0 || port > 65535) {
        return -1;
    }

    // Resolve hostname
    struct addrinfo hints, *result, *rp;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;

    char port_str[16];
    snprintf(port_str, sizeof(port_str), "%ld", (long)port);

    if (getaddrinfo(address->data, port_str, &hints, &result) != 0) {
        return -1;
    }

    int sockfd = -1;
    for (rp = result; rp != NULL; rp = rp->ai_next) {
        sockfd = socket(rp->ai_family, rp->ai_socktype, rp->ai_protocol);
        if (sockfd == -1) continue;

        // Set non-blocking for timeout support
        if (timeout_ms > 0) {
            set_nonblocking(sockfd);
        }

        int conn_result = connect(sockfd, rp->ai_addr, rp->ai_addrlen);

        if (conn_result == 0) {
            // Connected immediately
            if (timeout_ms > 0) set_blocking(sockfd);
            break;
        } else if (errno == EINPROGRESS && timeout_ms > 0) {
            // Wait for connection with timeout
            if (wait_writable(sockfd, timeout_ms)) {
                int error = 0;
                socklen_t len = sizeof(error);
                if (getsockopt(sockfd, SOL_SOCKET, SO_ERROR, &error, &len) == 0 && error == 0) {
                    set_blocking(sockfd);
                    break;
                }
            }
            close(sockfd);
            sockfd = -1;
        } else {
            close(sockfd);
            sockfd = -1;
        }
    }

    freeaddrinfo(result);

    // Enable TCP keepalive
    if (sockfd != -1) {
        int keepalive = 1;
        setsockopt(sockfd, SOL_SOCKET, SO_KEEPALIVE, &keepalive, sizeof(keepalive));
    }

    return (int64_t)sockfd;
}

void trq_tcp_close(int64_t handle) {
    if (handle >= 0) {
        close((int)handle);
    }
}

bool trq_tcp_send(int64_t handle, TrqString* data) {
    if (handle < 0 || !data || !data->data) {
        return false;
    }

    ssize_t total_sent = 0;
    ssize_t len = data->len;
    const char* ptr = data->data;

    while (total_sent < len) {
        ssize_t sent = send((int)handle, ptr + total_sent, len - total_sent, MSG_NOSIGNAL);
        if (sent <= 0) {
            if (errno == EINTR) continue;
            return false;
        }
        total_sent += sent;
    }

    return true;
}

bool trq_tcp_send_bytes(int64_t handle, TrqArray* data) {
    if (handle < 0 || !data || !data->data || data->len <= 0) {
        return false;
    }

    ssize_t total_sent = 0;
    ssize_t len = data->len * data->elem_size;
    const char* ptr = (const char*)data->data;

    while (total_sent < len) {
        ssize_t sent = send((int)handle, ptr + total_sent, len - total_sent, MSG_NOSIGNAL);
        if (sent <= 0) {
            if (errno == EINTR) continue;
            return false;
        }
        total_sent += sent;
    }

    return true;
}

TrqString* trq_tcp_receive(int64_t handle, int64_t timeout_ms) {
    if (handle < 0) {
        return trq_string_new("", 0);
    }

    if (timeout_ms > 0 && !wait_readable((int)handle, timeout_ms)) {
        return trq_string_new("", 0);
    }

    char buffer[8192];
    ssize_t received = recv((int)handle, buffer, sizeof(buffer) - 1, 0);

    if (received <= 0) {
        return trq_string_new("", 0);
    }

    buffer[received] = '\0';
    return trq_string_new(buffer, (int64_t)received);
}

TrqArray* trq_tcp_receive_bytes(int64_t handle, int64_t size, int64_t timeout_ms) {
    TrqArray* result = trq_array_new(0, sizeof(uint8_t));
    if (!result || handle < 0 || size <= 0) {
        return result;
    }

    if (timeout_ms > 0 && !wait_readable((int)handle, timeout_ms)) {
        return result;
    }

    uint8_t* buffer = (uint8_t*)malloc(size);
    if (!buffer) {
        return result;
    }

    ssize_t total_received = 0;
    while (total_received < size) {
        ssize_t received = recv((int)handle, buffer + total_received, size - total_received, 0);
        if (received <= 0) {
            if (errno == EINTR) continue;
            break;
        }
        total_received += received;
    }

    for (ssize_t i = 0; i < total_received; i++) {
        trq_array_push(result, &buffer[i], sizeof(uint8_t));
    }

    free(buffer);
    return result;
}

TrqString* trq_tcp_receive_until(int64_t handle, TrqString* delimiter, int64_t timeout_ms) {
    if (handle < 0 || !delimiter || !delimiter->data) {
        return trq_string_new("", 0);
    }

    char* buffer = NULL;
    size_t buffer_size = 0;
    size_t buffer_len = 0;

    while (1) {
        if (timeout_ms > 0 && !wait_readable((int)handle, timeout_ms)) {
            break;
        }

        char c;
        ssize_t received = recv((int)handle, &c, 1, 0);
        if (received <= 0) {
            if (errno == EINTR) continue;
            break;
        }

        // Grow buffer if needed
        if (buffer_len + 1 >= buffer_size) {
            size_t new_size = buffer_size == 0 ? 256 : buffer_size * 2;
            char* new_buffer = (char*)realloc(buffer, new_size);
            if (!new_buffer) break;
            buffer = new_buffer;
            buffer_size = new_size;
        }

        buffer[buffer_len++] = c;

        // Check for delimiter
        if (buffer_len >= (size_t)delimiter->len) {
            if (memcmp(buffer + buffer_len - delimiter->len, delimiter->data, delimiter->len) == 0) {
                buffer_len -= delimiter->len;
                break;
            }
        }
    }

    TrqString* result;
    if (buffer && buffer_len > 0) {
        result = trq_string_new(buffer, (int64_t)buffer_len);
    } else {
        result = trq_string_new("", 0);
    }

    free(buffer);
    return result;
}

bool trq_tcp_available(int64_t handle) {
    if (handle < 0) {
        return false;
    }

    struct pollfd pfd;
    pfd.fd = (int)handle;
    pfd.events = POLLIN;
    pfd.revents = 0;

    return poll(&pfd, 1, 0) > 0 && (pfd.revents & POLLIN);
}

int64_t trq_tcp_listen(TrqString* address, int64_t port, int64_t backlog) {
    if (port <= 0 || port > 65535) {
        return -1;
    }

    int sockfd = socket(AF_INET, SOCK_STREAM, 0);
    if (sockfd < 0) {
        return -1;
    }

    // Allow address reuse
    int opt = 1;
    setsockopt(sockfd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));

    struct sockaddr_in server_addr;
    memset(&server_addr, 0, sizeof(server_addr));
    server_addr.sin_family = AF_INET;
    server_addr.sin_port = htons((uint16_t)port);

    if (address && address->data && address->len > 0) {
        if (inet_pton(AF_INET, address->data, &server_addr.sin_addr) <= 0) {
            server_addr.sin_addr.s_addr = INADDR_ANY;
        }
    } else {
        server_addr.sin_addr.s_addr = INADDR_ANY;
    }

    if (bind(sockfd, (struct sockaddr*)&server_addr, sizeof(server_addr)) < 0) {
        close(sockfd);
        return -1;
    }

    if (listen(sockfd, (int)backlog) < 0) {
        close(sockfd);
        return -1;
    }

    return (int64_t)sockfd;
}

TrqTcpInfo* trq_tcp_accept(int64_t handle) {
    if (handle < 0) {
        return NULL;
    }

    struct sockaddr_in client_addr;
    socklen_t addr_len = sizeof(client_addr);

    int client_fd = accept((int)handle, (struct sockaddr*)&client_addr, &addr_len);
    if (client_fd < 0) {
        return NULL;
    }

    TrqTcpInfo* info = (TrqTcpInfo*)trq_alloc(sizeof(TrqTcpInfo));
    if (!info) {
        close(client_fd);
        return NULL;
    }

    char addr_str[INET_ADDRSTRLEN];
    inet_ntop(AF_INET, &client_addr.sin_addr, addr_str, INET_ADDRSTRLEN);

    info->address = trq_string_from_cstr(addr_str);
    info->port = (int64_t)ntohs(client_addr.sin_port);
    info->handle = (int64_t)client_fd;

    return info;
}

TrqTcpInfo* trq_tcp_accept_timeout(int64_t handle, int64_t timeout_ms) {
    if (handle < 0) {
        return NULL;
    }

    if (!wait_readable((int)handle, timeout_ms)) {
        TrqTcpInfo* info = (TrqTcpInfo*)trq_alloc(sizeof(TrqTcpInfo));
        if (info) {
            info->address = trq_string_new("", 0);
            info->port = 0;
            info->handle = -1;
        }
        return info;
    }

    return trq_tcp_accept(handle);
}

TrqString* trq_tcp_local_address(int64_t handle) {
    if (handle < 0) {
        return trq_string_new("", 0);
    }

    struct sockaddr_in addr;
    socklen_t addr_len = sizeof(addr);

    if (getsockname((int)handle, (struct sockaddr*)&addr, &addr_len) < 0) {
        return trq_string_new("", 0);
    }

    char addr_str[INET_ADDRSTRLEN];
    inet_ntop(AF_INET, &addr.sin_addr, addr_str, INET_ADDRSTRLEN);

    return trq_string_from_cstr(addr_str);
}

int64_t trq_tcp_local_port(int64_t handle) {
    if (handle < 0) {
        return -1;
    }

    struct sockaddr_in addr;
    socklen_t addr_len = sizeof(addr);

    if (getsockname((int)handle, (struct sockaddr*)&addr, &addr_len) < 0) {
        return -1;
    }

    return (int64_t)ntohs(addr.sin_port);
}

/*============================================================================
 * UDP Operations
 *============================================================================*/

// Store last sender info for UDP reply
static struct sockaddr_in udp_last_sender;
static socklen_t udp_last_sender_len = sizeof(udp_last_sender);

int64_t trq_udp_bind(int64_t port) {
    if (port < 0 || port > 65535) {
        return -1;
    }

    int sockfd = socket(AF_INET, SOCK_DGRAM, 0);
    if (sockfd < 0) {
        return -1;
    }

    int opt = 1;
    setsockopt(sockfd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = INADDR_ANY;
    addr.sin_port = htons((uint16_t)port);

    if (bind(sockfd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        close(sockfd);
        return -1;
    }

    return (int64_t)sockfd;
}

void trq_udp_close(int64_t handle) {
    if (handle >= 0) {
        close((int)handle);
    }
}

bool trq_udp_send_to(int64_t handle, TrqString* address, int64_t port, TrqString* data) {
    if (handle < 0 || !address || !address->data || !data || !data->data) {
        return false;
    }
    if (port <= 0 || port > 65535) {
        return false;
    }

    struct sockaddr_in dest_addr;
    memset(&dest_addr, 0, sizeof(dest_addr));
    dest_addr.sin_family = AF_INET;
    dest_addr.sin_port = htons((uint16_t)port);

    if (inet_pton(AF_INET, address->data, &dest_addr.sin_addr) <= 0) {
        // Try to resolve hostname
        struct hostent* host = gethostbyname(address->data);
        if (!host) {
            return false;
        }
        memcpy(&dest_addr.sin_addr, host->h_addr_list[0], host->h_length);
    }

    ssize_t sent = sendto((int)handle, data->data, data->len, 0,
                          (struct sockaddr*)&dest_addr, sizeof(dest_addr));

    return sent == data->len;
}

bool trq_udp_send_bytes_to(int64_t handle, TrqString* address, int64_t port, TrqArray* data) {
    if (handle < 0 || !address || !address->data || !data || !data->data) {
        return false;
    }
    if (port <= 0 || port > 65535) {
        return false;
    }

    struct sockaddr_in dest_addr;
    memset(&dest_addr, 0, sizeof(dest_addr));
    dest_addr.sin_family = AF_INET;
    dest_addr.sin_port = htons((uint16_t)port);

    if (inet_pton(AF_INET, address->data, &dest_addr.sin_addr) <= 0) {
        struct hostent* host = gethostbyname(address->data);
        if (!host) {
            return false;
        }
        memcpy(&dest_addr.sin_addr, host->h_addr_list[0], host->h_length);
    }

    size_t total_size = data->len * data->elem_size;
    ssize_t sent = sendto((int)handle, data->data, total_size, 0,
                          (struct sockaddr*)&dest_addr, sizeof(dest_addr));

    return sent == (ssize_t)total_size;
}

TrqString* trq_udp_receive(int64_t handle, int64_t timeout_ms) {
    if (handle < 0) {
        return trq_string_new("", 0);
    }

    if (timeout_ms > 0 && !wait_readable((int)handle, timeout_ms)) {
        return trq_string_new("", 0);
    }

    char buffer[65536];
    udp_last_sender_len = sizeof(udp_last_sender);

    ssize_t received = recvfrom((int)handle, buffer, sizeof(buffer) - 1, 0,
                                 (struct sockaddr*)&udp_last_sender, &udp_last_sender_len);

    if (received <= 0) {
        return trq_string_new("", 0);
    }

    buffer[received] = '\0';
    return trq_string_new(buffer, (int64_t)received);
}

TrqArray* trq_udp_receive_bytes(int64_t handle, int64_t size, int64_t timeout_ms) {
    TrqArray* result = trq_array_new(0, sizeof(uint8_t));
    if (!result || handle < 0 || size <= 0) {
        return result;
    }

    if (timeout_ms > 0 && !wait_readable((int)handle, timeout_ms)) {
        return result;
    }

    uint8_t* buffer = (uint8_t*)malloc(size);
    if (!buffer) {
        return result;
    }

    udp_last_sender_len = sizeof(udp_last_sender);
    ssize_t received = recvfrom((int)handle, buffer, size, 0,
                                 (struct sockaddr*)&udp_last_sender, &udp_last_sender_len);

    if (received > 0) {
        for (ssize_t i = 0; i < received; i++) {
            trq_array_push(result, &buffer[i], sizeof(uint8_t));
        }
    }

    free(buffer);
    return result;
}

bool trq_udp_reply(int64_t handle, TrqString* data) {
    if (handle < 0 || !data || !data->data) {
        return false;
    }

    ssize_t sent = sendto((int)handle, data->data, data->len, 0,
                          (struct sockaddr*)&udp_last_sender, udp_last_sender_len);

    return sent == data->len;
}

/*============================================================================
 * DNS and Network Utilities
 *============================================================================*/

TrqString* trq_resolve_hostname(TrqString* hostname) {
    if (!hostname || !hostname->data) {
        return trq_string_new("", 0);
    }

    struct addrinfo hints, *result;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;

    if (getaddrinfo(hostname->data, NULL, &hints, &result) != 0) {
        return trq_string_new("", 0);
    }

    char addr_str[INET_ADDRSTRLEN];
    struct sockaddr_in* addr = (struct sockaddr_in*)result->ai_addr;
    inet_ntop(AF_INET, &addr->sin_addr, addr_str, INET_ADDRSTRLEN);

    freeaddrinfo(result);
    return trq_string_from_cstr(addr_str);
}

TrqString* trq_get_local_ip(void) {
    // Create a UDP socket to determine local IP
    int sockfd = socket(AF_INET, SOCK_DGRAM, 0);
    if (sockfd < 0) {
        return trq_string_from_cstr("127.0.0.1");
    }

    // Connect to a public address (doesn't actually send data)
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons(80);
    inet_pton(AF_INET, "8.8.8.8", &addr.sin_addr);

    if (connect(sockfd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        close(sockfd);
        return trq_string_from_cstr("127.0.0.1");
    }

    struct sockaddr_in local_addr;
    socklen_t addr_len = sizeof(local_addr);
    if (getsockname(sockfd, (struct sockaddr*)&local_addr, &addr_len) < 0) {
        close(sockfd);
        return trq_string_from_cstr("127.0.0.1");
    }

    close(sockfd);

    char addr_str[INET_ADDRSTRLEN];
    inet_ntop(AF_INET, &local_addr.sin_addr, addr_str, INET_ADDRSTRLEN);

    return trq_string_from_cstr(addr_str);
}

/*============================================================================
 * HTTP Operations
 *============================================================================*/

/**
 * Parse URL into components.
 */
typedef struct {
    char* host;
    int port;
    char* path;
    bool is_https;
} ParsedUrl;

static ParsedUrl* parse_url(const char* url) {
    ParsedUrl* parsed = (ParsedUrl*)malloc(sizeof(ParsedUrl));
    if (!parsed) return NULL;

    memset(parsed, 0, sizeof(ParsedUrl));
    parsed->port = 80;
    parsed->path = strdup("/");

    const char* p = url;

    // Check protocol
    if (strncmp(p, "https://", 8) == 0) {
        parsed->is_https = true;
        parsed->port = 443;
        p += 8;
    } else if (strncmp(p, "http://", 7) == 0) {
        parsed->is_https = false;
        p += 7;
    }

    // Find host end
    const char* host_end = p;
    while (*host_end && *host_end != ':' && *host_end != '/') {
        host_end++;
    }

    parsed->host = (char*)malloc(host_end - p + 1);
    if (parsed->host) {
        strncpy(parsed->host, p, host_end - p);
        parsed->host[host_end - p] = '\0';
    }

    // Check for port
    if (*host_end == ':') {
        host_end++;
        parsed->port = atoi(host_end);
        while (*host_end && *host_end != '/') {
            host_end++;
        }
    }

    // Get path
    if (*host_end == '/') {
        free(parsed->path);
        parsed->path = strdup(host_end);
    }

    return parsed;
}

static void free_parsed_url(ParsedUrl* parsed) {
    if (parsed) {
        free(parsed->host);
        free(parsed->path);
        free(parsed);
    }
}

/**
 * Simple HTTP/1.1 request without SSL.
 * For HTTPS, returns error - would need OpenSSL integration.
 */
TrqHttpResponse* trq_http_request(
    TrqString* method,
    TrqString* url,
    TrqArray* headers,
    TrqString* body,
    int64_t timeout_ms,
    bool follow_redirects
) {
    TrqHttpResponse* response = (TrqHttpResponse*)trq_alloc(sizeof(TrqHttpResponse));
    if (!response) return NULL;

    response->status = 0;
    response->status_text = trq_string_new("", 0);
    response->body = trq_string_new("", 0);
    response->headers = trq_array_new(0, sizeof(TrqString*));

    if (!method || !method->data || !url || !url->data) {
        response->status = -1;
        trq_release(response->status_text);
        response->status_text = trq_string_from_cstr("خطأ: طريقة أو رابط فارغ");
        return response;
    }

    ParsedUrl* parsed = parse_url(url->data);
    if (!parsed || !parsed->host) {
        free_parsed_url(parsed);
        response->status = -1;
        trq_release(response->status_text);
        response->status_text = trq_string_from_cstr("خطأ: رابط غير صالح");
        return response;
    }

    // HTTPS not supported without OpenSSL
    if (parsed->is_https) {
        free_parsed_url(parsed);
        response->status = -1;
        trq_release(response->status_text);
        response->status_text = trq_string_from_cstr("خطأ: HTTPS غير مدعوم حالياً");
        return response;
    }

    // Connect to server
    TrqString* host_str = trq_string_from_cstr(parsed->host);
    int64_t sockfd = trq_tcp_connect(host_str, parsed->port, timeout_ms);
    trq_release(host_str);

    if (sockfd < 0) {
        free_parsed_url(parsed);
        response->status = -1;
        trq_release(response->status_text);
        response->status_text = trq_string_from_cstr("خطأ: فشل الاتصال بالخادم");
        return response;
    }

    // Build HTTP request
    char request_buffer[8192];
    int request_len = snprintf(request_buffer, sizeof(request_buffer),
        "%s %s HTTP/1.1\r\n"
        "Host: %s\r\n"
        "Connection: close\r\n"
        "User-Agent: Tarqeem/1.0\r\n",
        method->data, parsed->path, parsed->host);

    // Add custom headers
    if (headers && headers->len > 0) {
        for (int64_t i = 0; i < headers->len; i++) {
            TrqString** header_ptr = (TrqString**)((char*)headers->data + i * headers->elem_size);
            if (*header_ptr && (*header_ptr)->data) {
                request_len += snprintf(request_buffer + request_len,
                    sizeof(request_buffer) - request_len,
                    "%s\r\n", (*header_ptr)->data);
            }
        }
    }

    // Add content-length if body present
    if (body && body->data && body->len > 0) {
        request_len += snprintf(request_buffer + request_len,
            sizeof(request_buffer) - request_len,
            "Content-Length: %ld\r\n", (long)body->len);
    }

    // End headers
    request_len += snprintf(request_buffer + request_len,
        sizeof(request_buffer) - request_len, "\r\n");

    // Send request
    TrqString* request = trq_string_new(request_buffer, request_len);
    if (!trq_tcp_send(sockfd, request)) {
        trq_release(request);
        trq_tcp_close(sockfd);
        free_parsed_url(parsed);
        response->status = -1;
        trq_release(response->status_text);
        response->status_text = trq_string_from_cstr("خطأ: فشل إرسال الطلب");
        return response;
    }
    trq_release(request);

    // Send body if present
    if (body && body->data && body->len > 0) {
        trq_tcp_send(sockfd, body);
    }

    // Receive response
    char* response_buffer = NULL;
    size_t response_size = 0;
    size_t response_capacity = 0;

    while (1) {
        TrqString* chunk = trq_tcp_receive(sockfd, timeout_ms);
        if (!chunk || chunk->len == 0) {
            trq_release(chunk);
            break;
        }

        // Grow buffer
        if (response_size + chunk->len >= response_capacity) {
            size_t new_capacity = response_capacity == 0 ? 4096 : response_capacity * 2;
            while (new_capacity < response_size + chunk->len + 1) {
                new_capacity *= 2;
            }
            char* new_buffer = (char*)realloc(response_buffer, new_capacity);
            if (!new_buffer) {
                trq_release(chunk);
                break;
            }
            response_buffer = new_buffer;
            response_capacity = new_capacity;
        }

        memcpy(response_buffer + response_size, chunk->data, chunk->len);
        response_size += chunk->len;
        trq_release(chunk);
    }

    trq_tcp_close(sockfd);
    free_parsed_url(parsed);

    if (!response_buffer || response_size == 0) {
        free(response_buffer);
        response->status = -1;
        trq_release(response->status_text);
        response->status_text = trq_string_from_cstr("خطأ: لم يتم استلام رد");
        return response;
    }

    response_buffer[response_size] = '\0';

    // Parse HTTP response
    char* line_end = strstr(response_buffer, "\r\n");
    if (line_end) {
        // Parse status line: HTTP/1.1 200 OK
        char* status_start = strchr(response_buffer, ' ');
        if (status_start) {
            response->status = atoi(status_start + 1);
            char* status_text_start = strchr(status_start + 1, ' ');
            if (status_text_start) {
                size_t text_len = line_end - status_text_start - 1;
                trq_release(response->status_text);
                response->status_text = trq_string_new(status_text_start + 1, text_len);
            }
        }

        // Find body (after empty line)
        char* body_start = strstr(response_buffer, "\r\n\r\n");
        if (body_start) {
            body_start += 4;
            size_t body_len = response_size - (body_start - response_buffer);
            trq_release(response->body);
            response->body = trq_string_new(body_start, body_len);
        }
    }

    free(response_buffer);

    // Handle redirects
    if (follow_redirects && (response->status == 301 || response->status == 302 ||
                             response->status == 303 || response->status == 307)) {
        // Would need to parse Location header and recurse
        // For simplicity, not implementing recursive redirects
    }

    return response;
}

/**
 * Simple HTTP GET request.
 */
TrqString* trq_http_get(TrqString* url) {
    if (!url || !url->data) {
        return trq_string_new("", 0);
    }

    TrqString* method = trq_string_from_cstr("GET");
    TrqHttpResponse* response = trq_http_request(method, url, NULL, NULL, 30000, true);
    trq_release(method);

    if (!response) {
        return trq_string_new("", 0);
    }

    TrqString* body = response->body;
    if (body) {
        trq_retain(body);
    } else {
        body = trq_string_new("", 0);
    }

    // Free response but keep body
    trq_release(response->status_text);
    if (response->headers) trq_release(response->headers);
    trq_free(response);

    return body;
}

bool trq_http_download(TrqString* url, TrqString* path) {
    if (!url || !url->data || !path || !path->data) {
        return false;
    }

    TrqString* method = trq_string_from_cstr("GET");
    TrqHttpResponse* response = trq_http_request(method, url, NULL, NULL, 30000, true);
    trq_release(method);

    if (!response || response->status != 200) {
        if (response) trq_release(response);
        return false;
    }

    bool success = trq_file_write(path, response->body);
    trq_release(response);

    return success;
}

/*============================================================================
 * URL Encoding/Decoding
 *============================================================================*/

TrqString* trq_url_encode(TrqString* str) {
    if (!str || !str->data) {
        return trq_string_new("", 0);
    }

    // Worst case: every char becomes %XX (3x expansion)
    size_t max_len = str->len * 3 + 1;
    char* buffer = (char*)malloc(max_len);
    if (!buffer) {
        return trq_string_new("", 0);
    }

    size_t j = 0;
    for (int64_t i = 0; i < str->len; i++) {
        unsigned char c = (unsigned char)str->data[i];
        if ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') ||
            (c >= '0' && c <= '9') || c == '-' || c == '_' || c == '.' || c == '~') {
            buffer[j++] = c;
        } else {
            snprintf(buffer + j, 4, "%%%02X", c);
            j += 3;
        }
    }

    TrqString* result = trq_string_new(buffer, (int64_t)j);
    free(buffer);
    return result;
}

static int hex_to_int(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    return -1;
}

TrqString* trq_url_decode(TrqString* str) {
    if (!str || !str->data) {
        return trq_string_new("", 0);
    }

    char* buffer = (char*)malloc(str->len + 1);
    if (!buffer) {
        return trq_string_new("", 0);
    }

    size_t j = 0;
    for (int64_t i = 0; i < str->len; i++) {
        if (str->data[i] == '%' && i + 2 < str->len) {
            int high = hex_to_int(str->data[i + 1]);
            int low = hex_to_int(str->data[i + 2]);
            if (high >= 0 && low >= 0) {
                buffer[j++] = (char)((high << 4) | low);
                i += 2;
                continue;
            }
        } else if (str->data[i] == '+') {
            buffer[j++] = ' ';
            continue;
        }
        buffer[j++] = str->data[i];
    }

    TrqString* result = trq_string_new(buffer, (int64_t)j);
    free(buffer);
    return result;
}
