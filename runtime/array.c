/**
 * Tarqeem Runtime - Array Operations
 *
 * Implements dynamic arrays with reference counting.
 */

#include "tarqeem_rt.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

/*============================================================================
 * Internal Constants
 *============================================================================*/

#define ARRAY_INITIAL_CAP 8
#define ARRAY_GROWTH_FACTOR 2

/*============================================================================
 * Array Creation
 *============================================================================*/

TrqArray* trq_array_new(int64_t len, int64_t elem_size) {
    TrqArray* arr = (TrqArray*)trq_alloc(sizeof(TrqArray));
    if (!arr) {
        return NULL;
    }

    if (len < 0) {
        len = 0;
    }
    if (elem_size <= 0) {
        elem_size = sizeof(void*);
    }

    int64_t cap = len > ARRAY_INITIAL_CAP ? len : ARRAY_INITIAL_CAP;
    int64_t data_size = cap * elem_size;

    arr->data = malloc(data_size);
    if (!arr->data) {
        trq_free(arr);
        return NULL;
    }

    memset(arr->data, 0, data_size);

    arr->len = len;
    arr->cap = cap;
    arr->elem_size = elem_size;

    return arr;
}

/*============================================================================
 * Array Access
 *============================================================================*/

int64_t trq_array_len(TrqArray* arr) {
    if (!arr) {
        return 0;
    }
    return arr->len;
}

void* trq_array_get(TrqArray* arr, int64_t index) {
    if (!arr || !arr->data) {
        fprintf(stderr, "Error: Array access on null array / خطأ: الوصول إلى مصفوفة فارغة\n");
        return NULL;
    }

    if (index < 0 || index >= arr->len) {
        fprintf(stderr, "Error: Array index out of bounds: %ld (length: %ld) / خطأ: فهرس المصفوفة خارج الحدود\n",
                (long)index, (long)arr->len);
        return NULL;
    }

    return (char*)arr->data + (index * arr->elem_size);
}

void trq_array_set(TrqArray* arr, int64_t index, void* value) {
    if (!arr || !arr->data) {
        fprintf(stderr, "Error: Array access on null array / خطأ: الوصول إلى مصفوفة فارغة\n");
        return;
    }

    if (index < 0 || index >= arr->len) {
        fprintf(stderr, "Error: Array index out of bounds: %ld (length: %ld) / خطأ: فهرس المصفوفة خارج الحدود\n",
                (long)index, (long)arr->len);
        return;
    }

    memcpy((char*)arr->data + (index * arr->elem_size), value, arr->elem_size);
}

/*============================================================================
 * Array Modification
 *============================================================================*/

/**
 * Ensure array has capacity for at least new_cap elements.
 * Uses the array's stored elem_size.
 */
static bool trq_array_ensure_capacity(TrqArray* arr, int64_t new_cap) {
    if (!arr) {
        return false;
    }

    if (new_cap <= arr->cap) {
        return true;
    }

    int64_t cap = arr->cap;
    while (cap < new_cap) {
        cap *= ARRAY_GROWTH_FACTOR;
    }

    int64_t elem_size = arr->elem_size;
    void* new_data = realloc(arr->data, cap * elem_size);
    if (!new_data) {
        return false;
    }

    memset((char*)new_data + (arr->cap * elem_size), 0, (cap - arr->cap) * elem_size);

    arr->data = new_data;
    arr->cap = cap;
    return true;
}

void trq_array_push(TrqArray* arr, void* value, int64_t elem_size) {
    if (!arr) {
        fprintf(stderr, "Error: Push on null array / خطأ: إضافة إلى مصفوفة فارغة\n");
        return;
    }

    if (elem_size <= 0) {
        elem_size = arr->elem_size;
    }

    if (!trq_array_ensure_capacity(arr, arr->len + 1)) {
        fprintf(stderr, "Error: Failed to grow array / خطأ: فشل في توسيع المصفوفة\n");
        return;
    }

    memcpy((char*)arr->data + (arr->len * arr->elem_size), value, arr->elem_size);
    arr->len++;
}

void* trq_array_pop(TrqArray* arr) {
    if (!arr || arr->len == 0) {
        fprintf(stderr, "Error: Pop from empty array / خطأ: إزالة من مصفوفة فارغة\n");
        return NULL;
    }

    arr->len--;
    return (char*)arr->data + (arr->len * arr->elem_size);
}

/*============================================================================
 * Array Utilities
 *============================================================================*/

/**
 * Clone an array.
 */
TrqArray* trq_array_clone(TrqArray* arr, int64_t elem_size) {
    if (!arr) {
        return NULL;
    }

    if (elem_size <= 0) {
        elem_size = arr->elem_size;
    }

    TrqArray* clone = trq_array_new(arr->len, elem_size);
    if (!clone) {
        return NULL;
    }

    memcpy(clone->data, arr->data, arr->len * elem_size);
    return clone;
}

/**
 * Concatenate two arrays.
 */
TrqArray* trq_array_concat(TrqArray* a, TrqArray* b, int64_t elem_size) {
    if (!a && !b) {
        return trq_array_new(0, elem_size > 0 ? elem_size : (int64_t)sizeof(void*));
    }

    if (elem_size <= 0) {
        elem_size = a ? a->elem_size : b->elem_size;
    }

    int64_t len_a = a ? a->len : 0;
    int64_t len_b = b ? b->len : 0;
    int64_t total_len = len_a + len_b;

    TrqArray* result = trq_array_new(total_len, elem_size);
    if (!result) {
        return NULL;
    }

    if (a && len_a > 0) {
        memcpy(result->data, a->data, len_a * elem_size);
    }
    if (b && len_b > 0) {
        memcpy((char*)result->data + (len_a * elem_size), b->data, len_b * elem_size);
    }

    return result;
}

/**
 * Get a slice of an array.
 */
TrqArray* trq_array_slice(TrqArray* arr, int64_t start, int64_t end, int64_t elem_size) {
    if (!arr || !arr->data) {
        return trq_array_new(0, elem_size > 0 ? elem_size : (int64_t)sizeof(void*));
    }

    if (elem_size <= 0) {
        elem_size = arr->elem_size;
    }

    if (start < 0) {
        start = 0;
    }
    if (end > arr->len) {
        end = arr->len;
    }
    if (start >= end) {
        return trq_array_new(0, elem_size);
    }

    int64_t slice_len = end - start;
    TrqArray* result = trq_array_new(slice_len, elem_size);
    if (!result) {
        return NULL;
    }

    memcpy(result->data, (char*)arr->data + (start * elem_size), slice_len * elem_size);
    return result;
}

/**
 * Free array data (internal helper).
 */
void trq_array_free_data(TrqArray* arr) {
    if (arr && arr->data) {
        free(arr->data);
        arr->data = NULL;
        arr->len = 0;
        arr->cap = 0;
    }
}
