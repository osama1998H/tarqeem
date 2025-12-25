/**
 * Tarqeem WebAssembly Runtime Implementation
 *
 * This file implements the runtime functions for Tarqeem programs
 * compiled to WebAssembly.
 */

#include "tarqeem_wasm.h"

/*============================================================================
 * Memory Layout
 *
 * WASM linear memory is organized as follows:
 * [0 - 1023]     : Reserved (null pointer trap zone)
 * [1024 - 4095]  : Static data and string literals
 * [4096 - ...]   : Heap (managed by bump allocator)
 *
 * Each allocated block has a 8-byte header:
 * - 4 bytes: reference count
 * - 4 bytes: block size
 *============================================================================*/

#define HEAP_START 4096
#define REFCOUNT_OFFSET (-8)
#define SIZE_OFFSET (-4)
#define HEADER_SIZE 8
#define ALIGNMENT 8

/* Global heap pointer */
static uint32_t heap_ptr = HEAP_START;
static uint32_t heap_end = 0; /* Set during init based on memory size */

/* WASM memory grow intrinsic */
int32_t __builtin_wasm_memory_grow(int32_t index, int32_t pages);
int32_t __builtin_wasm_memory_size(int32_t index);

/*============================================================================
 * Memory Management
 *============================================================================*/

void trq_runtime_init(void) {
    /* Calculate initial heap end based on memory size */
    int32_t pages = __builtin_wasm_memory_size(0);
    heap_end = pages * 65536; /* 64KB per page */
}

void* trq_alloc(int32_t size) {
    if (size <= 0) {
        return (void*)0;
    }

    /* Align size to 8 bytes */
    size = (size + ALIGNMENT - 1) & ~(ALIGNMENT - 1);

    /* Add header space */
    int32_t total_size = size + HEADER_SIZE;

    /* Check if we need to grow memory */
    if (heap_ptr + total_size > heap_end) {
        int32_t needed_pages = ((heap_ptr + total_size - heap_end) / 65536) + 1;
        int32_t result = trq_memory_grow(needed_pages);
        if (result == -1) {
            /* Out of memory */
            return (void*)0;
        }
    }

    /* Allocate */
    uint32_t block_addr = heap_ptr;
    heap_ptr += total_size;

    /* Write header */
    uint32_t* header = (uint32_t*)block_addr;
    header[0] = 1;    /* refcount = 1 */
    header[1] = size; /* block size */

    /* Return pointer after header */
    return (void*)(block_addr + HEADER_SIZE);
}

void* trq_realloc(void* ptr, int32_t new_size) {
    if (ptr == (void*)0) {
        return trq_alloc(new_size);
    }
    if (new_size <= 0) {
        trq_free(ptr);
        return (void*)0;
    }

    /* Get current size */
    uint32_t* header = (uint32_t*)((char*)ptr - HEADER_SIZE);
    int32_t old_size = header[1];

    if (new_size <= old_size) {
        return ptr; /* No need to reallocate */
    }

    /* Allocate new block and copy */
    void* new_ptr = trq_alloc(new_size);
    if (new_ptr == (void*)0) {
        return (void*)0;
    }

    /* Copy data */
    char* src = (char*)ptr;
    char* dst = (char*)new_ptr;
    for (int32_t i = 0; i < old_size; i++) {
        dst[i] = src[i];
    }

    /* Free old block */
    trq_free(ptr);

    return new_ptr;
}

void trq_free(void* ptr) {
    if (ptr == (void*)0) {
        return;
    }
    /* In a bump allocator, we don't actually free memory */
    /* This is a trade-off for simplicity in WASM */
    /* Reference counting handles logical freeing */
}

void trq_retain(void* ptr) {
    if (ptr == (void*)0) {
        return;
    }
    uint32_t* refcount = (uint32_t*)((char*)ptr - HEADER_SIZE);
    (*refcount)++;
}

void trq_release(void* ptr) {
    if (ptr == (void*)0) {
        return;
    }
    uint32_t* refcount = (uint32_t*)((char*)ptr - HEADER_SIZE);
    if (*refcount > 0) {
        (*refcount)--;
        /* Note: In a bump allocator, we don't actually free */
        /* A more sophisticated allocator could reclaim memory here */
    }
}

int32_t trq_refcount(void* ptr) {
    if (ptr == (void*)0) {
        return 0;
    }
    uint32_t* refcount = (uint32_t*)((char*)ptr - HEADER_SIZE);
    return *refcount;
}

int32_t trq_heap_used(void) {
    return heap_ptr - HEAP_START;
}

int32_t trq_memory_grow(int32_t pages) {
    int32_t old_pages = __builtin_wasm_memory_grow(0, pages);
    if (old_pages != -1) {
        heap_end = (old_pages + pages) * 65536;
    }
    return old_pages;
}

/*============================================================================
 * String Operations
 *============================================================================*/

TrqString* trq_string_new(const char* data, int32_t len) {
    TrqString* str = (TrqString*)trq_alloc(sizeof(TrqString));
    if (str == (void*)0) {
        return (void*)0;
    }

    str->len = len;
    str->cap = len + 1;
    str->data = (char*)trq_alloc(str->cap);

    if (str->data == (void*)0) {
        return (void*)0;
    }

    /* Copy data */
    for (int32_t i = 0; i < len; i++) {
        str->data[i] = data[i];
    }
    str->data[len] = '\0';

    return str;
}

TrqString* trq_string_from_cstr(const char* cstr) {
    int32_t len = 0;
    while (cstr[len] != '\0') {
        len++;
    }
    return trq_string_new(cstr, len);
}

TrqString* trq_string_concat(TrqString* left, TrqString* right) {
    if (left == (void*)0 || right == (void*)0) {
        return (void*)0;
    }

    int32_t new_len = left->len + right->len;
    TrqString* result = (TrqString*)trq_alloc(sizeof(TrqString));
    if (result == (void*)0) {
        return (void*)0;
    }

    result->len = new_len;
    result->cap = new_len + 1;
    result->data = (char*)trq_alloc(result->cap);

    if (result->data == (void*)0) {
        return (void*)0;
    }

    /* Copy left string */
    for (int32_t i = 0; i < left->len; i++) {
        result->data[i] = left->data[i];
    }

    /* Copy right string */
    for (int32_t i = 0; i < right->len; i++) {
        result->data[left->len + i] = right->data[i];
    }

    result->data[new_len] = '\0';

    return result;
}

int32_t trq_string_len(TrqString* str) {
    if (str == (void*)0) {
        return 0;
    }
    return str->len;
}

int32_t trq_string_compare(TrqString* a, TrqString* b) {
    if (a == (void*)0 && b == (void*)0) return 0;
    if (a == (void*)0) return -1;
    if (b == (void*)0) return 1;

    int32_t min_len = a->len < b->len ? a->len : b->len;

    for (int32_t i = 0; i < min_len; i++) {
        if (a->data[i] < b->data[i]) return -1;
        if (a->data[i] > b->data[i]) return 1;
    }

    if (a->len < b->len) return -1;
    if (a->len > b->len) return 1;
    return 0;
}

TrqBool trq_string_equals(TrqString* a, TrqString* b) {
    return trq_string_compare(a, b) == 0 ? 1 : 0;
}

TrqString* trq_int_to_string(int64_t value) {
    char buffer[32];
    int32_t i = 0;
    int negative = 0;

    if (value == 0) {
        return trq_string_from_cstr("0");
    }

    if (value < 0) {
        negative = 1;
        value = -value;
    }

    while (value > 0) {
        buffer[i++] = '0' + (value % 10);
        value /= 10;
    }

    if (negative) {
        buffer[i++] = '-';
    }

    /* Reverse the string */
    char result[32];
    int32_t j = 0;
    while (i > 0) {
        result[j++] = buffer[--i];
    }
    result[j] = '\0';

    return trq_string_from_cstr(result);
}

TrqString* trq_float_to_string(double value) {
    /* Simple float to string conversion */
    /* For a full implementation, we'd need a proper dtoa algorithm */
    char buffer[64];
    int32_t i = 0;

    if (value < 0) {
        buffer[i++] = '-';
        value = -value;
    }

    int64_t int_part = (int64_t)value;
    double frac_part = value - int_part;

    /* Convert integer part */
    char int_buf[32];
    int32_t j = 0;
    if (int_part == 0) {
        int_buf[j++] = '0';
    } else {
        while (int_part > 0) {
            int_buf[j++] = '0' + (int_part % 10);
            int_part /= 10;
        }
    }
    while (j > 0) {
        buffer[i++] = int_buf[--j];
    }

    /* Decimal point */
    buffer[i++] = '.';

    /* Convert fractional part (6 digits) */
    for (int32_t k = 0; k < 6; k++) {
        frac_part *= 10;
        int digit = (int)frac_part;
        buffer[i++] = '0' + digit;
        frac_part -= digit;
    }

    buffer[i] = '\0';

    return trq_string_from_cstr(buffer);
}

TrqString* trq_bool_to_string(TrqBool value) {
    if (value) {
        /* "صحيح" in UTF-8 */
        return trq_string_from_cstr("\xd8\xb5\xd8\xad\xd9\x8a\xd8\xad");
    } else {
        /* "خطأ" in UTF-8 */
        return trq_string_from_cstr("\xd8\xae\xd8\xb7\xd8\xa3");
    }
}

/*============================================================================
 * Array Operations
 *============================================================================*/

TrqArray* trq_array_new(int32_t len, int32_t elem_size) {
    TrqArray* arr = (TrqArray*)trq_alloc(sizeof(TrqArray));
    if (arr == (void*)0) {
        return (void*)0;
    }

    int32_t cap = len > 8 ? len : 8;
    arr->len = len;
    arr->cap = cap;
    arr->elem_size = elem_size;
    arr->data = trq_alloc(cap * elem_size);

    if (arr->data == (void*)0) {
        return (void*)0;
    }

    /* Zero-initialize */
    char* data = (char*)arr->data;
    for (int32_t i = 0; i < cap * elem_size; i++) {
        data[i] = 0;
    }

    return arr;
}

int32_t trq_array_len(TrqArray* arr) {
    if (arr == (void*)0) {
        return 0;
    }
    return arr->len;
}

void* trq_array_get(TrqArray* arr, int32_t index) {
    if (arr == (void*)0 || index < 0 || index >= arr->len) {
        return (void*)0;
    }
    return (char*)arr->data + (index * arr->elem_size);
}

void trq_array_set(TrqArray* arr, int32_t index, void* value) {
    if (arr == (void*)0 || index < 0 || index >= arr->len || value == (void*)0) {
        return;
    }

    char* dst = (char*)arr->data + (index * arr->elem_size);
    char* src = (char*)value;
    for (int32_t i = 0; i < arr->elem_size; i++) {
        dst[i] = src[i];
    }
}

void trq_array_push(TrqArray* arr, void* value, int32_t elem_size) {
    if (arr == (void*)0 || value == (void*)0) {
        return;
    }

    /* Grow if needed */
    if (arr->len >= arr->cap) {
        int32_t new_cap = arr->cap * 2;
        void* new_data = trq_alloc(new_cap * arr->elem_size);
        if (new_data == (void*)0) {
            return;
        }

        /* Copy old data */
        char* src = (char*)arr->data;
        char* dst = (char*)new_data;
        for (int32_t i = 0; i < arr->len * arr->elem_size; i++) {
            dst[i] = src[i];
        }

        arr->data = new_data;
        arr->cap = new_cap;
    }

    /* Add new element */
    char* dst = (char*)arr->data + (arr->len * arr->elem_size);
    char* src = (char*)value;
    for (int32_t i = 0; i < arr->elem_size; i++) {
        dst[i] = src[i];
    }
    arr->len++;
}

void* trq_array_pop(TrqArray* arr) {
    if (arr == (void*)0 || arr->len == 0) {
        return (void*)0;
    }
    arr->len--;
    return (char*)arr->data + (arr->len * arr->elem_size);
}

/*============================================================================
 * I/O Operations
 *============================================================================*/

void trq_print(TrqString* str) {
    if (str == (void*)0 || str->data == (void*)0) {
        return;
    }
    __trq_js_print(str->data, str->len);
}

void trq_print_int(int64_t value) {
    __trq_js_print_int(value);
}

void trq_print_float(double value) {
    __trq_js_print_float(value);
}

void trq_print_bool(TrqBool value) {
    __trq_js_print_bool(value);
}

void trq_print_newline(void) {
    __trq_js_print_newline();
}

TrqString* trq_input(void) {
    char buffer[1024];
    int32_t len = __trq_js_input(buffer, 1024);
    return trq_string_new(buffer, len);
}

/*============================================================================
 * Math Operations
 *============================================================================*/

int64_t trq_pow_int(int64_t base, int64_t exp) {
    if (exp < 0) return 0;
    if (exp == 0) return 1;

    int64_t result = 1;
    while (exp > 0) {
        if (exp & 1) {
            result *= base;
        }
        base *= base;
        exp >>= 1;
    }
    return result;
}

double trq_pow_float(double base, double exp) {
    return __trq_js_pow(base, exp);
}

int64_t trq_abs_int(int64_t value) {
    return value < 0 ? -value : value;
}

double trq_abs_float(double value) {
    return __trq_js_abs(value);
}

double trq_sqrt(double value) {
    return __trq_js_sqrt(value);
}

double trq_floor(double value) {
    return __trq_js_floor(value);
}

double trq_ceil(double value) {
    return __trq_js_ceil(value);
}

double trq_round(double value) {
    return __trq_js_round(value);
}

int64_t trq_min_int(int64_t a, int64_t b) {
    return a < b ? a : b;
}

int64_t trq_max_int(int64_t a, int64_t b) {
    return a > b ? a : b;
}

double trq_min_float(double a, double b) {
    return a < b ? a : b;
}

double trq_max_float(double a, double b) {
    return a > b ? a : b;
}

double trq_sin(double value) {
    return __trq_js_sin(value);
}

double trq_cos(double value) {
    return __trq_js_cos(value);
}

double trq_tan(double value) {
    return __trq_js_tan(value);
}

double trq_log(double value) {
    return __trq_js_log(value);
}

double trq_exp(double value) {
    return __trq_js_exp(value);
}

/*============================================================================
 * Panic
 *============================================================================*/

void trq_panic(TrqString* message) {
    if (message != (void*)0) {
        TrqString* prefix = trq_string_from_cstr("PANIC: ");
        TrqString* full_msg = trq_string_concat(prefix, message);
        trq_print(full_msg);
        trq_print_newline();
    }
    /* In WASM, we can't really exit, so we just hang */
    /* A more sophisticated approach would use WASM exceptions */
    while (1) {}
}
