/**
 * Tarqeem Compression Runtime
 *
 * Provides gzip compression and decompression functions.
 * Uses system zlib library.
 */

#include "tarqeem_rt.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <zlib.h>

/*============================================================================
 * Internal Helpers
 *============================================================================*/

/* Initial buffer size for decompression */
#define DECOMPRESS_INITIAL_SIZE 4096

/* Chunk size for file operations */
#define CHUNK_SIZE 16384

/*============================================================================
 * Public API - String Compression
 *============================================================================*/

/**
 * Compress a string using gzip.
 * @param content String to compress
 * @return Byte array containing compressed data
 */
TrqArray* trq_gzip_compress_string(TrqString* content) {
    TrqArray* result = trq_array_new(0, sizeof(int64_t));

    if (!content || content->len == 0) {
        return result;
    }

    /* Estimate compressed size */
    uLongf dest_len = compressBound((uLong)content->len) + 18; /* +18 for gzip header/trailer */
    uint8_t* dest = malloc(dest_len);
    if (!dest) {
        return result;
    }

    /* Initialize zlib stream for gzip */
    z_stream stream;
    memset(&stream, 0, sizeof(stream));

    /* deflateInit2 with windowBits = 15 + 16 for gzip format */
    if (deflateInit2(&stream, Z_DEFAULT_COMPRESSION, Z_DEFLATED,
                     15 + 16, 8, Z_DEFAULT_STRATEGY) != Z_OK) {
        free(dest);
        return result;
    }

    stream.next_in = (Bytef*)content->data;
    stream.avail_in = (uInt)content->len;
    stream.next_out = dest;
    stream.avail_out = (uInt)dest_len;

    if (deflate(&stream, Z_FINISH) != Z_STREAM_END) {
        deflateEnd(&stream);
        free(dest);
        return result;
    }

    uLongf compressed_len = stream.total_out;
    deflateEnd(&stream);

    /* Copy to result array */
    for (uLongf i = 0; i < compressed_len; i++) {
        int64_t byte = (int64_t)dest[i];
        trq_array_push(result, &byte, sizeof(int64_t));
    }

    free(dest);
    return result;
}

/**
 * Decompress gzip data to string.
 * @param compressed Byte array containing compressed data
 * @return Decompressed string
 */
TrqString* trq_gzip_decompress_to_string(TrqArray* compressed) {
    if (!compressed || compressed->len == 0) {
        return trq_string_from_cstr("");
    }

    /* Convert int64_t array to uint8_t array */
    size_t in_len = (size_t)compressed->len;
    uint8_t* in_data = malloc(in_len);
    if (!in_data) {
        return trq_string_from_cstr("");
    }

    int64_t* int_data = (int64_t*)compressed->data;
    for (size_t i = 0; i < in_len; i++) {
        in_data[i] = (uint8_t)(int_data[i] & 0xFF);
    }

    /* Initialize zlib stream for gzip decompression */
    z_stream stream;
    memset(&stream, 0, sizeof(stream));

    /* inflateInit2 with windowBits = 15 + 16 for gzip format */
    if (inflateInit2(&stream, 15 + 16) != Z_OK) {
        free(in_data);
        return trq_string_from_cstr("");
    }

    stream.next_in = in_data;
    stream.avail_in = (uInt)in_len;

    /* Dynamic output buffer */
    size_t out_size = DECOMPRESS_INITIAL_SIZE;
    size_t out_len = 0;
    uint8_t* out_data = malloc(out_size);
    if (!out_data) {
        inflateEnd(&stream);
        free(in_data);
        return trq_string_from_cstr("");
    }

    int ret;
    do {
        /* Expand buffer if needed */
        if (out_len >= out_size) {
            out_size *= 2;
            uint8_t* new_data = realloc(out_data, out_size);
            if (!new_data) {
                inflateEnd(&stream);
                free(in_data);
                free(out_data);
                return trq_string_from_cstr("");
            }
            out_data = new_data;
        }

        stream.next_out = out_data + out_len;
        stream.avail_out = (uInt)(out_size - out_len);

        ret = inflate(&stream, Z_NO_FLUSH);
        if (ret == Z_STREAM_ERROR || ret == Z_DATA_ERROR || ret == Z_MEM_ERROR) {
            inflateEnd(&stream);
            free(in_data);
            free(out_data);
            return trq_string_from_cstr("");
        }

        out_len = stream.total_out;
    } while (ret != Z_STREAM_END);

    inflateEnd(&stream);
    free(in_data);

    /* Create result string */
    TrqString* result = trq_string_new((char*)out_data, (int64_t)out_len);
    free(out_data);

    return result;
}

/*============================================================================
 * Public API - Byte Array Compression
 *============================================================================*/

/**
 * Compress binary data using gzip.
 * @param data Byte array to compress
 * @return Byte array containing compressed data
 */
TrqArray* trq_gzip_compress_bytes(TrqArray* data) {
    TrqArray* result = trq_array_new(0, sizeof(int64_t));

    if (!data || data->len == 0) {
        return result;
    }

    /* Convert int64_t array to uint8_t array */
    size_t in_len = (size_t)data->len;
    uint8_t* in_data = malloc(in_len);
    if (!in_data) {
        return result;
    }

    int64_t* int_data = (int64_t*)data->data;
    for (size_t i = 0; i < in_len; i++) {
        in_data[i] = (uint8_t)(int_data[i] & 0xFF);
    }

    /* Estimate compressed size */
    uLongf dest_len = compressBound((uLong)in_len) + 18;
    uint8_t* dest = malloc(dest_len);
    if (!dest) {
        free(in_data);
        return result;
    }

    /* Initialize zlib stream for gzip */
    z_stream stream;
    memset(&stream, 0, sizeof(stream));

    if (deflateInit2(&stream, Z_DEFAULT_COMPRESSION, Z_DEFLATED,
                     15 + 16, 8, Z_DEFAULT_STRATEGY) != Z_OK) {
        free(in_data);
        free(dest);
        return result;
    }

    stream.next_in = in_data;
    stream.avail_in = (uInt)in_len;
    stream.next_out = dest;
    stream.avail_out = (uInt)dest_len;

    if (deflate(&stream, Z_FINISH) != Z_STREAM_END) {
        deflateEnd(&stream);
        free(in_data);
        free(dest);
        return result;
    }

    uLongf compressed_len = stream.total_out;
    deflateEnd(&stream);
    free(in_data);

    /* Copy to result array */
    for (uLongf i = 0; i < compressed_len; i++) {
        int64_t byte = (int64_t)dest[i];
        trq_array_push(result, &byte, sizeof(int64_t));
    }

    free(dest);
    return result;
}

/**
 * Decompress gzip data to byte array.
 * @param compressed Byte array containing compressed data
 * @return Decompressed byte array
 */
TrqArray* trq_gzip_decompress_bytes(TrqArray* compressed) {
    TrqArray* result = trq_array_new(0, sizeof(int64_t));

    if (!compressed || compressed->len == 0) {
        return result;
    }

    /* Convert int64_t array to uint8_t array */
    size_t in_len = (size_t)compressed->len;
    uint8_t* in_data = malloc(in_len);
    if (!in_data) {
        return result;
    }

    int64_t* int_arr = (int64_t*)compressed->data;
    for (size_t i = 0; i < in_len; i++) {
        in_data[i] = (uint8_t)(int_arr[i] & 0xFF);
    }

    /* Initialize zlib stream */
    z_stream stream;
    memset(&stream, 0, sizeof(stream));

    if (inflateInit2(&stream, 15 + 16) != Z_OK) {
        free(in_data);
        return result;
    }

    stream.next_in = in_data;
    stream.avail_in = (uInt)in_len;

    /* Dynamic output buffer */
    size_t out_size = DECOMPRESS_INITIAL_SIZE;
    size_t out_len = 0;
    uint8_t* out_data = malloc(out_size);
    if (!out_data) {
        inflateEnd(&stream);
        free(in_data);
        return result;
    }

    int ret;
    do {
        if (out_len >= out_size) {
            out_size *= 2;
            uint8_t* new_data = realloc(out_data, out_size);
            if (!new_data) {
                inflateEnd(&stream);
                free(in_data);
                free(out_data);
                return result;
            }
            out_data = new_data;
        }

        stream.next_out = out_data + out_len;
        stream.avail_out = (uInt)(out_size - out_len);

        ret = inflate(&stream, Z_NO_FLUSH);
        if (ret == Z_STREAM_ERROR || ret == Z_DATA_ERROR || ret == Z_MEM_ERROR) {
            inflateEnd(&stream);
            free(in_data);
            free(out_data);
            return result;
        }

        out_len = stream.total_out;
    } while (ret != Z_STREAM_END);

    inflateEnd(&stream);
    free(in_data);

    /* Copy to result array */
    for (size_t i = 0; i < out_len; i++) {
        int64_t byte = (int64_t)out_data[i];
        trq_array_push(result, &byte, sizeof(int64_t));
    }

    free(out_data);
    return result;
}

/*============================================================================
 * Public API - File Compression
 *============================================================================*/

/**
 * Compress a file to gzip format.
 * @param source Path to source file
 * @param dest Path to destination file (.gz)
 * @return true on success
 */
bool trq_gzip_compress_file(TrqString* source, TrqString* dest) {
    if (!source || !dest || source->len == 0 || dest->len == 0) {
        return false;
    }

    FILE* src_file = fopen(source->data, "rb");
    if (!src_file) {
        return false;
    }

    gzFile gz_file = gzopen(dest->data, "wb");
    if (!gz_file) {
        fclose(src_file);
        return false;
    }

    uint8_t buffer[CHUNK_SIZE];
    size_t bytes_read;

    while ((bytes_read = fread(buffer, 1, CHUNK_SIZE, src_file)) > 0) {
        if (gzwrite(gz_file, buffer, (unsigned int)bytes_read) != (int)bytes_read) {
            gzclose(gz_file);
            fclose(src_file);
            return false;
        }
    }

    gzclose(gz_file);
    fclose(src_file);
    return true;
}

/**
 * Decompress a gzip file.
 * @param source Path to gzip file
 * @param dest Path to destination file
 * @return true on success
 */
bool trq_gzip_decompress_file(TrqString* source, TrqString* dest) {
    if (!source || !dest || source->len == 0 || dest->len == 0) {
        return false;
    }

    gzFile gz_file = gzopen(source->data, "rb");
    if (!gz_file) {
        return false;
    }

    FILE* dest_file = fopen(dest->data, "wb");
    if (!dest_file) {
        gzclose(gz_file);
        return false;
    }

    uint8_t buffer[CHUNK_SIZE];
    int bytes_read;

    while ((bytes_read = gzread(gz_file, buffer, CHUNK_SIZE)) > 0) {
        if (fwrite(buffer, 1, (size_t)bytes_read, dest_file) != (size_t)bytes_read) {
            fclose(dest_file);
            gzclose(gz_file);
            return false;
        }
    }

    /* Check for decompression errors */
    int err;
    gzerror(gz_file, &err);
    if (err != Z_OK && err != Z_STREAM_END) {
        fclose(dest_file);
        gzclose(gz_file);
        return false;
    }

    fclose(dest_file);
    gzclose(gz_file);
    return true;
}
