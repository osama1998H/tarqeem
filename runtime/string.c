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

    int64_t count = 0;
    const unsigned char* p = (const unsigned char*)str->data;
    const unsigned char* end = p + str->len;

    while (p < end) {
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

    if (start < 0) {
        start = 0;
    }
    if (start >= str->len) {
        return trq_string_new("", 0);
    }

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
        return trq_string_from_cstr("صحيح");
    } else {
        return trq_string_from_cstr("خاطئ");
    }
}

int64_t trq_string_to_int(TrqString* str) {
    if (!str || !str->data || str->len == 0) {
        return 0;
    }

    char* end;
    long long result = strtoll(str->data, &end, 10);

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

    while (start <= end && (*start == ' ' || *start == '\t' || *start == '\n' || *start == '\r')) {
        start++;
    }

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

    memcpy(result->data, str->data, idx);
    memcpy(result->data + idx, new_str->data, new_str->len);
    memcpy(result->data + idx + new_str->len, str->data + idx + old_str->len, str->len - idx - old_str->len);
    result->data[new_len] = '\0';
    result->len = new_len;
    result->cap = new_len + 1;

    return result;
}

TrqArray* trq_string_split(TrqString* str, TrqString* delim) {
    TrqArray* result = trq_array_new(0, sizeof(TrqString*));
    if (!result) {
        return NULL;
    }

    if (!str || str->len == 0) {
        return result;
    }

    if (!delim || delim->len == 0) {
        TrqString* copy = trq_string_new(str->data, str->len);
        trq_array_push(result, &copy, sizeof(TrqString*));
        return result;
    }

    const char* start = str->data;
    const char* end = str->data + str->len;
    const char* pos = start;

    while (pos <= end - delim->len) {
        if (memcmp(pos, delim->data, delim->len) == 0) {
            TrqString* part = trq_string_new(start, pos - start);
            trq_array_push(result, &part, sizeof(TrqString*));
            pos += delim->len;
            start = pos;
        } else {
            pos++;
        }
    }

    if (start <= end) {
        TrqString* part = trq_string_new(start, end - start);
        trq_array_push(result, &part, sizeof(TrqString*));
    }

    return result;
}

/*============================================================================
 * Additional String Search Functions
 *============================================================================*/

int64_t trq_string_last_index_of(TrqString* str, TrqString* substr) {
    if (!str || !substr) {
        return -1;
    }
    if (substr->len == 0) {
        return str->len;
    }
    if (substr->len > str->len) {
        return -1;
    }

    for (int64_t i = str->len - substr->len; i >= 0; i--) {
        if (memcmp(str->data + i, substr->data, substr->len) == 0) {
            return i;
        }
    }
    return -1;
}

int64_t trq_string_count(TrqString* str, TrqString* substr) {
    if (!str || !substr || substr->len == 0) {
        return 0;
    }
    if (substr->len > str->len) {
        return 0;
    }

    int64_t count = 0;
    const char* pos = str->data;
    const char* end = str->data + str->len - substr->len + 1;

    while (pos < end) {
        if (memcmp(pos, substr->data, substr->len) == 0) {
            count++;
            pos += substr->len;
        } else {
            pos++;
        }
    }
    return count;
}

TrqString* trq_string_reverse(TrqString* str) {
    if (!str || str->len == 0) {
        return trq_string_new("", 0);
    }

    TrqString* result = (TrqString*)trq_alloc(sizeof(TrqString));
    if (!result) {
        return NULL;
    }

    result->data = (char*)malloc(str->len + 1);
    if (!result->data) {
        trq_free(result);
        return NULL;
    }

    const unsigned char* src = (const unsigned char*)str->data;
    int64_t n_chars = 0;
    int64_t* offsets = (int64_t*)malloc((str->len + 1) * sizeof(int64_t));
    if (!offsets) {
        free(result->data);
        trq_free(result);
        return NULL;
    }

    offsets[0] = 0;
    for (int64_t i = 0; i < str->len; ) {
        unsigned char c = src[i];
        int char_len = 1;
        if ((c & 0x80) == 0) {
            char_len = 1;
        } else if ((c & 0xE0) == 0xC0) {
            char_len = 2;
        } else if ((c & 0xF0) == 0xE0) {
            char_len = 3;
        } else if ((c & 0xF8) == 0xF0) {
            char_len = 4;
        }
        i += char_len;
        n_chars++;
        offsets[n_chars] = i;
    }

    char* dest = result->data;
    for (int64_t i = n_chars - 1; i >= 0; i--) {
        int64_t start = offsets[i];
        int64_t len = offsets[i + 1] - start;
        memcpy(dest, str->data + start, len);
        dest += len;
    }
    *dest = '\0';

    free(offsets);
    result->len = str->len;
    result->cap = str->len + 1;

    return result;
}

TrqString* trq_string_to_title(TrqString* str) {
    if (!str) {
        return trq_string_new("", 0);
    }

    TrqString* result = trq_string_new(str->data, str->len);
    if (!result) {
        return NULL;
    }

    bool capitalize_next = true;
    for (int64_t i = 0; i < result->len; i++) {
        char c = result->data[i];
        if (c == ' ' || c == '\t' || c == '\n' || c == '\r') {
            capitalize_next = true;
        } else if (capitalize_next && c >= 'a' && c <= 'z') {
            result->data[i] = c - 32;
            capitalize_next = false;
        } else if (c >= 'A' && c <= 'Z') {
            if (!capitalize_next) {
                result->data[i] = c + 32;
            }
            capitalize_next = false;
        } else {
            capitalize_next = false;
        }
    }
    return result;
}

TrqString* trq_string_trim_left(TrqString* str) {
    if (!str || str->len == 0) {
        return trq_string_new("", 0);
    }

    const char* start = str->data;
    const char* end = str->data + str->len;

    while (start < end && (*start == ' ' || *start == '\t' || *start == '\n' || *start == '\r')) {
        start++;
    }

    return trq_string_new(start, end - start);
}

TrqString* trq_string_trim_right(TrqString* str) {
    if (!str || str->len == 0) {
        return trq_string_new("", 0);
    }

    const char* start = str->data;
    const char* end = str->data + str->len - 1;

    while (end >= start && (*end == ' ' || *end == '\t' || *end == '\n' || *end == '\r')) {
        end--;
    }

    return trq_string_new(start, end - start + 1);
}

TrqString* trq_string_join(TrqArray* arr, TrqString* delim) {
    if (!arr || arr->len == 0) {
        return trq_string_new("", 0);
    }

    int64_t total_len = 0;
    TrqString** strings = (TrqString**)arr->data;
    for (int64_t i = 0; i < arr->len; i++) {
        if (strings[i]) {
            total_len += strings[i]->len;
        }
        if (delim && i < arr->len - 1) {
            total_len += delim->len;
        }
    }

    TrqString* result = (TrqString*)trq_alloc(sizeof(TrqString));
    if (!result) {
        return NULL;
    }

    result->data = (char*)malloc(total_len + 1);
    if (!result->data) {
        trq_free(result);
        return NULL;
    }

    char* dest = result->data;
    for (int64_t i = 0; i < arr->len; i++) {
        if (strings[i]) {
            memcpy(dest, strings[i]->data, strings[i]->len);
            dest += strings[i]->len;
        }
        if (delim && i < arr->len - 1) {
            memcpy(dest, delim->data, delim->len);
            dest += delim->len;
        }
    }
    *dest = '\0';

    result->len = total_len;
    result->cap = total_len + 1;

    return result;
}

TrqString* trq_string_replace_all(TrqString* str, TrqString* old_str, TrqString* new_str) {
    if (!str) {
        return trq_string_new("", 0);
    }
    if (!old_str || old_str->len == 0) {
        return trq_string_new(str->data, str->len);
    }

    TrqString* empty = trq_string_new("", 0);
    if (!new_str) {
        new_str = empty;
    }

    int64_t count = trq_string_count(str, old_str);
    if (count == 0) {
        trq_release(empty);
        return trq_string_new(str->data, str->len);
    }

    int64_t new_len = str->len + count * (new_str->len - old_str->len);
    TrqString* result = (TrqString*)trq_alloc(sizeof(TrqString));
    if (!result) {
        trq_release(empty);
        return NULL;
    }

    result->data = (char*)malloc(new_len + 1);
    if (!result->data) {
        trq_free(result);
        trq_release(empty);
        return NULL;
    }

    char* dest = result->data;
    const char* src = str->data;
    const char* end = str->data + str->len;

    while (src < end) {
        if (src <= end - old_str->len && memcmp(src, old_str->data, old_str->len) == 0) {
            memcpy(dest, new_str->data, new_str->len);
            dest += new_str->len;
            src += old_str->len;
        } else {
            *dest++ = *src++;
        }
    }
    *dest = '\0';

    result->len = new_len;
    result->cap = new_len + 1;

    trq_release(empty);
    return result;
}

TrqString* trq_string_pad_left(TrqString* str, int64_t target_len, TrqString* pad) {
    if (!str) {
        return trq_string_new("", 0);
    }

    int64_t current_len = trq_string_len_chars(str);
    if (current_len >= target_len) {
        return trq_string_new(str->data, str->len);
    }

    char pad_char = ' ';
    if (pad && pad->len > 0) {
        pad_char = pad->data[0];
    }

    int64_t pad_count = target_len - current_len;
    int64_t new_len = str->len + pad_count;

    TrqString* result = (TrqString*)trq_alloc(sizeof(TrqString));
    if (!result) {
        return NULL;
    }

    result->data = (char*)malloc(new_len + 1);
    if (!result->data) {
        trq_free(result);
        return NULL;
    }

    memset(result->data, pad_char, pad_count);
    memcpy(result->data + pad_count, str->data, str->len);
    result->data[new_len] = '\0';

    result->len = new_len;
    result->cap = new_len + 1;

    return result;
}

TrqString* trq_string_pad_right(TrqString* str, int64_t target_len, TrqString* pad) {
    if (!str) {
        return trq_string_new("", 0);
    }

    int64_t current_len = trq_string_len_chars(str);
    if (current_len >= target_len) {
        return trq_string_new(str->data, str->len);
    }

    char pad_char = ' ';
    if (pad && pad->len > 0) {
        pad_char = pad->data[0];
    }

    int64_t pad_count = target_len - current_len;
    int64_t new_len = str->len + pad_count;

    TrqString* result = (TrqString*)trq_alloc(sizeof(TrqString));
    if (!result) {
        return NULL;
    }

    result->data = (char*)malloc(new_len + 1);
    if (!result->data) {
        trq_free(result);
        return NULL;
    }

    memcpy(result->data, str->data, str->len);
    memset(result->data + str->len, pad_char, pad_count);
    result->data[new_len] = '\0';

    result->len = new_len;
    result->cap = new_len + 1;

    return result;
}

bool trq_string_is_numeric(TrqString* str) {
    if (!str || str->len == 0) {
        return false;
    }

    for (int64_t i = 0; i < str->len; i++) {
        char c = str->data[i];
        if (c < '0' || c > '9') {
            return false;
        }
    }
    return true;
}

bool trq_string_is_alpha(TrqString* str) {
    if (!str || str->len == 0) {
        return false;
    }

    const unsigned char* p = (const unsigned char*)str->data;
    const unsigned char* end = p + str->len;

    while (p < end) {
        unsigned char c = *p;

        if ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z')) {
            p++;
            continue;
        }

        if ((c & 0x80) != 0) {
            if ((c & 0xE0) == 0xC0) {
                p += 2;
            } else if ((c & 0xF0) == 0xE0) {
                p += 3;
            } else if ((c & 0xF8) == 0xF0) {
                p += 4;
            } else {
                return false;
            }
            continue;
        }

        return false;
    }
    return true;
}

bool trq_string_is_arabic(TrqString* str) {
    if (!str || str->len == 0) {
        return false;
    }

    const unsigned char* p = (const unsigned char*)str->data;
    const unsigned char* end = p + str->len;

    while (p < end) {
        unsigned char c = *p;

        if (c >= 0xD8 && c <= 0xDB) {
            if (p + 1 < end) {
                p += 2;
                continue;
            }
        }

        if (c == ' ' || c == '\t' || c == '\n' || c == '\r') {
            p++;
            continue;
        }

        return false;
    }
    return true;
}

TrqString* trq_string_char_at(TrqString* str, int64_t index) {
    if (!str || index < 0) {
        return trq_string_new("", 0);
    }

    const unsigned char* p = (const unsigned char*)str->data;
    const unsigned char* end = p + str->len;
    int64_t current = 0;

    while (p < end) {
        if (current == index) {
            int char_len = 1;
            unsigned char c = *p;
            if ((c & 0x80) == 0) {
                char_len = 1;
            } else if ((c & 0xE0) == 0xC0) {
                char_len = 2;
            } else if ((c & 0xF0) == 0xE0) {
                char_len = 3;
            } else if ((c & 0xF8) == 0xF0) {
                char_len = 4;
            }
            return trq_string_new((const char*)p, char_len);
        }

        unsigned char c = *p;
        if ((c & 0x80) == 0) {
            p++;
        } else if ((c & 0xE0) == 0xC0) {
            p += 2;
        } else if ((c & 0xF0) == 0xE0) {
            p += 3;
        } else if ((c & 0xF8) == 0xF0) {
            p += 4;
        } else {
            p++;
        }
        current++;
    }

    return trq_string_new("", 0);
}

TrqString* trq_string_substr_chars(TrqString* str, int64_t start, int64_t len) {
    if (!str || start < 0 || len < 0) {
        return trq_string_new("", 0);
    }

    const unsigned char* p = (const unsigned char*)str->data;
    const unsigned char* end = p + str->len;
    int64_t current = 0;

    while (p < end && current < start) {
        unsigned char c = *p;
        if ((c & 0x80) == 0) {
            p++;
        } else if ((c & 0xE0) == 0xC0) {
            p += 2;
        } else if ((c & 0xF0) == 0xE0) {
            p += 3;
        } else if ((c & 0xF8) == 0xF0) {
            p += 4;
        } else {
            p++;
        }
        current++;
    }

    if (p >= end) {
        return trq_string_new("", 0);
    }

    const unsigned char* substr_start = p;

    int64_t chars_collected = 0;
    while (p < end && chars_collected < len) {
        unsigned char c = *p;
        if ((c & 0x80) == 0) {
            p++;
        } else if ((c & 0xE0) == 0xC0) {
            p += 2;
        } else if ((c & 0xF0) == 0xE0) {
            p += 3;
        } else if ((c & 0xF8) == 0xF0) {
            p += 4;
        } else {
            p++;
        }
        chars_collected++;
    }

    return trq_string_new((const char*)substr_start, p - substr_start);
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
