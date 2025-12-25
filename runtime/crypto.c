/**
 * Tarqeem Cryptography Runtime
 *
 * Provides SHA-256 hashing and hex encoding functions.
 * SHA-256 implementation is embedded (public domain) to avoid external dependencies.
 */

#include "tarqeem_rt.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

/*============================================================================
 * SHA-256 Implementation (Public Domain)
 *============================================================================*/

/* SHA-256 constants */
static const uint32_t K256[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
};

/* SHA-256 context structure */
typedef struct {
    uint32_t state[8];
    uint64_t count;
    uint8_t buffer[64];
} SHA256_CTX;

/* Rotate right */
#define ROTR(x, n) (((x) >> (n)) | ((x) << (32 - (n))))

/* SHA-256 functions */
#define CH(x, y, z)  (((x) & (y)) ^ (~(x) & (z)))
#define MAJ(x, y, z) (((x) & (y)) ^ ((x) & (z)) ^ ((y) & (z)))
#define EP0(x)       (ROTR(x, 2) ^ ROTR(x, 13) ^ ROTR(x, 22))
#define EP1(x)       (ROTR(x, 6) ^ ROTR(x, 11) ^ ROTR(x, 25))
#define SIG0(x)      (ROTR(x, 7) ^ ROTR(x, 18) ^ ((x) >> 3))
#define SIG1(x)      (ROTR(x, 17) ^ ROTR(x, 19) ^ ((x) >> 10))

static void sha256_init(SHA256_CTX* ctx) {
    ctx->state[0] = 0x6a09e667;
    ctx->state[1] = 0xbb67ae85;
    ctx->state[2] = 0x3c6ef372;
    ctx->state[3] = 0xa54ff53a;
    ctx->state[4] = 0x510e527f;
    ctx->state[5] = 0x9b05688c;
    ctx->state[6] = 0x1f83d9ab;
    ctx->state[7] = 0x5be0cd19;
    ctx->count = 0;
}

static void sha256_transform(SHA256_CTX* ctx, const uint8_t* data) {
    uint32_t a, b, c, d, e, f, g, h, t1, t2, w[64];
    int i;

    /* Prepare message schedule */
    for (i = 0; i < 16; i++) {
        w[i] = ((uint32_t)data[i * 4] << 24) |
               ((uint32_t)data[i * 4 + 1] << 16) |
               ((uint32_t)data[i * 4 + 2] << 8) |
               ((uint32_t)data[i * 4 + 3]);
    }
    for (i = 16; i < 64; i++) {
        w[i] = SIG1(w[i - 2]) + w[i - 7] + SIG0(w[i - 15]) + w[i - 16];
    }

    /* Initialize working variables */
    a = ctx->state[0];
    b = ctx->state[1];
    c = ctx->state[2];
    d = ctx->state[3];
    e = ctx->state[4];
    f = ctx->state[5];
    g = ctx->state[6];
    h = ctx->state[7];

    /* Main loop */
    for (i = 0; i < 64; i++) {
        t1 = h + EP1(e) + CH(e, f, g) + K256[i] + w[i];
        t2 = EP0(a) + MAJ(a, b, c);
        h = g;
        g = f;
        f = e;
        e = d + t1;
        d = c;
        c = b;
        b = a;
        a = t1 + t2;
    }

    /* Update state */
    ctx->state[0] += a;
    ctx->state[1] += b;
    ctx->state[2] += c;
    ctx->state[3] += d;
    ctx->state[4] += e;
    ctx->state[5] += f;
    ctx->state[6] += g;
    ctx->state[7] += h;
}

static void sha256_update(SHA256_CTX* ctx, const uint8_t* data, size_t len) {
    size_t i;
    size_t index = (size_t)(ctx->count & 63);
    ctx->count += len;

    for (i = 0; i < len; i++) {
        ctx->buffer[index++] = data[i];
        if (index == 64) {
            sha256_transform(ctx, ctx->buffer);
            index = 0;
        }
    }
}

static void sha256_final(SHA256_CTX* ctx, uint8_t hash[32]) {
    size_t index = (size_t)(ctx->count & 63);
    uint64_t bits = ctx->count * 8;
    int i;

    /* Padding */
    ctx->buffer[index++] = 0x80;
    if (index > 56) {
        while (index < 64) ctx->buffer[index++] = 0x00;
        sha256_transform(ctx, ctx->buffer);
        index = 0;
    }
    while (index < 56) ctx->buffer[index++] = 0x00;

    /* Append length (big-endian) */
    for (i = 7; i >= 0; i--) {
        ctx->buffer[56 + (7 - i)] = (uint8_t)(bits >> (i * 8));
    }
    sha256_transform(ctx, ctx->buffer);

    /* Output hash (big-endian) */
    for (i = 0; i < 8; i++) {
        hash[i * 4] = (uint8_t)(ctx->state[i] >> 24);
        hash[i * 4 + 1] = (uint8_t)(ctx->state[i] >> 16);
        hash[i * 4 + 2] = (uint8_t)(ctx->state[i] >> 8);
        hash[i * 4 + 3] = (uint8_t)(ctx->state[i]);
    }
}

/*============================================================================
 * Hex Encoding
 *============================================================================*/

static const char HEX_CHARS[] = "0123456789abcdef";

static void bytes_to_hex(const uint8_t* bytes, size_t len, char* hex) {
    for (size_t i = 0; i < len; i++) {
        hex[i * 2] = HEX_CHARS[(bytes[i] >> 4) & 0x0F];
        hex[i * 2 + 1] = HEX_CHARS[bytes[i] & 0x0F];
    }
    hex[len * 2] = '\0';
}

static int hex_char_to_nibble(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

/*============================================================================
 * Public API - SHA-256 Hashing
 *============================================================================*/

/**
 * Compute SHA-256 hash of a string.
 * @param content String to hash
 * @return Hex-encoded hash string (64 characters)
 */
TrqString* trq_sha256_string(TrqString* content) {
    if (!content) {
        return trq_string_from_cstr("");
    }

    SHA256_CTX ctx;
    uint8_t hash[32];
    char hex[65];

    sha256_init(&ctx);
    sha256_update(&ctx, (const uint8_t*)content->data, (size_t)content->len);
    sha256_final(&ctx, hash);
    bytes_to_hex(hash, 32, hex);

    return trq_string_from_cstr(hex);
}

/**
 * Compute SHA-256 hash of a file.
 * @param path Path to file
 * @return Hex-encoded hash string (64 characters), or empty string on error
 */
TrqString* trq_sha256_file(TrqString* path) {
    if (!path || path->len == 0) {
        return trq_string_from_cstr("");
    }

    FILE* file = fopen(path->data, "rb");
    if (!file) {
        return trq_string_from_cstr("");
    }

    SHA256_CTX ctx;
    uint8_t buffer[8192];
    uint8_t hash[32];
    char hex[65];
    size_t bytes_read;

    sha256_init(&ctx);
    while ((bytes_read = fread(buffer, 1, sizeof(buffer), file)) > 0) {
        sha256_update(&ctx, buffer, bytes_read);
    }
    fclose(file);
    sha256_final(&ctx, hash);
    bytes_to_hex(hash, 32, hex);

    return trq_string_from_cstr(hex);
}

/**
 * Compute SHA-256 hash of binary data (byte array).
 * @param data Byte array
 * @return Hex-encoded hash string (64 characters)
 */
TrqString* trq_sha256_bytes(TrqArray* data) {
    if (!data || data->len == 0) {
        /* Hash of empty data */
        return trq_string_from_cstr("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    SHA256_CTX ctx;
    uint8_t hash[32];
    char hex[65];

    sha256_init(&ctx);

    /* Handle array data - each element is an int64_t, extract as byte */
    int64_t* int_data = (int64_t*)data->data;
    for (int64_t i = 0; i < data->len; i++) {
        uint8_t byte = (uint8_t)(int_data[i] & 0xFF);
        sha256_update(&ctx, &byte, 1);
    }

    sha256_final(&ctx, hash);
    bytes_to_hex(hash, 32, hex);

    return trq_string_from_cstr(hex);
}

/**
 * Compare two hashes using constant-time comparison.
 * This prevents timing attacks.
 * @param hash1 First hash
 * @param hash2 Second hash
 * @return true if equal
 */
bool trq_sha256_compare(TrqString* hash1, TrqString* hash2) {
    if (!hash1 || !hash2) return false;
    if (hash1->len != hash2->len) return false;

    volatile uint8_t result = 0;
    for (int64_t i = 0; i < hash1->len; i++) {
        result |= (uint8_t)(hash1->data[i] ^ hash2->data[i]);
    }
    return result == 0;
}

/*============================================================================
 * Public API - Hex Encoding
 *============================================================================*/

/**
 * Encode string to hexadecimal.
 * @param content String to encode
 * @return Hex-encoded string
 */
TrqString* trq_hex_encode(TrqString* content) {
    if (!content || content->len == 0) {
        return trq_string_from_cstr("");
    }

    size_t hex_len = (size_t)(content->len * 2);
    char* hex = malloc(hex_len + 1);
    if (!hex) {
        return trq_string_from_cstr("");
    }

    bytes_to_hex((const uint8_t*)content->data, (size_t)content->len, hex);

    TrqString* result = trq_string_from_cstr(hex);
    free(hex);
    return result;
}

/**
 * Decode hexadecimal string.
 * @param encoded Hex-encoded string
 * @return Decoded string
 */
TrqString* trq_hex_decode(TrqString* encoded) {
    if (!encoded || encoded->len == 0 || encoded->len % 2 != 0) {
        return trq_string_from_cstr("");
    }

    size_t out_len = (size_t)(encoded->len / 2);
    char* decoded = malloc(out_len + 1);
    if (!decoded) {
        return trq_string_from_cstr("");
    }

    for (size_t i = 0; i < out_len; i++) {
        int high = hex_char_to_nibble(encoded->data[i * 2]);
        int low = hex_char_to_nibble(encoded->data[i * 2 + 1]);
        if (high < 0 || low < 0) {
            free(decoded);
            return trq_string_from_cstr("");
        }
        decoded[i] = (char)((high << 4) | low);
    }
    decoded[out_len] = '\0';

    TrqString* result = trq_string_new(decoded, (int64_t)out_len);
    free(decoded);
    return result;
}

/**
 * Encode byte array to hexadecimal string.
 * @param data Byte array
 * @return Hex-encoded string
 */
TrqString* trq_hex_encode_bytes(TrqArray* data) {
    if (!data || data->len == 0) {
        return trq_string_from_cstr("");
    }

    size_t hex_len = (size_t)(data->len * 2);
    char* hex = malloc(hex_len + 1);
    if (!hex) {
        return trq_string_from_cstr("");
    }

    int64_t* int_data = (int64_t*)data->data;
    for (int64_t i = 0; i < data->len; i++) {
        uint8_t byte = (uint8_t)(int_data[i] & 0xFF);
        hex[i * 2] = HEX_CHARS[(byte >> 4) & 0x0F];
        hex[i * 2 + 1] = HEX_CHARS[byte & 0x0F];
    }
    hex[hex_len] = '\0';

    TrqString* result = trq_string_from_cstr(hex);
    free(hex);
    return result;
}

/**
 * Decode hexadecimal string to byte array.
 * @param encoded Hex-encoded string
 * @return Byte array
 */
TrqArray* trq_hex_decode_to_bytes(TrqString* encoded) {
    TrqArray* result = trq_array_new(0, sizeof(int64_t));

    if (!encoded || encoded->len == 0 || encoded->len % 2 != 0) {
        return result;
    }

    size_t out_len = (size_t)(encoded->len / 2);
    for (size_t i = 0; i < out_len; i++) {
        int high = hex_char_to_nibble(encoded->data[i * 2]);
        int low = hex_char_to_nibble(encoded->data[i * 2 + 1]);
        if (high < 0 || low < 0) {
            return result;
        }
        int64_t byte = (int64_t)((high << 4) | low);
        trq_array_push(result, &byte, sizeof(int64_t));
    }

    return result;
}
