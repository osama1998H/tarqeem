/**
 * Tarqeem Runtime - I/O Operations
 *
 * Implements input/output functions for the Tarqeem language.
 */

#include "tarqeem_rt.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

/*============================================================================
 * Output Functions
 *============================================================================*/

void trq_print(TrqString* str) {
    if (!str || !str->data) {
        return;
    }
    fwrite(str->data, 1, str->len, stdout);
}

void trq_print_int(int64_t value) {
    printf("%ld", (long)value);
}

void trq_print_float(double value) {
    printf("%g", value);
}

void trq_print_bool(bool value) {
    if (value) {
        // "صحيح" in UTF-8
        printf("صحيح");
    } else {
        // "خاطئ" in UTF-8
        printf("خاطئ");
    }
}

void trq_print_newline(void) {
    printf("\n");
    fflush(stdout);
}

void trq_print_array(TrqArray* arr) {
    if (!arr) {
        printf("[null]");
        return;
    }
    printf("[");
    for (int64_t i = 0; i < arr->len; i++) {
        if (i > 0) {
            printf("، ");
        }
        // Print element as integer by default
        // A more complete implementation would handle different element types
        if (arr->data) {
            int64_t* int_data = (int64_t*)arr->data;
            printf("%ld", (long)int_data[i]);
        }
    }
    printf("]");
}

/*============================================================================
 * Input Functions
 *============================================================================*/

#define INPUT_BUFFER_SIZE 4096

TrqString* trq_input(void) {
    char buffer[INPUT_BUFFER_SIZE];

    if (!fgets(buffer, INPUT_BUFFER_SIZE, stdin)) {
        // EOF or error
        return trq_string_new("", 0);
    }

    // Remove trailing newline if present
    size_t len = strlen(buffer);
    if (len > 0 && buffer[len - 1] == '\n') {
        buffer[len - 1] = '\0';
        len--;
    }
    // Also remove carriage return for Windows
    if (len > 0 && buffer[len - 1] == '\r') {
        buffer[len - 1] = '\0';
        len--;
    }

    return trq_string_new(buffer, (int64_t)len);
}

TrqString* trq_input_prompt(TrqString* prompt) {
    if (prompt && prompt->data) {
        fwrite(prompt->data, 1, prompt->len, stdout);
        fflush(stdout);
    }
    return trq_input();
}

/*============================================================================
 * File I/O (Future Extension)
 *============================================================================*/

/**
 * File handle structure for future file I/O support.
 */
typedef struct TrqFile {
    FILE* handle;
    TrqString* path;
    int64_t mode;
} TrqFile;

/**
 * Open a file for reading.
 */
TrqFile* trq_file_open_read(TrqString* path) {
    if (!path || !path->data) {
        fprintf(stderr, "Error: Cannot open file with null path / خطأ: لا يمكن فتح ملف بمسار فارغ\n");
        return NULL;
    }

    TrqFile* file = (TrqFile*)trq_alloc(sizeof(TrqFile));
    if (!file) {
        return NULL;
    }

    file->handle = fopen(path->data, "rb");
    if (!file->handle) {
        fprintf(stderr, "Error: Cannot open file '%s' / خطأ: لا يمكن فتح الملف '%s'\n",
                path->data, path->data);
        trq_free(file);
        return NULL;
    }

    file->path = path;
    trq_retain(path);
    file->mode = 0; // Read mode

    return file;
}

/**
 * Open a file for writing.
 */
TrqFile* trq_file_open_write(TrqString* path) {
    if (!path || !path->data) {
        fprintf(stderr, "Error: Cannot open file with null path / خطأ: لا يمكن فتح ملف بمسار فارغ\n");
        return NULL;
    }

    TrqFile* file = (TrqFile*)trq_alloc(sizeof(TrqFile));
    if (!file) {
        return NULL;
    }

    file->handle = fopen(path->data, "wb");
    if (!file->handle) {
        fprintf(stderr, "Error: Cannot open file '%s' for writing / خطأ: لا يمكن فتح الملف '%s' للكتابة\n",
                path->data, path->data);
        trq_free(file);
        return NULL;
    }

    file->path = path;
    trq_retain(path);
    file->mode = 1; // Write mode

    return file;
}

/**
 * Close a file.
 */
void trq_file_close(TrqFile* file) {
    if (!file) {
        return;
    }

    if (file->handle) {
        fclose(file->handle);
        file->handle = NULL;
    }

    if (file->path) {
        trq_release(file->path);
        file->path = NULL;
    }

    trq_free(file);
}

/**
 * Read entire file contents as string.
 */
TrqString* trq_file_read_all(TrqFile* file) {
    if (!file || !file->handle) {
        return trq_string_new("", 0);
    }

    // Get file size
    fseek(file->handle, 0, SEEK_END);
    long size = ftell(file->handle);
    fseek(file->handle, 0, SEEK_SET);

    if (size <= 0) {
        return trq_string_new("", 0);
    }

    // Allocate buffer
    char* buffer = (char*)malloc(size + 1);
    if (!buffer) {
        return trq_string_new("", 0);
    }

    // Read file
    size_t read_size = fread(buffer, 1, size, file->handle);
    buffer[read_size] = '\0';

    TrqString* result = trq_string_new(buffer, (int64_t)read_size);
    free(buffer);

    return result;
}

/**
 * Read a line from file.
 */
TrqString* trq_file_read_line(TrqFile* file) {
    if (!file || !file->handle) {
        return trq_string_new("", 0);
    }

    char buffer[INPUT_BUFFER_SIZE];

    if (!fgets(buffer, INPUT_BUFFER_SIZE, file->handle)) {
        return trq_string_new("", 0);
    }

    size_t len = strlen(buffer);
    if (len > 0 && buffer[len - 1] == '\n') {
        buffer[len - 1] = '\0';
        len--;
    }
    if (len > 0 && buffer[len - 1] == '\r') {
        buffer[len - 1] = '\0';
        len--;
    }

    return trq_string_new(buffer, (int64_t)len);
}

/**
 * Write string to file.
 */
void trq_file_write(TrqFile* file, TrqString* content) {
    if (!file || !file->handle || !content || !content->data) {
        return;
    }

    fwrite(content->data, 1, content->len, file->handle);
}

/**
 * Write line to file.
 */
void trq_file_write_line(TrqFile* file, TrqString* content) {
    trq_file_write(file, content);
    if (file && file->handle) {
        fputc('\n', file->handle);
    }
}

/**
 * Check if at end of file.
 */
bool trq_file_eof(TrqFile* file) {
    if (!file || !file->handle) {
        return true;
    }
    return feof(file->handle) != 0;
}
