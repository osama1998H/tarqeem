/**
 * Tarqeem Runtime - Memory Management
 *
 * Implements reference-counted memory allocation.
 */

#include "tarqeem_rt.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

/*============================================================================
 * Reference Counting Header
 *============================================================================*/

typedef struct RefCountHeader {
    int64_t refcount;
    int64_t size;
} RefCountHeader;

#define HEADER_SIZE sizeof(RefCountHeader)
#define GET_HEADER(ptr) ((RefCountHeader*)((char*)(ptr) - HEADER_SIZE))
#define GET_DATA(header) ((void*)((char*)(header) + HEADER_SIZE))

/*============================================================================
 * Memory Allocation
 *============================================================================*/

void* trq_alloc(int64_t size) {
    if (size <= 0) {
        return NULL;
    }

    RefCountHeader* header = (RefCountHeader*)malloc(HEADER_SIZE + size);
    if (!header) {
        fprintf(stderr, "Error: Memory allocation failed / خطأ: فشل في تخصيص الذاكرة\n");
        return NULL;
    }

    header->refcount = 1;
    header->size = size;

    void* data = GET_DATA(header);
    memset(data, 0, size);

    return data;
}

void* trq_realloc(void* ptr, int64_t new_size) {
    if (!ptr) {
        return trq_alloc(new_size);
    }

    if (new_size <= 0) {
        trq_free(ptr);
        return NULL;
    }

    RefCountHeader* old_header = GET_HEADER(ptr);
    RefCountHeader* new_header = (RefCountHeader*)realloc(old_header, HEADER_SIZE + new_size);

    if (!new_header) {
        fprintf(stderr, "Error: Memory reallocation failed / خطأ: فشل في إعادة تخصيص الذاكرة\n");
        return NULL;
    }

    if (new_size > new_header->size) {
        char* data = (char*)GET_DATA(new_header);
        memset(data + new_header->size, 0, new_size - new_header->size);
    }

    new_header->size = new_size;
    return GET_DATA(new_header);
}

void trq_free(void* ptr) {
    if (!ptr) {
        return;
    }

    RefCountHeader* header = GET_HEADER(ptr);
    free(header);
}

void trq_retain(void* ptr) {
    if (!ptr) {
        return;
    }

    RefCountHeader* header = GET_HEADER(ptr);
    header->refcount++;
}

void trq_release(void* ptr) {
    if (!ptr) {
        return;
    }

    RefCountHeader* header = GET_HEADER(ptr);
    header->refcount--;

    if (header->refcount <= 0) {
        free(header);
    }
}

int64_t trq_refcount(void* ptr) {
    if (!ptr) {
        return 0;
    }

    RefCountHeader* header = GET_HEADER(ptr);
    return header->refcount;
}
