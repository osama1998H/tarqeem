/**
 * Tarqeem Runtime Library
 *
 * This header defines the runtime functions called by generated LLVM IR.
 * These functions handle memory management, string operations, arrays,
 * I/O, and other runtime features.
 */

#ifndef TARQEEM_RT_H
#define TARQEEM_RT_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/*============================================================================
 * Memory Management
 *============================================================================*/

/**
 * Allocate memory with reference counting header.
 * @param size Number of bytes to allocate
 * @return Pointer to allocated memory (after header)
 */
void* trq_alloc(int64_t size);

/**
 * Reallocate memory.
 * @param ptr Pointer to existing allocation
 * @param new_size New size in bytes
 * @return Pointer to reallocated memory
 */
void* trq_realloc(void* ptr, int64_t new_size);

/**
 * Free memory.
 * @param ptr Pointer to free
 */
void trq_free(void* ptr);

/**
 * Increment reference count.
 * @param ptr Pointer to object
 */
void trq_retain(void* ptr);

/**
 * Decrement reference count and free if zero.
 * @param ptr Pointer to object
 */
void trq_release(void* ptr);

/**
 * Get current reference count.
 * @param ptr Pointer to object
 * @return Reference count
 */
int64_t trq_refcount(void* ptr);

/*============================================================================
 * String Operations
 *============================================================================*/

/**
 * Tarqeem string structure.
 * Strings are UTF-8 encoded and reference counted.
 */
typedef struct TrqString {
    int64_t len;      // Length in bytes
    int64_t cap;      // Capacity in bytes
    char* data;       // UTF-8 data (null-terminated)
} TrqString;

/* Forward declaration for TrqArray (used by string functions) */
typedef struct TrqArray TrqArray;

/**
 * Create a new string from raw data.
 * @param data UTF-8 string data
 * @param len Length in bytes
 * @return Pointer to new string
 */
TrqString* trq_string_new(const char* data, int64_t len);

/**
 * Create a string from a null-terminated C string.
 * @param cstr C string
 * @return Pointer to new string
 */
TrqString* trq_string_from_cstr(const char* cstr);

/**
 * Concatenate two strings.
 * @param left First string
 * @param right Second string
 * @return New concatenated string
 */
TrqString* trq_string_concat(TrqString* left, TrqString* right);

/**
 * Get string length in bytes.
 * @param str String
 * @return Length in bytes
 */
int64_t trq_string_len(TrqString* str);

/**
 * Get string length in Unicode code points.
 * @param str String
 * @return Length in code points
 */
int64_t trq_string_len_chars(TrqString* str);

/**
 * Compare two strings.
 * @param a First string
 * @param b Second string
 * @return <0 if a<b, 0 if a==b, >0 if a>b
 */
int64_t trq_string_compare(TrqString* a, TrqString* b);

/**
 * Check if two strings are equal.
 * @param a First string
 * @param b Second string
 * @return true if equal
 */
bool trq_string_equals(TrqString* a, TrqString* b);

/**
 * Get substring.
 * @param str Source string
 * @param start Start index (bytes)
 * @param len Length (bytes)
 * @return New substring
 */
TrqString* trq_string_substr(TrqString* str, int64_t start, int64_t len);

/**
 * Convert integer to string.
 * @param value Integer value
 * @return String representation
 */
TrqString* trq_int_to_string(int64_t value);

/**
 * Convert float to string.
 * @param value Float value
 * @return String representation
 */
TrqString* trq_float_to_string(double value);

/**
 * Convert boolean to string.
 * @param value Boolean value
 * @return "صحيح"/"true" or "خاطئ"/"false"
 */
TrqString* trq_bool_to_string(bool value);

/**
 * Parse string to integer.
 * @param str String to parse
 * @return Parsed integer or 0 on error
 */
int64_t trq_string_to_int(TrqString* str);

/**
 * Parse string to float.
 * @param str String to parse
 * @return Parsed float or 0.0 on error
 */
double trq_string_to_float(TrqString* str);

/**
 * Check if string contains substring.
 * @param str Main string
 * @param substr Substring to find
 * @return true if found
 */
bool trq_string_contains(TrqString* str, TrqString* substr);

/**
 * Check if string starts with prefix.
 * @param str Main string
 * @param prefix Prefix to check
 * @return true if starts with prefix
 */
bool trq_string_starts_with(TrqString* str, TrqString* prefix);

/**
 * Check if string ends with suffix.
 * @param str Main string
 * @param suffix Suffix to check
 * @return true if ends with suffix
 */
bool trq_string_ends_with(TrqString* str, TrqString* suffix);

/**
 * Find index of substring.
 * @param str Main string
 * @param substr Substring to find
 * @return Index of first occurrence, or -1 if not found
 */
int64_t trq_string_index_of(TrqString* str, TrqString* substr);

/**
 * Convert string to uppercase.
 * @param str String to convert
 * @return New uppercase string (ASCII only)
 */
TrqString* trq_string_to_upper(TrqString* str);

/**
 * Convert string to lowercase.
 * @param str String to convert
 * @return New lowercase string (ASCII only)
 */
TrqString* trq_string_to_lower(TrqString* str);

/**
 * Trim whitespace from both ends.
 * @param str String to trim
 * @return New trimmed string
 */
TrqString* trq_string_trim(TrqString* str);

/**
 * Repeat string n times.
 * @param str String to repeat
 * @param n Number of repetitions
 * @return New repeated string
 */
TrqString* trq_string_repeat(TrqString* str, int64_t n);

/**
 * Replace first occurrence.
 * @param str Source string
 * @param old_str String to find
 * @param new_str Replacement string
 * @return New string with replacement
 */
TrqString* trq_string_replace(TrqString* str, TrqString* old_str, TrqString* new_str);

/**
 * Split string by delimiter.
 * @param str String to split
 * @param delim Delimiter
 * @return Array of strings
 */
TrqArray* trq_string_split(TrqString* str, TrqString* delim);

/**
 * Find last index of substring.
 * @param str Main string
 * @param substr Substring to find
 * @return Index of last occurrence, or -1 if not found
 */
int64_t trq_string_last_index_of(TrqString* str, TrqString* substr);

/**
 * Count occurrences of substring.
 * @param str Main string
 * @param substr Substring to count
 * @return Number of occurrences
 */
int64_t trq_string_count(TrqString* str, TrqString* substr);

/**
 * Reverse a string.
 * @param str String to reverse
 * @return New reversed string (respects UTF-8 code points)
 */
TrqString* trq_string_reverse(TrqString* str);

/**
 * Convert string to title case.
 * @param str String to convert
 * @return New title case string
 */
TrqString* trq_string_to_title(TrqString* str);

/**
 * Trim leading whitespace.
 * @param str String to trim
 * @return New trimmed string
 */
TrqString* trq_string_trim_left(TrqString* str);

/**
 * Trim trailing whitespace.
 * @param str String to trim
 * @return New trimmed string
 */
TrqString* trq_string_trim_right(TrqString* str);

/**
 * Join array of strings with delimiter.
 * @param arr Array of strings
 * @param delim Delimiter
 * @return Joined string
 */
TrqString* trq_string_join(TrqArray* arr, TrqString* delim);

/**
 * Replace all occurrences.
 * @param str Source string
 * @param old_str String to find
 * @param new_str Replacement string
 * @return New string with all replacements
 */
TrqString* trq_string_replace_all(TrqString* str, TrqString* old_str, TrqString* new_str);

/**
 * Pad string on the left.
 * @param str String to pad
 * @param target_len Target length in characters
 * @param pad Padding character (string, first char used)
 * @return New padded string
 */
TrqString* trq_string_pad_left(TrqString* str, int64_t target_len, TrqString* pad);

/**
 * Pad string on the right.
 * @param str String to pad
 * @param target_len Target length in characters
 * @param pad Padding character (string, first char used)
 * @return New padded string
 */
TrqString* trq_string_pad_right(TrqString* str, int64_t target_len, TrqString* pad);

/**
 * Check if string is numeric (digits only).
 * @param str String to check
 * @return true if all characters are digits
 */
bool trq_string_is_numeric(TrqString* str);

/**
 * Check if string contains only alphabetic characters.
 * @param str String to check
 * @return true if all characters are alphabetic
 */
bool trq_string_is_alpha(TrqString* str);

/**
 * Check if string contains only Arabic characters.
 * @param str String to check
 * @return true if all characters are Arabic
 */
bool trq_string_is_arabic(TrqString* str);

/**
 * Get character at index (UTF-8 aware).
 * @param str String
 * @param index Character index (not byte index)
 * @return Single character as string, or empty string if out of bounds
 */
TrqString* trq_string_char_at(TrqString* str, int64_t index);

/**
 * Substring by character index (UTF-8 aware).
 * @param str Source string
 * @param start Start character index
 * @param len Number of characters
 * @return New substring
 */
TrqString* trq_string_substr_chars(TrqString* str, int64_t start, int64_t len);

/*============================================================================
 * Array Operations
 *============================================================================*/

/**
 * Tarqeem array structure.
 * Dynamic arrays with reference counting.
 */
struct TrqArray {
    int64_t len;       // Number of elements
    int64_t cap;       // Capacity
    int64_t elem_size; // Size of each element in bytes
    void* data;        // Element data
};

/**
 * Create a new array.
 * @param len Initial length
 * @param elem_size Size of each element
 * @return Pointer to new array
 */
TrqArray* trq_array_new(int64_t len, int64_t elem_size);

/**
 * Get array length.
 * @param arr Array
 * @return Number of elements
 */
int64_t trq_array_len(TrqArray* arr);

/**
 * Get pointer to array element.
 * @param arr Array
 * @param index Element index
 * @return Pointer to element
 */
void* trq_array_get(TrqArray* arr, int64_t index);

/**
 * Set array element.
 * @param arr Array
 * @param index Element index
 * @param value Pointer to value
 */
void trq_array_set(TrqArray* arr, int64_t index, void* value);

/**
 * Push element to end of array.
 * @param arr Array
 * @param value Pointer to value
 * @param elem_size Size of element
 */
void trq_array_push(TrqArray* arr, void* value, int64_t elem_size);

/**
 * Pop element from end of array.
 * @param arr Array
 * @return Pointer to popped element (valid until next operation)
 */
void* trq_array_pop(TrqArray* arr);

/*============================================================================
 * I/O Operations
 *============================================================================*/

/**
 * Print a string to stdout.
 * @param str String to print
 */
void trq_print(TrqString* str);

/**
 * Print an integer to stdout.
 * @param value Integer to print
 */
void trq_print_int(int64_t value);

/**
 * Print a float to stdout.
 * @param value Float to print
 */
void trq_print_float(double value);

/**
 * Print a boolean to stdout.
 * @param value Boolean to print
 */
void trq_print_bool(bool value);

/**
 * Print a newline to stdout.
 */
void trq_print_newline(void);

/**
 * Print an array to stdout.
 * @param arr Array to print
 *
 * NOTE: Current implementation only supports printing integer arrays.
 * Non-integer element types will be cast to integers for printing.
 * TODO: Add element type tracking to TrqArray for proper polymorphic printing.
 */
void trq_print_array(TrqArray* arr);

/**
 * Read a line from stdin.
 * @return String containing the line (without newline)
 *
 * NOTE: Returns empty string on both EOF and error.
 * Use trq_file_eof() with file handles for proper EOF detection.
 * TODO: Consider returning NULL on EOF for better error handling.
 */
TrqString* trq_input(void);

/**
 * Read a line from stdin with a prompt.
 * @param prompt Prompt to display
 * @return String containing the line
 */
TrqString* trq_input_prompt(TrqString* prompt);

/*============================================================================
 * Math Operations
 *============================================================================*/

/**
 * Integer power function.
 * @param base Base value
 * @param exp Exponent
 * @return base^exp
 */
int64_t trq_pow_int(int64_t base, int64_t exp);

/**
 * Float power function.
 * @param base Base value
 * @param exp Exponent
 * @return base^exp
 */
double trq_pow_float(double base, double exp);

/**
 * Integer absolute value.
 * @param value Value
 * @return Absolute value
 */
int64_t trq_abs_int(int64_t value);

/**
 * Float absolute value.
 * @param value Value
 * @return Absolute value
 */
double trq_abs_float(double value);

/**
 * Square root.
 * @param value Value
 * @return Square root
 */
double trq_sqrt(double value);

/**
 * Cube root.
 * @param value Value
 * @return Cube root
 */
double trq_cbrt(double value);

/**
 * Nth root.
 * @param value Value
 * @param n Root degree
 * @return Nth root
 */
double trq_nroot(double value, int64_t n);

/**
 * Natural logarithm (ln).
 * @param value Value
 * @return Natural logarithm
 */
double trq_log(double value);

/**
 * Base-10 logarithm.
 * @param value Value
 * @return Log base 10
 */
double trq_log10(double value);

/**
 * Base-2 logarithm.
 * @param value Value
 * @return Log base 2
 */
double trq_log2(double value);

/**
 * Exponential function (e^x).
 * @param value Exponent
 * @return e^value
 */
double trq_exp(double value);

/**
 * Floor function.
 * @param value Value
 * @return Largest integer <= value
 */
double trq_floor(double value);

/**
 * Ceiling function.
 * @param value Value
 * @return Smallest integer >= value
 */
double trq_ceil(double value);

/**
 * Round to nearest integer.
 * @param value Value
 * @return Nearest integer
 */
double trq_round(double value);

/**
 * Truncate toward zero.
 * @param value Value
 * @return Integer part
 */
double trq_trunc(double value);

/**
 * Integer minimum.
 * @param a First value
 * @param b Second value
 * @return Minimum
 */
int64_t trq_min_int(int64_t a, int64_t b);

/**
 * Integer maximum.
 * @param a First value
 * @param b Second value
 * @return Maximum
 */
int64_t trq_max_int(int64_t a, int64_t b);

/**
 * Float minimum.
 * @param a First value
 * @param b Second value
 * @return Minimum
 */
double trq_min_float(double a, double b);

/**
 * Float maximum.
 * @param a First value
 * @param b Second value
 * @return Maximum
 */
double trq_max_float(double a, double b);

/**
 * Clamp integer to range.
 * @param value Value to clamp
 * @param min Minimum
 * @param max Maximum
 * @return Clamped value
 */
int64_t trq_clamp_int(int64_t value, int64_t min, int64_t max);

/**
 * Clamp float to range.
 * @param value Value to clamp
 * @param min Minimum
 * @param max Maximum
 * @return Clamped value
 */
double trq_clamp_float(double value, double min, double max);

/**
 * Sign function.
 * @param value Value
 * @return -1, 0, or 1
 */
int64_t trq_sign(int64_t value);

/**
 * Integer modulo (always positive result).
 * @param a Dividend
 * @param b Divisor
 * @return Remainder (always >= 0)
 */
int64_t trq_mod(int64_t a, int64_t b);

/**
 * Greatest common divisor.
 * @param a First value
 * @param b Second value
 * @return GCD
 */
int64_t trq_gcd(int64_t a, int64_t b);

/**
 * Least common multiple.
 * @param a First value
 * @param b Second value
 * @return LCM
 */
int64_t trq_lcm(int64_t a, int64_t b);

/**
 * Factorial.
 * @param n Value
 * @return n!
 */
int64_t trq_factorial(int64_t n);

/* Trigonometric functions */
double trq_sin(double value);
double trq_cos(double value);
double trq_tan(double value);
double trq_cot(double value);
double trq_sec(double value);
double trq_csc(double value);

/* Inverse trigonometric functions */
double trq_asin(double value);
double trq_acos(double value);
double trq_atan(double value);
double trq_atan2(double y, double x);

/* Hyperbolic functions */
double trq_sinh(double value);
double trq_cosh(double value);
double trq_tanh(double value);

/* Conversion */
double trq_to_radians(double degrees);
double trq_to_degrees(double radians);

/* Random number generation */

/**
 * Seed the random number generator.
 * @param seed Seed value
 */
void trq_random_seed(int64_t seed);

/**
 * Get random integer.
 * @return Random integer
 */
int64_t trq_random_int(void);

/**
 * Get random integer in range [min, max].
 * @param min Minimum
 * @param max Maximum
 * @return Random integer in range
 */
int64_t trq_random_int_range(int64_t min, int64_t max);

/**
 * Get random float in [0, 1).
 * @return Random float
 */
double trq_random_float(void);

/**
 * Get random float in range [min, max).
 * @param min Minimum
 * @param max Maximum
 * @return Random float in range
 */
double trq_random_float_range(double min, double max);

/**
 * Get random boolean.
 * @return Random boolean
 */
bool trq_random_bool(void);

/*============================================================================
 * File System Operations
 *============================================================================*/

/**
 * File mode enumeration.
 */
typedef enum {
    TRQ_FILE_READ = 0,
    TRQ_FILE_WRITE = 1,
    TRQ_FILE_APPEND = 2,
    TRQ_FILE_READ_WRITE = 3
} TrqFileMode;

/**
 * Check if file exists.
 * @param path File path
 * @return true if file exists
 */
bool trq_file_exists(TrqString* path);

/**
 * Check if path is a file.
 * @param path Path
 * @return true if path is a file
 */
bool trq_file_is_file(TrqString* path);

/**
 * Check if path is a directory.
 * @param path Path
 * @return true if path is a directory
 */
bool trq_file_is_dir(TrqString* path);

/**
 * Read entire file contents.
 * @param path File path
 * @return File contents as string, or empty string on error
 */
TrqString* trq_file_read(TrqString* path);

/**
 * Write content to file (overwrites).
 * @param path File path
 * @param content Content to write
 * @return true on success
 */
bool trq_file_write(TrqString* path, TrqString* content);

/**
 * Append content to file.
 * @param path File path
 * @param content Content to append
 * @return true on success
 */
bool trq_file_append(TrqString* path, TrqString* content);

/**
 * Delete file.
 * @param path File path
 * @return true on success
 */
bool trq_file_delete(TrqString* path);

/**
 * Copy file.
 * @param src Source path
 * @param dst Destination path
 * @return true on success
 */
bool trq_file_copy(TrqString* src, TrqString* dst);

/**
 * Move/rename file.
 * @param src Source path
 * @param dst Destination path
 * @return true on success
 */
bool trq_file_move(TrqString* src, TrqString* dst);

/**
 * Get file size in bytes.
 * @param path File path
 * @return File size or -1 on error
 */
int64_t trq_file_size(TrqString* path);

/**
 * Create directory.
 * @param path Directory path
 * @return true on success
 */
bool trq_dir_create(TrqString* path);

/**
 * Create directory and parents.
 * @param path Directory path
 * @return true on success
 */
bool trq_dir_create_all(TrqString* path);

/**
 * Delete directory (must be empty).
 * @param path Directory path
 * @return true on success
 */
bool trq_dir_delete(TrqString* path);

/**
 * List directory contents.
 * @param path Directory path
 * @return Array of file names
 */
TrqArray* trq_dir_list(TrqString* path);

/**
 * Get current working directory.
 * @return Current directory path
 */
TrqString* trq_dir_current(void);

/**
 * Get home directory.
 * @return Home directory path
 */
TrqString* trq_dir_home(void);

/**
 * Get temp directory.
 * @return Temp directory path
 */
TrqString* trq_dir_temp(void);

/* Path operations */

/**
 * Join path components.
 * @param base Base path
 * @param component Component to join
 * @return Combined path
 */
TrqString* trq_path_join(TrqString* base, TrqString* component);

/**
 * Get parent directory.
 * @param path Path
 * @return Parent directory path
 */
TrqString* trq_path_parent(TrqString* path);

/**
 * Get file name from path.
 * @param path Path
 * @return File name
 */
TrqString* trq_path_filename(TrqString* path);

/**
 * Get file extension.
 * @param path Path
 * @return Extension (without dot)
 */
TrqString* trq_path_extension(TrqString* path);

/**
 * Get file name without extension.
 * @param path Path
 * @return Stem
 */
TrqString* trq_path_stem(TrqString* path);

/**
 * Convert to absolute path.
 * @param path Relative path
 * @return Absolute path
 */
TrqString* trq_path_absolute(TrqString* path);

/**
 * Check if path is absolute.
 * @param path Path
 * @return true if absolute
 */
bool trq_path_is_absolute(TrqString* path);

/**
 * Get path separator.
 * @return "/" on Unix, "\\" on Windows
 */
TrqString* trq_path_separator(void);

/*============================================================================
 * Console I/O
 *============================================================================*/

/**
 * Print formatted output (like printf).
 * @param format Format string
 * @param ... Arguments
 */
void trq_printf(const char* format, ...);

/**
 * Print to stderr.
 * @param str String to print
 */
void trq_print_error(TrqString* str);

/**
 * Read integer from input.
 * @return Parsed integer
 */
int64_t trq_input_int(void);

/**
 * Read float from input.
 * @return Parsed float
 */
double trq_input_float(void);

/*============================================================================
 * Exception Handling
 *============================================================================*/

/**
 * Exception structure.
 */
typedef struct TrqException {
    TrqString* message;
    TrqString* type;
    void* stack_trace;
} TrqException;

/**
 * Throw an exception.
 * @param exception Exception to throw
 */
void trq_throw(TrqException* exception);

/**
 * Get the current exception.
 * @return Current exception or NULL
 */
TrqException* trq_get_exception(void);

/**
 * Clear the current exception.
 */
void trq_clear_exception(void);

/*============================================================================
 * Type Checking
 *============================================================================*/

/**
 * Get type name of a value.
 * @param value Pointer to value
 * @return Type name string
 */
TrqString* trq_type_of(void* value);

/*============================================================================
 * Networking Operations
 *============================================================================*/

/**
 * TCP connection info structure.
 */
typedef struct TrqTcpInfo {
    TrqString* address;
    int64_t port;
    int64_t handle;
} TrqTcpInfo;

/**
 * Connect to TCP server.
 * @param address Server address
 * @param port Server port
 * @param timeout_ms Connection timeout in milliseconds
 * @return Socket handle or -1 on error
 */
int64_t trq_tcp_connect(TrqString* address, int64_t port, int64_t timeout_ms);

/**
 * Close TCP connection.
 * @param handle Socket handle
 */
void trq_tcp_close(int64_t handle);

/**
 * Send data over TCP.
 * @param handle Socket handle
 * @param data Data to send
 * @return true on success
 */
bool trq_tcp_send(int64_t handle, TrqString* data);

/**
 * Send bytes over TCP.
 * @param handle Socket handle
 * @param data Byte array to send
 * @return true on success
 */
bool trq_tcp_send_bytes(int64_t handle, TrqArray* data);

/**
 * Receive data from TCP.
 * @param handle Socket handle
 * @param timeout_ms Receive timeout in milliseconds
 * @return Received data or empty string on error
 */
TrqString* trq_tcp_receive(int64_t handle, int64_t timeout_ms);

/**
 * Receive bytes from TCP.
 * @param handle Socket handle
 * @param size Number of bytes to receive
 * @param timeout_ms Receive timeout in milliseconds
 * @return Byte array or empty array on error
 */
TrqArray* trq_tcp_receive_bytes(int64_t handle, int64_t size, int64_t timeout_ms);

/**
 * Receive until delimiter.
 * @param handle Socket handle
 * @param delimiter Delimiter string
 * @param timeout_ms Receive timeout in milliseconds
 * @return Received data or empty string on error
 */
TrqString* trq_tcp_receive_until(int64_t handle, TrqString* delimiter, int64_t timeout_ms);

/**
 * Check if data is available.
 * @param handle Socket handle
 * @return true if data is available
 */
bool trq_tcp_available(int64_t handle);

/**
 * Create TCP server socket.
 * @param address Bind address
 * @param port Port number
 * @param backlog Connection queue size
 * @return Server socket handle or -1 on error
 */
int64_t trq_tcp_listen(TrqString* address, int64_t port, int64_t backlog);

/**
 * Accept TCP connection.
 * @param handle Server socket handle
 * @return Connection info structure
 */
TrqTcpInfo* trq_tcp_accept(int64_t handle);

/**
 * Accept TCP connection with timeout.
 * @param handle Server socket handle
 * @param timeout_ms Accept timeout in milliseconds
 * @return Connection info structure (handle -1 on timeout)
 */
TrqTcpInfo* trq_tcp_accept_timeout(int64_t handle, int64_t timeout_ms);

/**
 * Get local address of socket.
 * @param handle Socket handle
 * @return Local address string
 */
TrqString* trq_tcp_local_address(int64_t handle);

/**
 * Get local port of socket.
 * @param handle Socket handle
 * @return Local port number
 */
int64_t trq_tcp_local_port(int64_t handle);

/* UDP Operations */

/**
 * Bind UDP socket.
 * @param port Port number
 * @return Socket handle or -1 on error
 */
int64_t trq_udp_bind(int64_t port);

/**
 * Close UDP socket.
 * @param handle Socket handle
 */
void trq_udp_close(int64_t handle);

/**
 * Send UDP datagram.
 * @param handle Socket handle
 * @param address Destination address
 * @param port Destination port
 * @param data Data to send
 * @return true on success
 */
bool trq_udp_send_to(int64_t handle, TrqString* address, int64_t port, TrqString* data);

/**
 * Send UDP bytes.
 * @param handle Socket handle
 * @param address Destination address
 * @param port Destination port
 * @param data Byte array to send
 * @return true on success
 */
bool trq_udp_send_bytes_to(int64_t handle, TrqString* address, int64_t port, TrqArray* data);

/**
 * Receive UDP datagram.
 * @param handle Socket handle
 * @param timeout_ms Receive timeout in milliseconds
 * @return Received data or empty string on timeout
 */
TrqString* trq_udp_receive(int64_t handle, int64_t timeout_ms);

/**
 * Receive UDP bytes.
 * @param handle Socket handle
 * @param size Maximum bytes to receive
 * @param timeout_ms Receive timeout in milliseconds
 * @return Byte array or empty array on timeout
 */
TrqArray* trq_udp_receive_bytes(int64_t handle, int64_t size, int64_t timeout_ms);

/**
 * Reply to last sender.
 * @param handle Socket handle
 * @param data Data to send
 * @return true on success
 */
bool trq_udp_reply(int64_t handle, TrqString* data);

/**
 * Resolve hostname to IP.
 * @param hostname Hostname to resolve
 * @return IP address string or empty on error
 */
TrqString* trq_resolve_hostname(TrqString* hostname);

/**
 * Get local IP address.
 * @return Local IP address string
 */
TrqString* trq_get_local_ip(void);

/* HTTP Operations */

/**
 * HTTP response structure.
 */
typedef struct TrqHttpResponse {
    int64_t status;
    TrqString* status_text;
    TrqString* body;
    TrqArray* headers;
} TrqHttpResponse;

/**
 * Perform HTTP request.
 * @param method HTTP method
 * @param url Request URL
 * @param headers Request headers array
 * @param body Request body
 * @param timeout_ms Request timeout in milliseconds
 * @param follow_redirects Whether to follow redirects
 * @return HTTP response structure
 */
TrqHttpResponse* trq_http_request(
    TrqString* method,
    TrqString* url,
    TrqArray* headers,
    TrqString* body,
    int64_t timeout_ms,
    bool follow_redirects
);

/**
 * Download file from URL.
 * @param url Source URL
 * @param path Destination path
 * @return true on success
 */
bool trq_http_download(TrqString* url, TrqString* path);

/**
 * URL encode string.
 * @param str String to encode
 * @return URL-encoded string
 */
TrqString* trq_url_encode(TrqString* str);

/**
 * URL decode string.
 * @param str String to decode
 * @return URL-decoded string
 */
TrqString* trq_url_decode(TrqString* str);

/**
 * Base64 encode string.
 * @param str String to encode
 * @return Base64-encoded string
 */
TrqString* trq_base64_encode(TrqString* str);

/**
 * Base64 decode string.
 * @param str String to decode
 * @return Decoded string
 */
TrqString* trq_base64_decode(TrqString* str);

/*============================================================================
 * Date and Time Operations
 *============================================================================*/

/**
 * Date structure.
 */
typedef struct TrqDate {
    int64_t year;
    int64_t month;
    int64_t day;
} TrqDate;

/**
 * Time structure.
 */
typedef struct TrqTime {
    int64_t hour;
    int64_t minute;
    int64_t second;
    int64_t millisecond;
} TrqTime;

/**
 * DateTime structure.
 */
typedef struct TrqDateTime {
    int64_t year;
    int64_t month;
    int64_t day;
    int64_t hour;
    int64_t minute;
    int64_t second;
    int64_t millisecond;
} TrqDateTime;

/**
 * Get today's date.
 * @return Date structure
 */
TrqDate* trq_date_today(void);

/**
 * Parse date from ISO string.
 * @param str Date string (YYYY-MM-DD)
 * @return Date structure
 */
TrqDate* trq_date_parse(TrqString* str);

/**
 * Create date from Unix timestamp.
 * @param timestamp Unix timestamp in seconds
 * @return Date structure
 */
TrqDate* trq_date_from_timestamp(int64_t timestamp);

/**
 * Add days to date.
 * @param year Year
 * @param month Month
 * @param day Day
 * @param days Days to add
 * @return New date structure
 */
TrqDate* trq_date_add_days(int64_t year, int64_t month, int64_t day, int64_t days);

/**
 * Add months to date.
 * @param year Year
 * @param month Month
 * @param day Day
 * @param months Months to add
 * @return New date structure
 */
TrqDate* trq_date_add_months(int64_t year, int64_t month, int64_t day, int64_t months);

/**
 * Get difference in days between two dates.
 * @return Number of days
 */
int64_t trq_date_diff_days(int64_t y1, int64_t m1, int64_t d1, int64_t y2, int64_t m2, int64_t d2);

/**
 * Get day of week (0 = Sunday).
 * @return Day of week (0-6)
 */
int64_t trq_day_of_week(int64_t year, int64_t month, int64_t day);

/**
 * Get day of year (1-366).
 * @return Day of year
 */
int64_t trq_day_of_year(int64_t year, int64_t month, int64_t day);

/**
 * Get ISO week number.
 * @return Week number (1-53)
 */
int64_t trq_week_number(int64_t year, int64_t month, int64_t day);

/**
 * Get days in month.
 * @param year Year
 * @param month Month
 * @return Number of days in month
 */
int64_t trq_days_in_month(int64_t year, int64_t month);

/**
 * Convert date to Unix timestamp.
 * @return Unix timestamp in seconds
 */
int64_t trq_date_to_timestamp(int64_t year, int64_t month, int64_t day);

/**
 * Format date using pattern.
 * @param year Year
 * @param month Month
 * @param day Day
 * @param pattern Format pattern
 * @return Formatted date string
 */
TrqString* trq_date_format(int64_t year, int64_t month, int64_t day, TrqString* pattern);

/**
 * Get current time.
 * @return Time structure
 */
TrqTime* trq_time_now(void);

/**
 * Parse time from string.
 * @param str Time string (HH:MM:SS)
 * @return Time structure
 */
TrqTime* trq_time_parse(TrqString* str);

/**
 * Format time using pattern.
 * @return Formatted time string
 */
TrqString* trq_time_format(int64_t hour, int64_t minute, int64_t second, int64_t milli, TrqString* pattern);

/**
 * Get current datetime.
 * @return DateTime structure
 */
TrqDateTime* trq_datetime_now(void);

/**
 * Create datetime from Unix timestamp.
 * @param timestamp Unix timestamp in seconds
 * @return DateTime structure
 */
TrqDateTime* trq_datetime_from_timestamp(int64_t timestamp);

/**
 * Parse datetime from ISO string.
 * @param str DateTime string (YYYY-MM-DDTHH:MM:SS)
 * @return DateTime structure
 */
TrqDateTime* trq_datetime_parse(TrqString* str);

/**
 * Format datetime using pattern.
 * @return Formatted datetime string
 */
TrqString* trq_datetime_format(
    int64_t year, int64_t month, int64_t day,
    int64_t hour, int64_t minute, int64_t second,
    TrqString* pattern
);

/**
 * Sleep for milliseconds.
 * @param milliseconds Duration to sleep
 */
void trq_sleep(int64_t milliseconds);

/**
 * Get high-resolution time in milliseconds.
 * @return Milliseconds since some fixed point
 */
int64_t trq_performance_now(void);

/*============================================================================
 * Panic / Assert
 *============================================================================*/

/**
 * Panic with message (terminates program).
 * @param message Panic message
 */
void trq_panic(TrqString* message);

/*============================================================================
 * Runtime Initialization
 *============================================================================*/

/**
 * Initialize the Tarqeem runtime.
 * Called automatically before main.
 */
void trq_runtime_init(void);

/**
 * Cleanup the Tarqeem runtime.
 * Called automatically after main.
 */
void trq_runtime_cleanup(void);

#ifdef __cplusplus
}
#endif

#endif /* TARQEEM_RT_H */
