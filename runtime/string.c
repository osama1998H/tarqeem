/**
 * Tarqeem Runtime - String Operations
 *
 * Implements UTF-8 string handling with reference counting.
 */

#include "tarqeem_rt.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

/*============================================================================
 * String Creation
 *============================================================================*/

TrqString* trq_string_new(const char* data, int64_t len) {
    TrqString* str = (TrqString*)trq_alloc(sizeof(TrqString));
    if (!str) {
        return NULL;
    }

    if (len < 0) {
        len = 0;
    }

    // Allocate data with extra byte for null terminator
    int64_t cap = len + 1;
    str->data = (char*)malloc(cap);
    if (!str->data) {
        trq_free(str);
        return NULL;
    }

    if (data && len > 0) {
        memcpy(str->data, data, len);
    }
    str->data[len] = '\0';
    str->len = len;
    str->cap = cap;

    return str;
}

TrqString* trq_string_from_cstr(const char* cstr) {
    if (!cstr) {
        return trq_string_new("", 0);
    }
    return trq_string_new(cstr, strlen(cstr));
}

/*============================================================================
 * String Operations
 *============================================================================*/

TrqString* trq_string_concat(TrqString* left, TrqString* right) {
    if (!left && !right) {
        return trq_string_new("", 0);
    }
    if (!left) {
        return trq_string_new(right->data, right->len);
    }
    if (!right) {
        return trq_string_new(left->data, left->len);
    }

    int64_t new_len = left->len + right->len;
    TrqString* result = (TrqString*)trq_alloc(sizeof(TrqString));
    if (!result) {
        return NULL;
    }

    result->data = (char*)malloc(new_len + 1);
    if (!result->data) {
        trq_free(result);
        return NULL;
    }

    memcpy(result->data, left->data, left->len);
    memcpy(result->data + left->len, right->data, right->len);
    result->data[new_len] = '\0';
    result->len = new_len;
    result->cap = new_len + 1;

    return result;
}

int64_t trq_string_len(TrqString* str) {
    if (!str) {
        return 0;
    }
    return str->len;
}

int64_t trq_string_len_chars(TrqString* str) {
    if (!str || !str->data) {
        return 0;
    }

    // Count UTF-8 code points
    int64_t count = 0;
    const unsigned char* p = (const unsigned char*)str->data;
    const unsigned char* end = p + str->len;

    while (p < end) {
        // UTF-8 continuation bytes start with 10xxxxxx
        // Skip them, count only leading bytes
        if ((*p & 0xC0) != 0x80) {
            count++;
        }
        p++;
    }

    return count;
}

int64_t trq_string_compare(TrqString* a, TrqString* b) {
    if (!a && !b) {
        return 0;
    }
    if (!a) {
        return -1;
    }
    if (!b) {
        return 1;
    }

    int64_t min_len = a->len < b->len ? a->len : b->len;
    int result = memcmp(a->data, b->data, min_len);

    if (result != 0) {
        return result;
    }

    // Strings are equal up to min_len, compare lengths
    if (a->len < b->len) {
        return -1;
    }
    if (a->len > b->len) {
        return 1;
    }
    return 0;
}

bool trq_string_equals(TrqString* a, TrqString* b) {
    if (!a && !b) {
        return true;
    }
    if (!a || !b) {
        return false;
    }
    if (a->len != b->len) {
        return false;
    }
    return memcmp(a->data, b->data, a->len) == 0;
}

TrqString* trq_string_substr(TrqString* str, int64_t start, int64_t len) {
    if (!str || !str->data) {
        return trq_string_new("", 0);
    }

    // Clamp start
    if (start < 0) {
        start = 0;
    }
    if (start >= str->len) {
        return trq_string_new("", 0);
    }

    // Clamp length
    if (len < 0) {
        len = 0;
    }
    if (start + len > str->len) {
        len = str->len - start;
    }

    return trq_string_new(str->data + start, len);
}

/*============================================================================
 * Type Conversions
 *============================================================================*/

TrqString* trq_int_to_string(int64_t value) {
    char buffer[32];
    int len = snprintf(buffer, sizeof(buffer), "%ld", (long)value);
    return trq_string_new(buffer, len);
}

TrqString* trq_float_to_string(double value) {
    char buffer[64];
    int len = snprintf(buffer, sizeof(buffer), "%g", value);
    return trq_string_new(buffer, len);
}

TrqString* trq_bool_to_string(bool value) {
    if (value) {
        // "صحيح" in UTF-8
        return trq_string_from_cstr("صحيح");
    } else {
        // "خاطئ" in UTF-8
        return trq_string_from_cstr("خاطئ");
    }
}

int64_t trq_string_to_int(TrqString* str) {
    if (!str || !str->data || str->len == 0) {
        return 0;
    }

    char* end;
    long long result = strtoll(str->data, &end, 10);

    // Check if conversion was successful
    if (end == str->data) {
        return 0;
    }

    return (int64_t)result;
}

double trq_string_to_float(TrqString* str) {
    if (!str || !str->data || str->len == 0) {
        return 0.0;
    }

    char* end;
    double result = strtod(str->data, &end);

    // Check if conversion was successful
    if (end == str->data) {
        return 0.0;
    }

    return result;
}

/*============================================================================
 * String Search Operations
 *============================================================================*/

bool trq_string_contains(TrqString* str, TrqString* substr) {
    if (!str || !substr) {
        return false;
    }
    if (substr->len == 0) {
        return true;
    }
    if (substr->len > str->len) {
        return false;
    }

    // Simple substring search
    for (int64_t i = 0; i <= str->len - substr->len; i++) {
        if (memcmp(str->data + i, substr->data, substr->len) == 0) {
            return true;
        }
    }
    return false;
}

bool trq_string_starts_with(TrqString* str, TrqString* prefix) {
    if (!str || !prefix) {
        return false;
    }
    if (prefix->len == 0) {
        return true;
    }
    if (prefix->len > str->len) {
        return false;
    }
    return memcmp(str->data, prefix->data, prefix->len) == 0;
}

bool trq_string_ends_with(TrqString* str, TrqString* suffix) {
    if (!str || !suffix) {
        return false;
    }
    if (suffix->len == 0) {
        return true;
    }
    if (suffix->len > str->len) {
        return false;
    }
    return memcmp(str->data + str->len - suffix->len, suffix->data, suffix->len) == 0;
}

int64_t trq_string_index_of(TrqString* str, TrqString* substr) {
    if (!str || !substr) {
        return -1;
    }
    if (substr->len == 0) {
        return 0;
    }
    if (substr->len > str->len) {
        return -1;
    }

    for (int64_t i = 0; i <= str->len - substr->len; i++) {
        if (memcmp(str->data + i, substr->data, substr->len) == 0) {
            return i;
        }
    }
    return -1;
}

/*============================================================================
 * String Transformation
 *============================================================================*/

TrqString* trq_string_to_upper(TrqString* str) {
    if (!str) {
        return trq_string_new("", 0);
    }

    TrqString* result = trq_string_new(str->data, str->len);
    if (!result) {
        return NULL;
    }

    // Convert ASCII characters only
    for (int64_t i = 0; i < result->len; i++) {
        char c = result->data[i];
        if (c >= 'a' && c <= 'z') {
            result->data[i] = c - 32;
        }
    }
    return result;
}

TrqString* trq_string_to_lower(TrqString* str) {
    if (!str) {
        return trq_string_new("", 0);
    }

    TrqString* result = trq_string_new(str->data, str->len);
    if (!result) {
        return NULL;
    }

    // Convert ASCII characters only
    for (int64_t i = 0; i < result->len; i++) {
        char c = result->data[i];
        if (c >= 'A' && c <= 'Z') {
            result->data[i] = c + 32;
        }
    }
    return result;
}

TrqString* trq_string_trim(TrqString* str) {
    if (!str || str->len == 0) {
        return trq_string_new("", 0);
    }

    const char* start = str->data;
    const char* end = str->data + str->len - 1;

    // Trim leading whitespace
    while (start <= end && (*start == ' ' || *start == '\t' || *start == '\n' || *start == '\r')) {
        start++;
    }

    // Trim trailing whitespace
    while (end >= start && (*end == ' ' || *end == '\t' || *end == '\n' || *end == '\r')) {
        end--;
    }

    int64_t new_len = end - start + 1;
    if (new_len <= 0) {
        return trq_string_new("", 0);
    }

    return trq_string_new(start, new_len);
}

TrqString* trq_string_repeat(TrqString* str, int64_t n) {
    if (!str || n <= 0) {
        return trq_string_new("", 0);
    }

    int64_t new_len = str->len * n;
    TrqString* result = (TrqString*)trq_alloc(sizeof(TrqString));
    if (!result) {
        return NULL;
    }

    result->data = (char*)malloc(new_len + 1);
    if (!result->data) {
        trq_free(result);
        return NULL;
    }

    for (int64_t i = 0; i < n; i++) {
        memcpy(result->data + (i * str->len), str->data, str->len);
    }
    result->data[new_len] = '\0';
    result->len = new_len;
    result->cap = new_len + 1;

    return result;
}

TrqString* trq_string_replace(TrqString* str, TrqString* old_str, TrqString* new_str) {
    if (!str) {
        return trq_string_new("", 0);
    }
    if (!old_str || old_str->len == 0) {
        return trq_string_new(str->data, str->len);
    }
    if (!new_str) {
        new_str = trq_string_new("", 0);
    }

    int64_t idx = trq_string_index_of(str, old_str);
    if (idx < 0) {
        return trq_string_new(str->data, str->len);
    }

    // Calculate new length
    int64_t new_len = str->len - old_str->len + new_str->len;
    TrqString* result = (TrqString*)trq_alloc(sizeof(TrqString));
    if (!result) {
        return NULL;
    }

    result->data = (char*)malloc(new_len + 1);
    if (!result->data) {
        trq_free(result);
        return NULL;
    }

    // Copy before match
    memcpy(result->data, str->data, idx);
    // Copy replacement
    memcpy(result->data + idx, new_str->data, new_str->len);
    // Copy after match
    memcpy(result->data + idx + new_str->len, str->data + idx + old_str->len, str->len - idx - old_str->len);
    result->data[new_len] = '\0';
    result->len = new_len;
    result->cap = new_len + 1;

    return result;
}

TrqArray* trq_string_split(TrqString* str, TrqString* delim) {
    // Create array of string pointers
    TrqArray* result = trq_array_new(0, sizeof(TrqString*));
    if (!result) {
        return NULL;
    }

    if (!str || str->len == 0) {
        return result;
    }

    if (!delim || delim->len == 0) {
        // No delimiter, return the whole string
        TrqString* copy = trq_string_new(str->data, str->len);
        trq_array_push(result, &copy, sizeof(TrqString*));
        return result;
    }

    const char* start = str->data;
    const char* end = str->data + str->len;
    const char* pos = start;

    while (pos <= end - delim->len) {
        if (memcmp(pos, delim->data, delim->len) == 0) {
            // Found delimiter, add substring
            TrqString* part = trq_string_new(start, pos - start);
            trq_array_push(result, &part, sizeof(TrqString*));
            pos += delim->len;
            start = pos;
        } else {
            pos++;
        }
    }

    // Add remaining part
    if (start <= end) {
        TrqString* part = trq_string_new(start, end - start);
        trq_array_push(result, &part, sizeof(TrqString*));
    }

    return result;
}

/*============================================================================
 * String Helper - Free String Data
 *============================================================================*/

/**
 * Internal function to free string data.
 * Called when the string's reference count reaches zero.
 */
void trq_string_free_data(TrqString* str) {
    if (str && str->data) {
        free(str->data);
        str->data = NULL;
        str->len = 0;
        str->cap = 0;
    }
}
