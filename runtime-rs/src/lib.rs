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
mod helpers; // Internal helpers (not re-exported)
pub mod io;
pub mod math;
pub mod memory;
pub mod network;
pub mod runtime;
#[cfg(feature = "graphics")]
pub mod sdl2;
pub mod string;
pub mod types;

// Re-export all public types
pub use types::{RefCountHeader, TrqArray, TrqHttpResponse, TrqString, TrqTcpInfo, HEADER_SIZE};

// Re-export all memory functions
pub use memory::{trq_alloc, trq_free, trq_realloc, trq_refcount, trq_release, trq_retain};

// Re-export all string functions
pub use string::{
    trq_bool_to_string, trq_float_to_string, trq_int_to_string, trq_string_char_at,
    trq_string_clone, trq_string_compare, trq_string_concat, trq_string_contains, trq_string_count,
    trq_string_ends_with, trq_string_equals, trq_string_free_data, trq_string_from_cstr,
    trq_string_index_of, trq_string_is_alpha, trq_string_is_arabic, trq_string_is_numeric,
    trq_string_join, trq_string_last_index_of, trq_string_len, trq_string_len_chars,
    trq_string_new, trq_string_pad_left, trq_string_pad_right, trq_string_repeat,
    trq_string_replace, trq_string_replace_all, trq_string_reverse, trq_string_split,
    trq_string_starts_with, trq_string_substr, trq_string_substr_chars, trq_string_to_float,
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
    trq_path_extension,
    trq_path_filename,
    trq_path_is_absolute,
    trq_path_join,
    trq_path_parent,
    trq_path_stem,
    trq_print,
    trq_print_bool,
    trq_print_error,
    trq_print_float,
    trq_print_int,
    trq_print_newline,
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
    trq_abort, trq_assert, trq_debug, trq_env_get, trq_env_remove, trq_env_set, trq_panic,
    trq_runtime_cleanup, trq_runtime_init, trq_version,
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

// Re-export all SDL2/graphics functions (when graphics feature is enabled)
#[cfg(feature = "graphics")]
pub use sdl2::{
    // Colors
    trq_color_black,
    trq_color_blue,
    trq_color_cyan,
    trq_color_gray,
    trq_color_green,
    trq_color_hsl,
    trq_color_magenta,
    trq_color_orange,
    trq_color_red,
    trq_color_rgb,
    trq_color_rgba,
    trq_color_white,
    trq_color_yellow,
    // Timing
    trq_delay,
    // Events
    trq_event_get,
    trq_event_key,
    trq_event_mouse_button,
    trq_event_mouse_x,
    trq_event_mouse_y,
    trq_event_poll,
    trq_event_type,
    trq_event_wait,
    trq_get_ticks,
    // Keyboard/Mouse state
    trq_key_pressed,
    trq_mouse_button_pressed,
    trq_mouse_position,
    trq_mouse_x,
    trq_mouse_y,
    // Rendering
    trq_render_circle,
    trq_render_clear,
    trq_render_ellipse,
    trq_render_fill_circle,
    trq_render_fill_rect,
    trq_render_fill_rect_struct,
    trq_render_line,
    trq_render_point,
    trq_render_present,
    trq_render_rect,
    trq_render_rect_struct,
    trq_render_set_color,
    trq_render_set_color_struct,
    // Initialization
    trq_sdl_init,
    trq_sdl_is_init,
    trq_sdl_quit,
    // Window management
    trq_window_close,
    trq_window_create,
    trq_window_height,
    trq_window_hide,
    trq_window_set_fullscreen,
    trq_window_set_position,
    trq_window_set_size,
    trq_window_set_title,
    trq_window_show,
    trq_window_width,
    // Types
    TrqColor,
    TrqEvent,
    TrqEventType,
    TrqRect,
    // Key constants
    TRQ_KEY_0,
    TRQ_KEY_1,
    TRQ_KEY_2,
    TRQ_KEY_3,
    TRQ_KEY_4,
    TRQ_KEY_5,
    TRQ_KEY_6,
    TRQ_KEY_7,
    TRQ_KEY_8,
    TRQ_KEY_9,
    TRQ_KEY_A,
    TRQ_KEY_B,
    TRQ_KEY_BACKSPACE,
    TRQ_KEY_C,
    TRQ_KEY_D,
    TRQ_KEY_DOWN,
    TRQ_KEY_E,
    TRQ_KEY_ENTER,
    TRQ_KEY_ESCAPE,
    TRQ_KEY_F,
    TRQ_KEY_G,
    TRQ_KEY_H,
    TRQ_KEY_I,
    TRQ_KEY_J,
    TRQ_KEY_K,
    TRQ_KEY_L,
    TRQ_KEY_LEFT,
    TRQ_KEY_M,
    TRQ_KEY_N,
    TRQ_KEY_O,
    TRQ_KEY_P,
    TRQ_KEY_Q,
    TRQ_KEY_R,
    TRQ_KEY_RIGHT,
    TRQ_KEY_S,
    TRQ_KEY_SPACE,
    TRQ_KEY_T,
    TRQ_KEY_TAB,
    TRQ_KEY_U,
    TRQ_KEY_UP,
    TRQ_KEY_V,
    TRQ_KEY_W,
    TRQ_KEY_X,
    TRQ_KEY_Y,
    TRQ_KEY_Z,
};
