# Standard Library (stdlib)

<div dir="rtl" align="right">

## المكتبة القياسية

هذا المجلد يحتوي على المكتبة القياسية لترقيم المكتوبة بلغة ترقيم نفسها.

### الوحدات

| الوحدة | الوصف |
|--------|-------|
| `مجموعات/` | مجموعات البيانات (قائمة، قاموس، مجموعة، طابور، مكدس) |
| `رياضيات/` | دوال رياضية وثوابت ومولد أرقام عشوائية |
| `نص/` | معالجة النصوص والتنسيق وباني النص |
| `ملفات/` | عمليات الملفات والمسارات والمجلدات |
| `طرفية/` | الإدخال والإخراج والألوان |
| `وقت/` | التاريخ والوقت والمدة |
| `شبكة/` | اتصالات TCP/UDP وعميل HTTP |
| `أخطاء/` | معالجة الأخطاء ونتيجة واختياري |

### الاستخدام

```tarqeem
// مجموعات
استورد { قائمة، خريطة } من "مجموعات"
متغير أرقام = جديد قائمة<عدد>()
أرقام.أضف(5)

// رياضيات
استورد { باي، مطلق، عشوائي_بين } من "رياضيات"
اطبع(باي)

// التاريخ والوقت
استورد { تاريخ، الآن } من "وقت"
متغير اليوم = تاريخ.اليوم()
اطبع(اليوم.اسم_اليوم())

// معالجة الأخطاء — غير متاحة بعد، انظر الملاحظة أدناه
استورد { نتيجة، نجاح، فشل } من "أخطاء"
متغير النتيجة = نجاح<عدد، استثناء>(42)

// الشبكة
استورد { احصل، طلب } من "شبكة"
متغير استجابة = احصل("https://example.com")
```

> **ملاحظة**: وحدة `أخطاء` غير قابلة للاستخدام بعد — ملفها
> `أخطاء/فهرس.ترقيم` يعرّف `صنف خطأ`، و`خطأ` هي القيمة المنطقية لا اسم صنف؛
> صنف الاستثناء الأساسي اسمه `استثناء`. يتابع ذلك
> [#243](https://github.com/osama1998H/tarqeem/issues/243).

</div>

## Standard Library

This directory contains the Tarqeem standard library written in Tarqeem itself.

### Modules

| Module | Description |
|--------|-------------|
| `مجموعات/` | Data collections (List, Map, Set, Queue, Stack) |
| `رياضيات/` | Math functions, constants, and random numbers |
| `نص/` | String processing, formatting, and StringBuilder |
| `ملفات/` | File, path, and directory operations |
| `طرفية/` | Console I/O and ANSI colors |
| `وقت/` | Date, time, and duration |
| `شبكة/` | TCP/UDP networking and HTTP client |
| `أخطاء/` | Error handling, Result, and Option types |

### Usage

```tarqeem
// Collections
استورد { قائمة، خريطة } من "مجموعات"
متغير numbers = جديد قائمة<عدد>()
numbers.أضف(5)

// Math
استورد { باي، مطلق، عشوائي_بين } من "رياضيات"
اطبع(باي)

// Date and Time
استورد { تاريخ، الآن } من "وقت"
متغير today = تاريخ.اليوم()
اطبع(today.اسم_اليوم())

// Error Handling
استورد { نتيجة، نجاح، فشل } من "أخطاء"
متغير result = نجاح<عدد، استثناء>(42)

// Networking
استورد { احصل، طلب } من "شبكة"
متغير response = احصل("https://example.com")
```

> **Note**: the `أخطاء` module is not usable yet — its `أخطاء/فهرس.ترقيم`
> declares `صنف خطأ`, and `خطأ` is the boolean literal, not a class name; the
> base exception class is `استثناء`. Tracked by
> [#243](https://github.com/osama1998H/tarqeem/issues/243).

### Module Structure

Each package's `فهرس.ترقيم` is a barrel: it re-exports its submodules with
`صدّر * من "./اساسي"`, so importing the package name reaches every symbol.

```
stdlib/
├── مجموعات/                   # Collections
│   ├── فهرس.ترقيم            # Module re-exports
│   ├── قائمة.ترقيم            # List<T>
│   ├── مجموعة.ترقيم           # Set<T>
│   ├── قاموس.ترقيم            # Map<K,V>
│   ├── طابور.ترقيم            # Queue<T>
│   ├── مكدس.ترقيم             # Stack<T>
│   └── متكرر.ترقيم            # Iterator interface
│
├── رياضيات/                   # Math
│   ├── فهرس.ترقيم
│   ├── اساسي.ترقيم            # Basic math functions (جذر/سقف/قرّب and
│   │                          # friends come straight from the runtime)
│   ├── مثلثات.ترقيم           # Trigonometry
│   ├── عشوائي.ترقيم           # Random numbers
│   └── ثوابت.ترقيم            # Mathematical constants
│
├── نص/                        # String utilities
│   ├── فهرس.ترقيم
│   ├── اساسي.ترقيم            # Basic string functions
│   ├── بناء.ترقيم             # StringBuilder
│   └── تنسيق.ترقيم            # Formatting
│
├── ملفات/                     # File system
│   ├── فهرس.ترقيم
│   ├── ملف.ترقيم              # File operations
│   ├── مسار.ترقيم             # Path handling
│   └── مجلد.ترقيم             # Directory operations
│
├── طرفية/                     # Console I/O
│   ├── فهرس.ترقيم
│   ├── اساسي.ترقيم            # Print and input functions
│   ├── الوان.ترقيم            # ANSI colors
│   └── تنسيق.ترقيم            # Console formatting
│
├── وقت/                       # Date and Time
│   ├── فهرس.ترقيم
│   ├── تاريخ.ترقيم            # Date class with Arabic month/day names
│   └── وقت.ترقيم              # Time, DateTime, and Duration classes
│
├── شبكة/                      # Networking
│   ├── فهرس.ترقيم
│   ├── اتصال.ترقيم            # TCP/UDP connections
│   ├── خادم.ترقيم             # TCP/UDP servers
│   └── http.ترقيم             # HTTP client
│
└── أخطاء/                     # Error handling
    └── فهرس.ترقيم             # Error types, Result<T,E>, Option<T>
```