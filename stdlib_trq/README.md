# Standard Library (stdlib_trq)

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

// معالجة الأخطاء
استورد { نتيجة، نجاح، فشل } من "أخطاء"
متغير نتيجة = نجاح<عدد، خطأ>(42)

// الشبكة
استورد { احصل، طلب_http } من "شبكة"
متغير استجابة = احصل("https://example.com")
```

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
متغير result = نجاح<عدد، خطأ>(42)

// Networking
استورد { احصل، طلب_http } من "شبكة"
متغير response = احصل("https://example.com")
```

### Module Structure

```
stdlib_trq/
├── مجموعات/                   # Collections
│   ├── mod.trq               # Module re-exports
│   ├── قائمة.trq              # List<T>
│   ├── مجموعة.trq             # Set<T>
│   ├── قاموس.trq              # Map<K,V>
│   ├── طابور.trq              # Queue<T>
│   ├── مكدس.trq               # Stack<T>
│   └── متكرر.trq              # Iterator interface
│
├── رياضيات/                   # Math
│   ├── mod.trq
│   ├── اساسي.trq              # Basic math functions
│   ├── مثلثات.trq             # Trigonometry
│   ├── عشوائي.trq             # Random numbers
│   └── ثوابت.trq              # Mathematical constants
│
├── نص/                        # String utilities
│   ├── mod.trq
│   ├── اساسي.trq              # Basic string functions
│   ├── بناء.trq               # StringBuilder
│   └── تنسيق.trq              # Formatting
│
├── ملفات/                     # File system
│   ├── mod.trq
│   ├── ملف.trq                # File operations
│   ├── مسار.trq               # Path handling
│   └── مجلد.trq               # Directory operations
│
├── طرفية/                     # Console I/O
│   ├── mod.trq
│   ├── اساسي.trq              # Print and input functions
│   ├── الوان.trq              # ANSI colors
│   └── تنسيق.trq              # Console formatting
│
├── وقت/                       # Date and Time
│   ├── mod.trq
│   ├── تاريخ.trq              # Date class with Arabic month/day names
│   └── وقت.trq                # Time, DateTime, and Duration classes
│
├── شبكة/                      # Networking
│   ├── mod.trq
│   ├── اتصال.trq              # TCP/UDP connections
│   ├── خادم.trq               # TCP/UDP servers
│   └── http.trq               # HTTP client
│
└── أخطاء/                     # Error handling
    └── mod.trq                # Error types, Result<T,E>, Option<T>
```

### Development Status

Phase 3 Standard Library is **complete**:
- ✅ Milestone 3.0: P1 Bug Fixes
- ✅ Milestone 3.1: Module System
- ✅ Milestone 3.2: Core Collections
- ✅ Milestone 3.3: String Utilities
- ✅ Milestone 3.4: Math Library
- ✅ Milestone 3.5: File System
- ✅ Milestone 3.6: I/O and Console
- ✅ Milestone 3.7: Networking
- ✅ Milestone 3.8: Date and Time
- ✅ Milestone 3.9: Error Handling
