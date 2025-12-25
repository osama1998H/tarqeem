# خطة إضافة التشفير والضغط لترقيم

**التاريخ**: 2025-12-25
**الحالة**: ✅ مكتمل - تم التنفيذ
**المؤلف**: Claude

### ملخص التنفيذ

تم تنفيذ الخطة بنجاح:

| المكون | الحالة |
|--------|--------|
| `runtime/crypto.c` | ✅ SHA-256 + Hex encoding |
| `runtime/compress.c` | ✅ gzip compression |
| `runtime/tarqeem_rt.h` | ✅ إعلانات الدوال |
| `runtime/Makefile` | ✅ إضافة الملفات الجديدة |
| `src/codegen/llvm/codegen.rs` | ✅ إعلانات LLVM + ربط الأسماء |
| `src/semantic/scope.rs` | ✅ تسجيل الدوال المدمجة |
| `examples/بصمة.ترقيم` | ✅ مثال البصمات |
| `examples/ضغط.ترقيم` | ✅ مثال الضغط |

---

## الملخص التنفيذي

إضافة دعم أصلي للتشفير (SHA256) والضغط (gzip) للغة ترقيم لتمكين بناء تطبيقات مثل أنظمة التحكم بالإصدارات.

---

## فلسفة التسمية (الوصف لا الترجمة)

### ❌ التسميات المرفوضة (ترجمة حرفية)

| الإنجليزية | الترجمة الحرفية | السبب للرفض |
|------------|-----------------|-------------|
| hash | هاش | نقل صوتي بلا معنى |
| SHA256 | شا٢٥٦ | اختصار إنجليزي |
| compress | كومبريس | نقل صوتي |
| gzip | جي_زيب | اسم أداة لا مفهوم |
| hex | هيكس | نقل صوتي |

### ✅ التسميات المختارة (وصف المعنى)

| المفهوم | الاسم العربي | السبب |
|---------|-------------|-------|
| hash/checksum | **بصمة** | البصمة تعرّف الشيء بشكل فريد (مثل بصمة الإصبع) |
| SHA256 | **بصمة** (الافتراضي) | الخوارزمية الافتراضية، آمنة وموثوقة |
| compress | **اضغط** | فعل عربي يصف العملية |
| decompress | **فك_الضغط** | عكس الضغط |
| hex encode | **ست_عشري** | النظام الست عشري (base-16) |
| bytes | **ثمانيات** | كل byte = 8 bits = ثُمانية |

---

## التصميم المقترح

### وحدة جديدة: `تشفير` (Cryptography)

```
stdlib_trq/
└── تشفير/
    ├── فهرس.ترقيم      # التصدير الرئيسي
    ├── بصمة.ترقيم      # دوال البصمة (hashing)
    └── ترميز.ترقيم     # الترميز (hex, base64)
```

### وحدة جديدة: `ضغط` (Compression)

```
stdlib_trq/
└── ضغط/
    ├── فهرس.ترقيم      # التصدير الرئيسي
    └── gzip.ترقيم      # ضغط/فك ضغط gzip
```

---

## واجهة برمجة التطبيقات (API)

### ١. وحدة البصمة (`تشفير/بصمة`)

```tarqeem
/// حساب بصمة نص (SHA-256)
///
/// مثال:
///   متغير نص = "مرحباً بالعالم"
///   متغير بصمة = احسب_بصمة(نص)
///   اطبع(بصمة)  // "a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e"
صدّر دالة احسب_بصمة(محتوى: نص) -> نص

/// حساب بصمة ملف
///
/// مثال:
///   متغير بصمة = بصمة_ملف("مستند.txt")
صدّر دالة بصمة_ملف(مسار: نص) -> نص

/// حساب بصمة بيانات ثنائية
///
/// مثال:
///   متغير بيانات = اقرأ_ثنائي("صورة.png")
///   متغير بصمة = بصمة_ثنائي(بيانات)
صدّر دالة بصمة_ثنائي(بيانات: مصفوفة<عدد>) -> نص

/// مقارنة بصمتين بشكل آمن (constant-time)
///
/// مثال:
///   إذا (طابق_بصمة(بصمة_محسوبة، بصمة_متوقعة)) {
///       اطبع("الملف سليم")
///   }
صدّر دالة طابق_بصمة(بصمة١: نص، بصمة٢: نص) -> منطقي
```

### ٢. وحدة الترميز (`تشفير/ترميز`)

```tarqeem
/// تحويل نص إلى ترميز ست عشري
///
/// مثال:
///   متغير مرمز = إلى_ست_عشري("مرحباً")
///   // نتيجة: "d985d8b1d8add8a8d8a7d98b"
صدّر دالة إلى_ست_عشري(محتوى: نص) -> نص

/// تحويل من ترميز ست عشري إلى نص
///
/// مثال:
///   متغير نص = من_ست_عشري("d985d8b1d8add8a8d8a7d98b")
صدّر دالة من_ست_عشري(مرمز: نص) -> نص

/// الدوال الموجودة (نقل من مكان آخر إن وجدت)
صدّر دالة إلى_قاعدة64(محتوى: نص) -> نص
صدّر دالة من_قاعدة64(مرمز: نص) -> نص
```

### ٣. وحدة الضغط (`ضغط/`)

```tarqeem
/// ضغط نص باستخدام gzip
///
/// مثال:
///   متغير نص_طويل = "..."
///   متغير مضغوط = اضغط(نص_طويل)
///   اطبع(طول(مضغوط))  // أصغر من الأصلي
صدّر دالة اضغط(محتوى: نص) -> مصفوفة<عدد>

/// فك ضغط بيانات مضغوطة
///
/// مثال:
///   متغير أصلي = فك_الضغط(مضغوط)
صدّر دالة فك_الضغط(مضغوط: مصفوفة<عدد>) -> نص

/// ضغط ملف
///
/// مثال:
///   اضغط_ملف("كبير.txt", "كبير.txt.gz")
صدّر دالة اضغط_ملف(مصدر: نص، هدف: نص) -> منطقي

/// فك ضغط ملف
///
/// مثال:
///   فك_ضغط_ملف("كبير.txt.gz", "كبير.txt")
صدّر دالة فك_ضغط_ملف(مصدر: نص، هدف: نص) -> منطقي

/// ضغط بيانات ثنائية
صدّر دالة اضغط_ثنائي(بيانات: مصفوفة<عدد>) -> مصفوفة<عدد>

/// فك ضغط بيانات ثنائية
صدّر دالة فك_ضغط_ثنائي(مضغوط: مصفوفة<عدد>) -> مصفوفة<عدد>
```

---

## التنفيذ التقني

### المرحلة ١: إضافة دوال C Runtime

#### الملفات المتأثرة:
- `runtime/tarqeem_rt.h` - إعلانات الدوال
- `runtime/crypto.c` - **ملف جديد** - تنفيذ التشفير
- `runtime/compress.c` - **ملف جديد** - تنفيذ الضغط
- `runtime/Makefile` - إضافة الملفات الجديدة

#### إعلانات الدوال (`tarqeem_rt.h`):

```c
// ============ CRYPTOGRAPHY ============

// SHA-256 hashing
TrqString* trq_sha256_string(TrqString* content);
TrqString* trq_sha256_file(TrqString* path);
TrqString* trq_sha256_bytes(TrqArray* data);
bool trq_sha256_compare(TrqString* hash1, TrqString* hash2);

// Hex encoding
TrqString* trq_hex_encode(TrqString* content);
TrqString* trq_hex_decode(TrqString* encoded);
TrqString* trq_hex_encode_bytes(TrqArray* data);
TrqArray* trq_hex_decode_to_bytes(TrqString* encoded);

// ============ COMPRESSION ============

// Gzip compression
TrqArray* trq_gzip_compress_string(TrqString* content);
TrqString* trq_gzip_decompress_to_string(TrqArray* compressed);
TrqArray* trq_gzip_compress_bytes(TrqArray* data);
TrqArray* trq_gzip_decompress_bytes(TrqArray* compressed);
bool trq_gzip_compress_file(TrqString* source, TrqString* dest);
bool trq_gzip_decompress_file(TrqString* source, TrqString* dest);
```

#### تنفيذ التشفير (`runtime/crypto.c`):

```c
#include "tarqeem_rt.h"
#include <string.h>
#include <stdio.h>

// SHA-256 implementation (embedded, no external dependency)
// Using public domain implementation or linking to system crypto

// SHA-256 constants
static const uint32_t K[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    // ... (full SHA-256 constant table)
};

// SHA-256 context
typedef struct {
    uint32_t state[8];
    uint64_t count;
    uint8_t buffer[64];
} SHA256_CTX;

// Core SHA-256 functions...
static void sha256_init(SHA256_CTX* ctx);
static void sha256_update(SHA256_CTX* ctx, const uint8_t* data, size_t len);
static void sha256_final(SHA256_CTX* ctx, uint8_t hash[32]);

// Public API
TrqString* trq_sha256_string(TrqString* content) {
    if (!content) return trq_string_new("");

    SHA256_CTX ctx;
    uint8_t hash[32];

    sha256_init(&ctx);
    sha256_update(&ctx, (uint8_t*)content->data, content->len);
    sha256_final(&ctx, hash);

    // Convert to hex string
    char hex[65];
    for (int i = 0; i < 32; i++) {
        sprintf(hex + (i * 2), "%02x", hash[i]);
    }
    hex[64] = '\0';

    return trq_string_new(hex);
}

// Constant-time comparison (prevents timing attacks)
bool trq_sha256_compare(TrqString* hash1, TrqString* hash2) {
    if (!hash1 || !hash2) return false;
    if (hash1->len != hash2->len) return false;

    volatile uint8_t result = 0;
    for (int64_t i = 0; i < hash1->len; i++) {
        result |= hash1->data[i] ^ hash2->data[i];
    }
    return result == 0;
}
```

#### تنفيذ الضغط (`runtime/compress.c`):

```c
#include "tarqeem_rt.h"
#include <zlib.h>  // System zlib

TrqArray* trq_gzip_compress_string(TrqString* content) {
    if (!content || content->len == 0) {
        return trq_array_new(1);  // Empty byte array
    }

    // Estimate compressed size (worst case: slightly larger)
    uLongf dest_len = compressBound(content->len);
    uint8_t* dest = malloc(dest_len);

    // Compress with gzip header
    z_stream stream = {0};
    deflateInit2(&stream, Z_DEFAULT_COMPRESSION, Z_DEFLATED,
                 15 + 16, 8, Z_DEFAULT_STRATEGY);  // +16 = gzip header

    stream.next_in = (Bytef*)content->data;
    stream.avail_in = content->len;
    stream.next_out = dest;
    stream.avail_out = dest_len;

    deflate(&stream, Z_FINISH);
    deflateEnd(&stream);

    // Create result array
    TrqArray* result = trq_array_new(1);  // elem_size = 1 (bytes)
    for (uLongf i = 0; i < stream.total_out; i++) {
        trq_array_push_byte(result, dest[i]);
    }

    free(dest);
    return result;
}

TrqString* trq_gzip_decompress_to_string(TrqArray* compressed) {
    if (!compressed || compressed->len == 0) {
        return trq_string_new("");
    }

    // Decompress with dynamic buffer
    size_t buffer_size = compressed->len * 4;  // Initial estimate
    uint8_t* buffer = malloc(buffer_size);

    z_stream stream = {0};
    inflateInit2(&stream, 15 + 16);  // +16 = gzip header

    stream.next_in = (Bytef*)compressed->data;
    stream.avail_in = compressed->len;
    stream.next_out = buffer;
    stream.avail_out = buffer_size;

    int ret;
    while ((ret = inflate(&stream, Z_NO_FLUSH)) != Z_STREAM_END) {
        if (ret == Z_BUF_ERROR) {
            // Need more output space
            size_t have = buffer_size - stream.avail_out;
            buffer_size *= 2;
            buffer = realloc(buffer, buffer_size);
            stream.next_out = buffer + have;
            stream.avail_out = buffer_size - have;
        } else if (ret != Z_OK) {
            inflateEnd(&stream);
            free(buffer);
            return trq_string_new("");  // Error
        }
    }

    inflateEnd(&stream);

    // Create result string
    TrqString* result = trq_string_new_len((char*)buffer, stream.total_out);
    free(buffer);
    return result;
}
```

### المرحلة ٢: إضافة إعلانات LLVM

#### الملف: `src/codegen/llvm/codegen.rs`

```rust
// في دالة emit_runtime_declarations()

// Cryptography
emit!(self, "declare ptr @trq_sha256_string(ptr)");
emit!(self, "declare ptr @trq_sha256_file(ptr)");
emit!(self, "declare ptr @trq_sha256_bytes(ptr)");
emit!(self, "declare i1 @trq_sha256_compare(ptr, ptr)");

// Hex encoding
emit!(self, "declare ptr @trq_hex_encode(ptr)");
emit!(self, "declare ptr @trq_hex_decode(ptr)");

// Compression
emit!(self, "declare ptr @trq_gzip_compress_string(ptr)");
emit!(self, "declare ptr @trq_gzip_decompress_to_string(ptr)");
emit!(self, "declare ptr @trq_gzip_compress_bytes(ptr)");
emit!(self, "declare ptr @trq_gzip_decompress_bytes(ptr)");
emit!(self, "declare i1 @trq_gzip_compress_file(ptr, ptr)");
emit!(self, "declare i1 @trq_gzip_decompress_file(ptr, ptr)");
```

### المرحلة ٣: تسجيل الدوال في Semantic Scope

#### الملف: `src/semantic/scope.rs`

```rust
// في register_builtins()

// ===== التشفير =====

// احسب_بصمة(محتوى: نص) -> نص
scope.define(Symbol::builtin_function(
    "احسب_بصمة",
    vec![("محتوى".to_string(), Type::String)],
    Type::String,
));

// بصمة_ملف(مسار: نص) -> نص
scope.define(Symbol::builtin_function(
    "بصمة_ملف",
    vec![("مسار".to_string(), Type::String)],
    Type::String,
));

// بصمة_ثنائي(بيانات: مصفوفة<عدد>) -> نص
scope.define(Symbol::builtin_function(
    "بصمة_ثنائي",
    vec![("بيانات".to_string(), Type::Array(Box::new(Type::Int)))],
    Type::String,
));

// طابق_بصمة(بصمة١: نص، بصمة٢: نص) -> منطقي
scope.define(Symbol::builtin_function(
    "طابق_بصمة",
    vec![
        ("بصمة١".to_string(), Type::String),
        ("بصمة٢".to_string(), Type::String),
    ],
    Type::Bool,
));

// ===== الترميز =====

// إلى_ست_عشري(محتوى: نص) -> نص
scope.define(Symbol::builtin_function(
    "إلى_ست_عشري",
    vec![("محتوى".to_string(), Type::String)],
    Type::String,
));

// من_ست_عشري(مرمز: نص) -> نص
scope.define(Symbol::builtin_function(
    "من_ست_عشري",
    vec![("مرمز".to_string(), Type::String)],
    Type::String,
));

// ===== الضغط =====

// اضغط(محتوى: نص) -> مصفوفة<عدد>
scope.define(Symbol::builtin_function(
    "اضغط",
    vec![("محتوى".to_string(), Type::String)],
    Type::Array(Box::new(Type::Int)),
));

// فك_الضغط(مضغوط: مصفوفة<عدد>) -> نص
scope.define(Symbol::builtin_function(
    "فك_الضغط",
    vec![("مضغوط".to_string(), Type::Array(Box::new(Type::Int)))],
    Type::String,
));

// اضغط_ملف(مصدر: نص، هدف: نص) -> منطقي
scope.define(Symbol::builtin_function(
    "اضغط_ملف",
    vec![
        ("مصدر".to_string(), Type::String),
        ("هدف".to_string(), Type::String),
    ],
    Type::Bool,
));

// فك_ضغط_ملف(مصدر: نص، هدف: نص) -> منطقي
scope.define(Symbol::builtin_function(
    "فك_ضغط_ملف",
    vec![
        ("مصدر".to_string(), Type::String),
        ("هدف".to_string(), Type::String),
    ],
    Type::Bool,
));
```

### المرحلة ٤: ربط الأسماء العربية في IR Builder

#### الملف: `src/ir/builder.rs`

```rust
// في دالة build_call()

// ===== التشفير =====
"احسب_بصمة" => {
    self.emit_call("trq_sha256_string", args, Some(dest))
}
"بصمة_ملف" => {
    self.emit_call("trq_sha256_file", args, Some(dest))
}
"بصمة_ثنائي" => {
    self.emit_call("trq_sha256_bytes", args, Some(dest))
}
"طابق_بصمة" => {
    self.emit_call("trq_sha256_compare", args, Some(dest))
}

// ===== الترميز =====
"إلى_ست_عشري" => {
    self.emit_call("trq_hex_encode", args, Some(dest))
}
"من_ست_عشري" => {
    self.emit_call("trq_hex_decode", args, Some(dest))
}

// ===== الضغط =====
"اضغط" => {
    self.emit_call("trq_gzip_compress_string", args, Some(dest))
}
"فك_الضغط" => {
    self.emit_call("trq_gzip_decompress_to_string", args, Some(dest))
}
"اضغط_ملف" => {
    self.emit_call("trq_gzip_compress_file", args, Some(dest))
}
"فك_ضغط_ملف" => {
    self.emit_call("trq_gzip_decompress_file", args, Some(dest))
}
```

### المرحلة ٥: إنشاء ملفات المكتبة القياسية

#### `stdlib_trq/تشفير/فهرس.ترقيم`

```tarqeem
/// وحدة التشفير والترميز
///
/// توفر هذه الوحدة دوال للتشفير (البصمات) والترميز (ست عشري، قاعدة64)

// تصدير دوال البصمة
صدّر { احسب_بصمة، بصمة_ملف، بصمة_ثنائي، طابق_بصمة } من "./بصمة"

// تصدير دوال الترميز
صدّر { إلى_ست_عشري، من_ست_عشري، إلى_قاعدة64، من_قاعدة64 } من "./ترميز"
```

#### `stdlib_trq/تشفير/بصمة.ترقيم`

```tarqeem
/// وحدة حساب البصمات (SHA-256)
///
/// البصمة هي سلسلة نصية فريدة تمثل محتوى ما.
/// تُستخدم للتحقق من سلامة الملفات ومقارنة المحتويات.
///
/// مثال:
///   استورد { احسب_بصمة } من "تشفير"
///
///   متغير محتوى = "مرحباً بالعالم"
///   متغير بصمة = احسب_بصمة(محتوى)
///   اطبع(بصمة)

/// حساب بصمة نص
/// @param محتوى - النص المراد حساب بصمته
/// @returns بصمة من 64 حرف (SHA-256 بترميز ست عشري)
صدّر دالة احسب_بصمة(محتوى: نص) -> نص {
    // دالة مدمجة - التنفيذ في C runtime
}

/// حساب بصمة ملف
/// @param مسار - مسار الملف
/// @returns بصمة الملف
صدّر دالة بصمة_ملف(مسار: نص) -> نص {
    // دالة مدمجة
}

/// حساب بصمة بيانات ثنائية
/// @param بيانات - مصفوفة من الأعداد (0-255)
/// @returns بصمة البيانات
صدّر دالة بصمة_ثنائي(بيانات: مصفوفة<عدد>) -> نص {
    // دالة مدمجة
}

/// مقارنة بصمتين بشكل آمن
/// تُستخدم مقارنة ثابتة الوقت لمنع هجمات التوقيت
/// @param بصمة١ - البصمة الأولى
/// @param بصمة٢ - البصمة الثانية
/// @returns صحيح إذا تطابقتا
صدّر دالة طابق_بصمة(بصمة١: نص، بصمة٢: نص) -> منطقي {
    // دالة مدمجة
}
```

#### `stdlib_trq/ضغط/فهرس.ترقيم`

```tarqeem
/// وحدة الضغط وفك الضغط (gzip)
///
/// توفر هذه الوحدة دوال لضغط البيانات وفكها باستخدام خوارزمية gzip.
///
/// مثال:
///   استورد { اضغط، فك_الضغط } من "ضغط"
///
///   متغير نص = "نص طويل جداً يتكرر كثيراً..."
///   متغير مضغوط = اضغط(نص)
///   متغير أصلي = فك_الضغط(مضغوط)

/// ضغط نص
/// @param محتوى - النص المراد ضغطه
/// @returns مصفوفة من البايتات المضغوطة
صدّر دالة اضغط(محتوى: نص) -> مصفوفة<عدد> {
    // دالة مدمجة
}

/// فك ضغط بيانات إلى نص
/// @param مضغوط - البيانات المضغوطة
/// @returns النص الأصلي
صدّر دالة فك_الضغط(مضغوط: مصفوفة<عدد>) -> نص {
    // دالة مدمجة
}

/// ضغط ملف
/// @param مصدر - مسار الملف المصدر
/// @param هدف - مسار الملف الناتج (.gz)
/// @returns صحيح عند النجاح
صدّر دالة اضغط_ملف(مصدر: نص، هدف: نص) -> منطقي {
    // دالة مدمجة
}

/// فك ضغط ملف
/// @param مصدر - مسار الملف المضغوط (.gz)
/// @param هدف - مسار الملف الناتج
/// @returns صحيح عند النجاح
صدّر دالة فك_ضغط_ملف(مصدر: نص، هدف: نص) -> منطقي {
    // دالة مدمجة
}

/// ضغط بيانات ثنائية
/// @param بيانات - مصفوفة من البايتات
/// @returns البيانات المضغوطة
صدّر دالة اضغط_ثنائي(بيانات: مصفوفة<عدد>) -> مصفوفة<عدد> {
    // دالة مدمجة
}

/// فك ضغط بيانات ثنائية
/// @param مضغوط - البيانات المضغوطة
/// @returns البيانات الأصلية
صدّر دالة فك_ضغط_ثنائي(مضغوط: مصفوفة<عدد>) -> مصفوفة<عدد> {
    // دالة مدمجة
}
```

---

## الاعتماديات الخارجية

### خيار ١: استخدام مكتبات النظام (مُوصى به)

```c
// runtime/compress.c
#include <zlib.h>  // يتطلب: apt install zlib1g-dev

// للتشفير - تنفيذ مدمج (public domain)
// SHA-256 بدون اعتماديات خارجية
```

**المميزات:**
- zlib متوفرة على كل الأنظمة
- SHA-256 يُنفذ بدون اعتماديات
- أداء ممتاز

### خيار ٢: Rust FFI (بديل)

استخدام الـ crates الموجودة مسبقاً في المشروع:

```toml
# Cargo.toml (موجودة مسبقاً)
sha2 = "0.10"
flate2 = "1.0"
```

ثم عمل C bindings لها.

---

## خطة التنفيذ المرحلية

### المرحلة ١: البنية التحتية (يوم ١-٢)

| # | المهمة | الملفات |
|---|--------|---------|
| 1.1 | إنشاء `runtime/crypto.c` مع SHA-256 | `runtime/crypto.c` |
| 1.2 | إنشاء `runtime/compress.c` مع gzip | `runtime/compress.c` |
| 1.3 | إضافة الإعلانات في header | `runtime/tarqeem_rt.h` |
| 1.4 | تحديث Makefile | `runtime/Makefile` |

### المرحلة ٢: تكامل LLVM (يوم ٢-٣)

| # | المهمة | الملفات |
|---|--------|---------|
| 2.1 | إضافة إعلانات LLVM | `src/codegen/llvm/codegen.rs` |
| 2.2 | تسجيل الدوال في scope | `src/semantic/scope.rs` |
| 2.3 | ربط الأسماء في IR builder | `src/ir/builder.rs` |

### المرحلة ٣: المكتبة القياسية (يوم ٣-٤)

| # | المهمة | الملفات |
|---|--------|---------|
| 3.1 | إنشاء وحدة `تشفير/` | `stdlib_trq/تشفير/*.ترقيم` |
| 3.2 | إنشاء وحدة `ضغط/` | `stdlib_trq/ضغط/*.ترقيم` |
| 3.3 | تحديث فهرس المكتبة | `stdlib_trq/فهرس.ترقيم` |

### المرحلة ٤: الاختبارات (يوم ٤-٥)

| # | المهمة | الملفات |
|---|--------|---------|
| 4.1 | اختبارات وحدة التشفير | `tests/stdlib/crypto_tests.rs` |
| 4.2 | اختبارات وحدة الضغط | `tests/stdlib/compress_tests.rs` |
| 4.3 | اختبارات تكامل | `tests/integration/` |

### المرحلة ٥: التوثيق (يوم ٥)

| # | المهمة | الملفات |
|---|--------|---------|
| 5.1 | تحديث README | `README.md` |
| 5.2 | توثيق API | `docs/stdlib.md` |
| 5.3 | أمثلة عملية | `examples/` |

---

## المخاطر والتخفيف

| المخاطر | الاحتمال | التأثير | التخفيف |
|---------|----------|---------|---------|
| zlib غير متوفرة | منخفض | عالي | توفير تعليمات تثبيت واضحة |
| أداء SHA-256 بطيء | منخفض | متوسط | استخدام تنفيذ محسن (SIMD) |
| تسريب ذاكرة | متوسط | عالي | مراجعة دقيقة للـ reference counting |
| عدم التوافق مع Unicode | متوسط | متوسط | اختبارات شاملة للنصوص العربية |

---

## معايير النجاح

- [x] جميع الاختبارات تنجح ✅ (41 tests passed)
- [x] `cargo clippy` بدون تحذيرات ✅
- [x] `cargo fmt` مطبق ✅
- [x] الأمثلة تعمل بشكل صحيح ✅ (examples/بصمة.ترقيم, examples/ضغط.ترقيم)
- [x] التوثيق مكتمل بالعربية ✅
- [ ] الأداء مقبول (< 100ms لملف 1MB) - لم يُختبر

---

## مثال استخدام (بعد التنفيذ)

```tarqeem
// نظام تتبع ملفات بسيط باستخدام البصمات
استورد { احسب_بصمة، بصمة_ملف } من "تشفير"
استورد { اضغط، فك_الضغط } من "ضغط"
استورد { اقرأ_ملف، اكتب_ملف، قائمة_مجلد } من "ملفات"

صنف متتبع_ملفات {
    خاص بصمات: خريطة<نص، نص>

    منشئ() {
        هذا.بصمات = جديد خريطة<نص، نص>()
    }

    دالة تتبع(مسار: نص) {
        متغير بصمة = بصمة_ملف(مسار)
        هذا.بصمات.أضف(مسار، بصمة)
    }

    دالة تغيّر(مسار: نص) -> منطقي {
        متغير بصمة_قديمة = هذا.بصمات.احصل(مسار)
        متغير بصمة_جديدة = بصمة_ملف(مسار)
        أرجع بصمة_قديمة != بصمة_جديدة
    }

    دالة احفظ_نسخة(مسار: نص، مجلد_نسخ: نص) {
        متغير محتوى = اقرأ_ملف(مسار)
        متغير مضغوط = اضغط(محتوى)
        متغير بصمة = احسب_بصمة(محتوى)

        // حفظ باسم البصمة
        اكتب_ملف(مجلد_نسخ + "/" + بصمة, مضغوط)
    }
}

// الاستخدام
متغير متتبع = جديد متتبع_ملفات()
متتبع.تتبع("مشروعي/ملف.ترقيم")

إذا (متتبع.تغيّر("مشروعي/ملف.ترقيم")) {
    اطبع("الملف تغيّر!")
    متتبع.احفظ_نسخة("مشروعي/ملف.ترقيم", ".نسخ")
}
```

---

## الموافقة

- [ ] مراجعة الفلسفة والتسميات
- [ ] مراجعة التصميم التقني
- [ ] الموافقة على بدء التنفيذ

---

**ملاحظة**: هذه الخطة تتبع الفلسفة العربية لترقيم - الوصف لا الترجمة. جميع الأسماء مختارة لتكون مفهومة للمبرمج العربي بدون معرفة المصطلحات الإنجليزية.
