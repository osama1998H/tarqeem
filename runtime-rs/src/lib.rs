//! Tarqeem Runtime Library (Rust Implementation)
//!
//! This crate provides the runtime library for Tarqeem programs, implementing
//! memory management, string operations, array handling, I/O, math, and other core functionality.
//!
//! # ABI Compatibility
//!
//! All exported functions use the C calling convention (`extern "C"`) and are
//! marked with `#[no_mangle]` to ensure they can be linked with compiled
//! Tarqeem programs, JIT-compiled code, and the interpreter.
//!
//! # Module Structure
//!
//! - `types`: FFI-compatible type definitions (TrqString, TrqArray, RefCountHeader)
//! - `memory`: Reference-counted memory allocation
//! - `string`: String operations (creation, manipulation, conversion)
//! - `array`: Array operations (creation, access, modification)
//! - `io`: Input/output operations (console, file, directory, path)
//! - `math`: Mathematical functions (basic, trig, log, random)
//! - `runtime`: Runtime initialization and utilities
//!
//! # Example Usage (from C)
//!
//! ```c
//! #include <stdint.h>
//!
//! // Link with libtrq.a or libtrq.so
//! extern void* trq_alloc(int64_t size);
//! extern void trq_retain(void* ptr);
//! extern void trq_release(void* ptr);
//! extern int64_t trq_refcount(void* ptr);
//!
//! int main() {
//!     void* ptr = trq_alloc(100);
//!     // refcount is 1
//!
//!     trq_retain(ptr);
//!     // refcount is 2
//!
//!     trq_release(ptr);
//!     // refcount is 1
//!
//!     trq_release(ptr);
//!     // refcount is 0, memory freed
//!
//!     return 0;
//! }
//! ```

// FFI library - callers are responsible for pointer validity
#![allow(non_snake_case)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(clippy::manual_range_contains)]

pub mod array;
pub mod compress;
pub mod crypto;
pub mod date;
mod helpers; // Internal helpers (not re-exported)
pub mod io;
pub mod math;
pub mod memory;
pub mod network;
pub mod runtime;
pub mod string;
pub mod types;

// Re-export all public types
pub use types::{RefCountHeader, TrqArray, TrqHttpResponse, TrqString, TrqTcpInfo, HEADER_SIZE};

// Re-export all memory functions
pub use memory::{trq_alloc, trq_free, trq_realloc, trq_refcount, trq_release, trq_retain};

// Re-export all string functions
pub use string::{
    trq_bool_to_string, trq_float_to_string, trq_int_to_string, trq_string_char_at,
    trq_string_char_code, trq_string_clone, trq_string_compare, trq_string_concat,
    trq_string_contains, trq_string_count, trq_string_ends_with, trq_string_equals,
    trq_string_free_data, trq_string_from_bytes, trq_string_from_char_code, trq_string_from_cstr,
    trq_string_index_of, trq_string_is_alpha, trq_string_is_arabic, trq_string_is_numeric,
    trq_string_join, trq_string_last_index_of, trq_string_len, trq_string_len_chars,
    trq_string_new, trq_string_pad_left, trq_string_pad_right, trq_string_repeat,
    trq_string_replace, trq_string_replace_all, trq_string_reverse, trq_string_split,
    trq_string_starts_with, trq_string_substr_chars, trq_string_to_bytes, trq_string_to_float,
    trq_string_to_int, trq_string_to_lower, trq_string_to_title, trq_string_to_upper,
    trq_string_trim, trq_string_trim_left, trq_string_trim_right,
};

// Re-export all array functions
pub use array::{
    trq_array_clone, trq_array_concat, trq_array_ensure_capacity, trq_array_free_data,
    trq_array_get, trq_array_len, trq_array_new, trq_array_pop, trq_array_push, trq_array_set,
    trq_array_slice,
};

// Re-export all I/O functions
pub use io::{
    // Directory operations
    trq_dir_create,
    trq_dir_create_all,
    trq_dir_current,
    trq_dir_delete,
    trq_dir_home,
    trq_dir_list,
    trq_dir_temp,
    // File operations (basic)
    trq_file_append,
    // File handle/stream operations
    trq_file_close,
    trq_file_copy,
    trq_file_delete,
    trq_file_eof,
    trq_file_exists,
    trq_file_flush,
    trq_file_is_dir,
    trq_file_is_file,
    trq_file_move,
    trq_file_open,
    trq_file_open_append,
    trq_file_open_read,
    trq_file_open_write,
    trq_file_read,
    trq_file_read_line,
    trq_file_size,
    trq_file_write,
    trq_file_write_line,
    // Console I/O
    trq_input,
    trq_input_float,
    trq_input_int,
    trq_input_prompt,
    // Path operations
    trq_path_absolute,
    trq_path_delete,
    trq_path_extension,
    trq_path_filename,
    trq_path_is_absolute,
    trq_path_join,
    trq_path_parent,
    trq_path_status,
    trq_path_stem,
    trq_print,
    trq_print_bool,
    trq_print_error,
    trq_print_float,
    trq_print_int,
    trq_print_newline,
    trq_print_optional_scalar,
    // Stream reading
    trq_read_stream,
    // Stream writing
    trq_write_stream,
};

// Re-export all math functions
pub use math::{
    // Basic math
    trq_abs_float,
    trq_abs_int,
    // Inverse trig
    trq_acos,
    trq_asin,
    trq_atan,
    trq_atan2,
    trq_cbrt,
    // Rounding
    trq_ceil,
    // Comparison
    trq_clamp_float,
    trq_clamp_int,
    // Trigonometric
    trq_cos,
    // Hyperbolic
    trq_cosh,
    trq_cot,
    trq_csc,
    // Constants
    trq_e,
    // Logarithmic
    trq_exp,
    // Number theory
    trq_factorial,
    trq_floor,
    trq_gcd,
    trq_lcm,
    trq_log,
    trq_log10,
    trq_log2,
    trq_max_float,
    trq_max_int,
    trq_min_float,
    trq_min_int,
    trq_mod,
    trq_nroot,
    trq_pi,
    trq_pow_float,
    trq_pow_int,
    // Random
    trq_random_bool,
    trq_random_float,
    trq_random_float_range,
    trq_random_int,
    trq_random_int_range,
    trq_random_seed,
    trq_round,
    trq_sec,
    trq_sign,
    trq_sin,
    trq_sinh,
    trq_sqrt,
    trq_tan,
    trq_tanh,
    // Angle conversion
    trq_to_degrees,
    trq_to_radians,
    trq_trunc,
};

// Re-export all runtime functions
pub use runtime::{
    trq_abort, trq_assert, trq_debug, trq_env_get, trq_env_remove, trq_env_set, trq_exit,
    trq_panic, trq_performance_now, trq_program_args, trq_runtime_cleanup, trq_runtime_init,
    trq_time_now, trq_version,
};

// Re-export all network functions
pub use network::{
    // DNS/Utility (2 functions)
    trq_get_local_ip,
    // HTTP (3 functions)
    trq_http_download,
    trq_http_get,
    trq_http_request,
    trq_resolve_hostname,
    // TCP (13 functions)
    trq_tcp_accept,
    trq_tcp_accept_timeout,
    trq_tcp_available,
    trq_tcp_close,
    trq_tcp_connect,
    trq_tcp_listen,
    trq_tcp_local_address,
    trq_tcp_local_port,
    trq_tcp_receive,
    trq_tcp_receive_bytes,
    trq_tcp_receive_until,
    trq_tcp_send,
    trq_tcp_send_bytes,
    // UDP (7 functions)
    trq_udp_bind,
    trq_udp_close,
    trq_udp_receive,
    trq_udp_receive_bytes,
    trq_udp_reply,
    trq_udp_send_bytes_to,
    trq_udp_send_to,
    // URL encoding (2 functions)
    trq_url_decode,
    trq_url_encode,
};

// Re-export all crypto functions
pub use crypto::{
    // Base64 encoding functions (2)
    trq_base64_decode,
    trq_base64_encode,
    // Hex encoding functions (4)
    trq_hex_decode,
    trq_hex_decode_to_bytes,
    trq_hex_encode,
    trq_hex_encode_bytes,
    // SHA-256 functions (4)
    trq_sha256_bytes,
    trq_sha256_compare,
    trq_sha256_file,
    trq_sha256_string,
};

// Re-export all date/time functions (8)
pub use date::{
    trq_date_diff_days, trq_date_format, trq_datetime_format, trq_day_of_week, trq_day_of_year,
    trq_days_in_month, trq_time_format, trq_week_number, INVALID,
};

// Re-export all compression functions
pub use compress::{
    // String/bytes compression (4)
    trq_gzip_compress_bytes,
    // File compression (2)
    trq_gzip_compress_file,
    trq_gzip_compress_string,
    trq_gzip_decompress_bytes,
    trq_gzip_decompress_file,
    trq_gzip_decompress_to_string,
};
