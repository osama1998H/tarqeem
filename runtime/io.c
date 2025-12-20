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
 * Write string to file handle.
 */
void trq_file_handle_write(TrqFile* file, TrqString* content) {
    if (!file || !file->handle || !content || !content->data) {
        return;
    }

    fwrite(content->data, 1, content->len, file->handle);
}

/**
 * Write line to file handle.
 */
void trq_file_handle_write_line(TrqFile* file, TrqString* content) {
    trq_file_handle_write(file, content);
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

/*============================================================================
 * File System Operations (Simplified API)
 *============================================================================*/

#include <sys/stat.h>
#include <unistd.h>
#include <dirent.h>
#include <errno.h>

bool trq_file_exists(TrqString* path) {
    if (!path || !path->data) {
        return false;
    }
    struct stat st;
    return stat(path->data, &st) == 0;
}

bool trq_file_is_file(TrqString* path) {
    if (!path || !path->data) {
        return false;
    }
    struct stat st;
    if (stat(path->data, &st) != 0) {
        return false;
    }
    return S_ISREG(st.st_mode);
}

bool trq_file_is_dir(TrqString* path) {
    if (!path || !path->data) {
        return false;
    }
    struct stat st;
    if (stat(path->data, &st) != 0) {
        return false;
    }
    return S_ISDIR(st.st_mode);
}

TrqString* trq_file_read(TrqString* path) {
    if (!path || !path->data) {
        return trq_string_new("", 0);
    }

    FILE* f = fopen(path->data, "rb");
    if (!f) {
        return trq_string_new("", 0);
    }

    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);

    if (size <= 0) {
        fclose(f);
        return trq_string_new("", 0);
    }

    char* buffer = (char*)malloc(size + 1);
    if (!buffer) {
        fclose(f);
        return trq_string_new("", 0);
    }

    size_t read_size = fread(buffer, 1, size, f);
    buffer[read_size] = '\0';
    fclose(f);

    TrqString* result = trq_string_new(buffer, (int64_t)read_size);
    free(buffer);
    return result;
}

bool trq_file_write(TrqString* path, TrqString* content) {
    if (!path || !path->data) {
        return false;
    }

    FILE* f = fopen(path->data, "wb");
    if (!f) {
        return false;
    }

    if (content && content->data && content->len > 0) {
        size_t written = fwrite(content->data, 1, content->len, f);
        fclose(f);
        return written == (size_t)content->len;
    }

    fclose(f);
    return true;
}

bool trq_file_append(TrqString* path, TrqString* content) {
    if (!path || !path->data) {
        return false;
    }

    FILE* f = fopen(path->data, "ab");
    if (!f) {
        return false;
    }

    if (content && content->data && content->len > 0) {
        size_t written = fwrite(content->data, 1, content->len, f);
        fclose(f);
        return written == (size_t)content->len;
    }

    fclose(f);
    return true;
}

bool trq_file_delete(TrqString* path) {
    if (!path || !path->data) {
        return false;
    }
    return remove(path->data) == 0;
}

bool trq_file_copy(TrqString* src, TrqString* dst) {
    if (!src || !src->data || !dst || !dst->data) {
        return false;
    }

    TrqString* content = trq_file_read(src);
    if (!content) {
        return false;
    }

    bool result = trq_file_write(dst, content);
    trq_release(content);
    return result;
}

bool trq_file_move(TrqString* src, TrqString* dst) {
    if (!src || !src->data || !dst || !dst->data) {
        return false;
    }
    return rename(src->data, dst->data) == 0;
}

int64_t trq_file_size(TrqString* path) {
    if (!path || !path->data) {
        return -1;
    }
    struct stat st;
    if (stat(path->data, &st) != 0) {
        return -1;
    }
    return (int64_t)st.st_size;
}

bool trq_dir_create(TrqString* path) {
    if (!path || !path->data) {
        return false;
    }
    return mkdir(path->data, 0755) == 0;
}

bool trq_dir_create_all(TrqString* path) {
    if (!path || !path->data) {
        return false;
    }

    char* temp = strdup(path->data);
    if (!temp) {
        return false;
    }

    char* p = temp;
    while (*p) {
        if (*p == '/') {
            *p = '\0';
            if (strlen(temp) > 0) {
                mkdir(temp, 0755);
            }
            *p = '/';
        }
        p++;
    }
    bool result = mkdir(temp, 0755) == 0 || errno == EEXIST;
    free(temp);
    return result;
}

bool trq_dir_delete(TrqString* path) {
    if (!path || !path->data) {
        return false;
    }
    return rmdir(path->data) == 0;
}

TrqArray* trq_dir_list(TrqString* path) {
    TrqArray* result = trq_array_new(0, sizeof(TrqString*));
    if (!result) {
        return NULL;
    }

    if (!path || !path->data) {
        return result;
    }

    DIR* dir = opendir(path->data);
    if (!dir) {
        return result;
    }

    struct dirent* entry;
    while ((entry = readdir(dir)) != NULL) {
        // Skip . and ..
        if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) {
            continue;
        }
        TrqString* name = trq_string_from_cstr(entry->d_name);
        trq_array_push(result, &name, sizeof(TrqString*));
    }

    closedir(dir);
    return result;
}

TrqString* trq_dir_current(void) {
    char buffer[4096];
    if (getcwd(buffer, sizeof(buffer)) != NULL) {
        return trq_string_from_cstr(buffer);
    }
    return trq_string_new("", 0);
}

TrqString* trq_dir_home(void) {
    const char* home = getenv("HOME");
    if (home) {
        return trq_string_from_cstr(home);
    }
    return trq_string_new("", 0);
}

TrqString* trq_dir_temp(void) {
    const char* temp = getenv("TMPDIR");
    if (!temp) temp = getenv("TMP");
    if (!temp) temp = getenv("TEMP");
    if (!temp) temp = "/tmp";
    return trq_string_from_cstr(temp);
}

/*============================================================================
 * Path Operations
 *============================================================================*/

TrqString* trq_path_join(TrqString* base, TrqString* component) {
    if (!base || base->len == 0) {
        return component ? trq_string_new(component->data, component->len) : trq_string_new("", 0);
    }
    if (!component || component->len == 0) {
        return trq_string_new(base->data, base->len);
    }

    // Check if base ends with /
    bool has_slash = base->data[base->len - 1] == '/';
    bool comp_starts_slash = component->data[0] == '/';

    TrqString* result;
    if (has_slash && comp_starts_slash) {
        // base/ + /component -> base/component
        TrqString* comp_no_slash = trq_string_substr(component, 1, component->len - 1);
        result = trq_string_concat(base, comp_no_slash);
        trq_release(comp_no_slash);
    } else if (!has_slash && !comp_starts_slash) {
        // base + component -> base/component
        TrqString* slash = trq_string_from_cstr("/");
        TrqString* temp = trq_string_concat(base, slash);
        result = trq_string_concat(temp, component);
        trq_release(slash);
        trq_release(temp);
    } else {
        result = trq_string_concat(base, component);
    }

    return result;
}

TrqString* trq_path_parent(TrqString* path) {
    if (!path || path->len == 0) {
        return trq_string_new("", 0);
    }

    // Find last /
    int64_t last_slash = -1;
    for (int64_t i = path->len - 1; i >= 0; i--) {
        if (path->data[i] == '/') {
            last_slash = i;
            break;
        }
    }

    if (last_slash <= 0) {
        return trq_string_from_cstr(last_slash == 0 ? "/" : ".");
    }

    return trq_string_substr(path, 0, last_slash);
}

TrqString* trq_path_filename(TrqString* path) {
    if (!path || path->len == 0) {
        return trq_string_new("", 0);
    }

    // Find last /
    int64_t last_slash = -1;
    for (int64_t i = path->len - 1; i >= 0; i--) {
        if (path->data[i] == '/') {
            last_slash = i;
            break;
        }
    }

    if (last_slash < 0) {
        return trq_string_new(path->data, path->len);
    }

    return trq_string_substr(path, last_slash + 1, path->len - last_slash - 1);
}

TrqString* trq_path_extension(TrqString* path) {
    TrqString* filename = trq_path_filename(path);
    if (!filename || filename->len == 0) {
        return trq_string_new("", 0);
    }

    // Find last .
    int64_t last_dot = -1;
    for (int64_t i = filename->len - 1; i >= 0; i--) {
        if (filename->data[i] == '.') {
            last_dot = i;
            break;
        }
    }

    if (last_dot <= 0) {
        trq_release(filename);
        return trq_string_new("", 0);
    }

    TrqString* result = trq_string_substr(filename, last_dot + 1, filename->len - last_dot - 1);
    trq_release(filename);
    return result;
}

TrqString* trq_path_stem(TrqString* path) {
    TrqString* filename = trq_path_filename(path);
    if (!filename || filename->len == 0) {
        return trq_string_new("", 0);
    }

    // Find last .
    int64_t last_dot = -1;
    for (int64_t i = filename->len - 1; i >= 0; i--) {
        if (filename->data[i] == '.') {
            last_dot = i;
            break;
        }
    }

    if (last_dot <= 0) {
        return filename;
    }

    TrqString* result = trq_string_substr(filename, 0, last_dot);
    trq_release(filename);
    return result;
}

TrqString* trq_path_absolute(TrqString* path) {
    if (!path || path->len == 0) {
        return trq_dir_current();
    }

    if (path->data[0] == '/') {
        return trq_string_new(path->data, path->len);
    }

    TrqString* cwd = trq_dir_current();
    TrqString* result = trq_path_join(cwd, path);
    trq_release(cwd);
    return result;
}

bool trq_path_is_absolute(TrqString* path) {
    if (!path || path->len == 0) {
        return false;
    }
    return path->data[0] == '/';
}

TrqString* trq_path_separator(void) {
    return trq_string_from_cstr("/");
}

/*============================================================================
 * Additional Console I/O
 *============================================================================*/

#include <stdarg.h>

void trq_printf(const char* format, ...) {
    va_list args;
    va_start(args, format);
    vprintf(format, args);
    va_end(args);
}

void trq_print_error(TrqString* str) {
    if (str && str->data) {
        fwrite(str->data, 1, str->len, stderr);
    }
}

int64_t trq_input_int(void) {
    TrqString* line = trq_input();
    int64_t result = trq_string_to_int(line);
    trq_release(line);
    return result;
}

double trq_input_float(void) {
    TrqString* line = trq_input();
    double result = trq_string_to_float(line);
    trq_release(line);
    return result;
}
