/**
 * Tarqeem WebAssembly Runtime Library
 * مكتبة وقت التشغيل لـ WebAssembly
 *
 * This header defines the runtime functions for Tarqeem programs
 * compiled to WebAssembly. It provides a minimal runtime that works
 * in both browser and WASI environments.
 */

#ifndef TARQEEM_WASM_H
#define TARQEEM_WASM_H

#include <stdint.h>
#include <stddef.h>

/* WASM uses 32-bit pointers, but we use 64-bit integers for arithmetic */
typedef int64_t TrqInt;
typedef double TrqFloat;
typedef int32_t TrqBool;

/*============================================================================
 * JavaScript Import Declarations
 *
 * These functions are provided by the JavaScript runtime and must be
 * imported when instantiating the WebAssembly module.
 *============================================================================*/

/* I/O operations - implemented in JavaScript */
extern void __trq_js_print(const char* ptr, int32_t len);
extern void __trq_js_print_int(int64_t value);
extern void __trq_js_print_float(double value);
extern void __trq_js_print_bool(int32_t value);
extern void __trq_js_print_newline(void);
extern int32_t __trq_js_input(char* buf, int32_t max_len);

/* Math operations - fallback to JavaScript if needed */
extern double __trq_js_sqrt(double x);
extern double __trq_js_pow(double base, double exp);
extern double __trq_js_sin(double x);
extern double __trq_js_cos(double x);
extern double __trq_js_tan(double x);
extern double __trq_js_floor(double x);
extern double __trq_js_ceil(double x);
extern double __trq_js_round(double x);
extern double __trq_js_abs(double x);
extern double __trq_js_log(double x);
extern double __trq_js_exp(double x);

/* Time operations */
extern int64_t __trq_js_now(void);
extern double __trq_js_performance_now(void);

/*============================================================================
 * Memory Management
 *
 * Simple bump allocator for WASM linear memory. Reference counting is
 * supported but cyclic references require manual handling.
 *============================================================================*/

/**
 * Allocate memory with reference counting header.
 * @param size Number of bytes to allocate
 * @return Pointer to allocated memory (after header)
 */
void* trq_alloc(int32_t size);

/**
 * Reallocate memory.
 * @param ptr Pointer to existing allocation
 * @param new_size New size in bytes
 * @return Pointer to reallocated memory
 */
void* trq_realloc(void* ptr, int32_t new_size);

/**
 * Free memory (decrements refcount, frees if zero).
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
int32_t trq_refcount(void* ptr);

/**
 * Get current heap usage.
 * @return Bytes used in heap
 */
int32_t trq_heap_used(void);

/**
 * Grow WASM memory by specified number of pages.
 * @param pages Number of 64KB pages to add
 * @return Previous size in pages, or -1 on failure
 */
int32_t trq_memory_grow(int32_t pages);

/*============================================================================
 * String Operations
 *============================================================================*/

/**
 * Tarqeem string structure (WASM version with 32-bit pointers).
 */
typedef struct TrqString {
    int32_t len;      /* Length in bytes */
    int32_t cap;      /* Capacity in bytes */
    char* data;       /* UTF-8 data (null-terminated) */
} TrqString;

/**
 * Create a new string from raw data.
 * @param data UTF-8 string data
 * @param len Length in bytes
 * @return Pointer to new string
 */
TrqString* trq_string_new(const char* data, int32_t len);

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
int32_t trq_string_len(TrqString* str);

/**
 * Compare two strings.
 * @param a First string
 * @param b Second string
 * @return <0 if a<b, 0 if a==b, >0 if a>b
 */
int32_t trq_string_compare(TrqString* a, TrqString* b);

/**
 * Check if two strings are equal.
 * @param a First string
 * @param b Second string
 * @return 1 if equal, 0 otherwise
 */
TrqBool trq_string_equals(TrqString* a, TrqString* b);

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
 * @return "صحيح" or "خطأ"
 */
TrqString* trq_bool_to_string(TrqBool value);

/*============================================================================
 * Array Operations
 *============================================================================*/

/**
 * Tarqeem array structure (WASM version).
 */
typedef struct TrqArray {
    int32_t len;       /* Number of elements */
    int32_t cap;       /* Capacity */
    int32_t elem_size; /* Size of each element in bytes */
    void* data;        /* Element data */
} TrqArray;

/**
 * Create a new array.
 * @param len Initial length
 * @param elem_size Size of each element
 * @return Pointer to new array
 */
TrqArray* trq_array_new(int32_t len, int32_t elem_size);

/**
 * Get array length.
 * @param arr Array
 * @return Number of elements
 */
int32_t trq_array_len(TrqArray* arr);

/**
 * Get pointer to array element.
 * @param arr Array
 * @param index Element index
 * @return Pointer to element
 */
void* trq_array_get(TrqArray* arr, int32_t index);

/**
 * Set array element.
 * @param arr Array
 * @param index Element index
 * @param value Pointer to value
 */
void trq_array_set(TrqArray* arr, int32_t index, void* value);

/**
 * Push element to end of array.
 * @param arr Array
 * @param value Pointer to value
 * @param elem_size Size of element
 */
void trq_array_push(TrqArray* arr, void* value, int32_t elem_size);

/**
 * Pop element from end of array.
 * @param arr Array
 * @return Pointer to popped element
 */
void* trq_array_pop(TrqArray* arr);

/*============================================================================
 * I/O Operations
 *============================================================================*/

/**
 * Print a string.
 * @param str String to print
 */
void trq_print(TrqString* str);

/**
 * Print an integer.
 * @param value Integer to print
 */
void trq_print_int(int64_t value);

/**
 * Print a float.
 * @param value Float to print
 */
void trq_print_float(double value);

/**
 * Print a boolean.
 * @param value Boolean to print
 */
void trq_print_bool(TrqBool value);

/**
 * Print a newline.
 */
void trq_print_newline(void);

/**
 * Read a line of input.
 * @return String containing the input
 */
TrqString* trq_input(void);

/*============================================================================
 * Math Operations
 *============================================================================*/

int64_t trq_pow_int(int64_t base, int64_t exp);
double trq_pow_float(double base, double exp);
int64_t trq_abs_int(int64_t value);
double trq_abs_float(double value);
double trq_sqrt(double value);
double trq_floor(double value);
double trq_ceil(double value);
double trq_round(double value);
int64_t trq_min_int(int64_t a, int64_t b);
int64_t trq_max_int(int64_t a, int64_t b);
double trq_min_float(double a, double b);
double trq_max_float(double a, double b);

/* Trigonometric functions */
double trq_sin(double value);
double trq_cos(double value);
double trq_tan(double value);

/* Logarithmic functions */
double trq_log(double value);
double trq_exp(double value);

/*============================================================================
 * Runtime Initialization
 *============================================================================*/

/**
 * Initialize the Tarqeem WASM runtime.
 * Called automatically at module start.
 */
void trq_runtime_init(void);

/**
 * Panic with message (terminates execution).
 * @param message Panic message
 */
void trq_panic(TrqString* message);

#endif /* TARQEEM_WASM_H */
