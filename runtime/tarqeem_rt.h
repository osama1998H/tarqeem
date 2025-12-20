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

/*============================================================================
 * Array Operations
 *============================================================================*/

/**
 * Tarqeem array structure.
 * Dynamic arrays with reference counting.
 */
typedef struct TrqArray {
    int64_t len;      // Number of elements
    int64_t cap;      // Capacity
    void* data;       // Element data
} TrqArray;

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
 * Read a line from stdin.
 * @return String containing the line (without newline)
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
