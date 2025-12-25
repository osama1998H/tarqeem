# Tarqeem Built-in Functions & Standard Library

دليل الدوال المدمجة والمكتبة القياسية في ترقيم

This document provides a comprehensive reference for all built-in functions and standard library modules available in Tarqeem.

---

## Overview | نظرة عامة

Tarqeem provides two layers of functionality:

| Layer | Implementation | Availability | Performance |
|-------|----------------|--------------|-------------|
| **Built-in Functions** | C runtime | Always global, no import | Fastest (direct C calls) |
| **Standard Library** | Tarqeem source | Requires `استورد` | Slightly slower |

### Architecture Diagram

```
User Code (.trq)
      ↓
Standard Library (stdlib_trq/*.ترقيم)
      ↓ uses
Built-in Functions (global scope)
      ↓
Runtime (runtime/builtins.c)
      ↓
libc + System Calls
```

---

## Part 1: Built-in Functions | الدوال المدمجة

Built-in functions are hardcoded into the compiler and available globally without any import.

- **Definition**: `src/semantic/scope.rs` (lines 107-573)
- **Runtime**: `runtime/builtins.c`
- **Total**: 67 core functions

---

### I/O Functions | دوال الإدخال والإخراج

| Function | Signature | Description |
|----------|-----------|-------------|
| `اطبع` | `(قيمة: أي)` | Print value without newline |
| `طباعة` | `(قيمة: أي)` | Alias for `اطبع` |
| `اطبع_سطر` | `(قيمة: أي)` | Print value with newline |
| `اطبع_خطأ` | `(قيمة: أي)` | Print to stderr |

**Examples:**
```tarqeem
اطبع("مرحباً ")
اطبع_سطر("بالعالم!")  // مرحباً بالعالم!
اطبع_خطأ("حدث خطأ!")
```

---

### Input Functions | دوال الإدخال

| Function | Signature | Description |
|----------|-----------|-------------|
| `ادخل` | `() -> نص` | Read string from stdin |
| `ادخل_رسالة` | `(رسالة: نص) -> نص` | Read with prompt |
| `ادخل_عدد` | `() -> عدد` | Read integer |
| `ادخل_عشري` | `() -> عدد_عشري` | Read float |

**Examples:**
```tarqeem
متغير اسم = ادخل_رسالة("ما اسمك؟ ")
متغير عمر = ادخل_عدد()
متغير وزن = ادخل_عشري()
```

---

### Type Conversion | تحويل الأنماط

| Function | Signature | Description |
|----------|-----------|-------------|
| `عدد` | `(قيمة: أي) -> عدد` | Convert to integer |
| `عدد_عشري` | `(قيمة: أي) -> عدد_عشري` | Convert to float |
| `نص` | `(قيمة: أي) -> نص` | Convert to string |
| `منطقي` | `(قيمة: أي) -> منطقي` | Convert to boolean |

**Additional Conversion Functions:**

| Function | Signature | Description |
|----------|-----------|-------------|
| `عدد_لنص` | `(ع: عدد) -> نص` | Int to String |
| `عشري_لنص` | `(ع: عدد_عشري) -> نص` | Float to String |
| `منطقي_لنص` | `(م: منطقي) -> نص` | Bool to String |
| `نص_لعدد` | `(ن: نص) -> عدد` | String to Int |
| `نص_لعشري` | `(ن: نص) -> عدد_عشري` | String to Float |

**Examples:**
```tarqeem
متغير س = عدد("42")        // 42
متغير ص = عدد_عشري("3.14") // 3.14
متغير ن = نص(100)          // "100"
متغير م = منطقي(1)         // صحيح
```

---

### Type & Info Functions | دوال النوع والمعلومات

| Function | Signature | Description |
|----------|-----------|-------------|
| `طول` | `(قيمة: أي) -> عدد` | Get length (string/array) |
| `نوع` | `(قيمة: أي) -> نص` | Get type name |

**Examples:**
```tarqeem
متغير ن = طول("مرحباً")  // 6 (characters)
متغير ت = نوع(42)        // "عدد"
```

---

### Math: Basic Operations | الرياضيات: العمليات الأساسية

| Function | Signature | Description |
|----------|-----------|-------------|
| `مطلق` | `(س: أي) -> أي` | Absolute value (generic) |
| `مطلق_عدد` | `(س: عدد) -> عدد` | Absolute value (int) |
| `قوة` | `(أساس: عدد_عشري، أس: عدد_عشري) -> عدد_عشري` | Power (float) |
| `قوة_عدد` | `(أساس: عدد، أس: عدد) -> عدد` | Power (int) |
| `جذر` | `(س: عدد_عشري) -> عدد_عشري` | Square root |
| `جذر_تكعيبي` | `(س: عدد_عشري) -> عدد_عشري` | Cube root |
| `لوغاريتم` | `(س: عدد_عشري) -> عدد_عشري` | Natural logarithm (ln) |
| `لوغ10` | `(س: عدد_عشري) -> عدد_عشري` | Log base 10 |
| `لوغاريتم10` | `(س: عدد_عشري) -> عدد_عشري` | Log base 10 (alias) |
| `لوغ2` | `(س: عدد_عشري) -> عدد_عشري` | Log base 2 |
| `أس` | `(س: عدد_عشري) -> عدد_عشري` | e^x (exponential) |
| `أسي` | `(س: عدد_عشري) -> عدد_عشري` | e^x (alias) |

**Examples:**
```tarqeem
متغير م = مطلق(-5)        // 5
متغير ق = قوة(2.0، 10.0)  // 1024.0
متغير ج = جذر(16.0)       // 4.0
متغير ل = لوغاريتم(2.718) // ~1.0
```

---

### Math: Rounding | الرياضيات: التقريب

| Function | Signature | Description |
|----------|-----------|-------------|
| `أرضية` | `(س: عدد_عشري) -> عدد_عشري` | Floor (round down) |
| `سقف` | `(س: عدد_عشري) -> عدد_عشري` | Ceiling (round up) |
| `قرّب` | `(س: عدد_عشري) -> عدد_عشري` | Round to nearest |
| `تقريب` | `(س: عدد_عشري) -> عدد_عشري` | Round (alias) |

**Examples:**
```tarqeem
متغير أ = أرضية(3.7)  // 3.0
متغير س = سقف(3.2)    // 4.0
متغير ق = قرّب(3.5)   // 4.0
```

---

### Math: Min/Max/Clamp | الرياضيات: الأدنى/الأقصى/الحصر

| Function | Signature | Description |
|----------|-----------|-------------|
| `أقل` | `(أ: أي، ب: أي) -> أي` | Minimum (generic) |
| `أدنى` | `(أ: أي، ب: أي) -> أي` | Minimum (alias) |
| `أقل_عدد` | `(أ: عدد، ب: عدد) -> عدد` | Minimum (int) |
| `أكبر` | `(أ: أي، ب: أي) -> أي` | Maximum (generic) |
| `أقصى` | `(أ: أي، ب: أي) -> أي` | Maximum (alias) |
| `أكبر_عدد` | `(أ: عدد، ب: عدد) -> عدد` | Maximum (int) |
| `حصر` | `(قيمة: أي، أدنى: أي، أقصى: أي) -> أي` | Clamp (generic) |
| `حصر_عدد` | `(قيمة: عدد، أدنى: عدد، أقصى: عدد) -> عدد` | Clamp (int) |

**Examples:**
```tarqeem
متغير أ = أقل(5، 3)           // 3
متغير ب = أكبر(5، 3)          // 5
متغير ج = حصر(15، 0، 10)      // 10 (clamped to max)
متغير د = حصر_عدد(-5، 0، 10)  // 0 (clamped to min)
```

---

### Math: Number Theory | الرياضيات: نظرية الأعداد

| Function | Signature | Description |
|----------|-----------|-------------|
| `علامة` | `(س: عدد) -> عدد` | Sign (-1, 0, or 1) |
| `باقي` | `(أ: عدد، ب: عدد) -> عدد` | Modulo |
| `قاسم_مشترك` | `(أ: عدد، ب: عدد) -> عدد` | GCD (Greatest Common Divisor) |
| `مضاعف_مشترك` | `(أ: عدد، ب: عدد) -> عدد` | LCM (Least Common Multiple) |
| `عاملي` | `(س: عدد) -> عدد` | Factorial (n!) |

**Examples:**
```tarqeem
متغير ع = علامة(-42)           // -1
متغير ب = باقي(17، 5)          // 2
متغير ق = قاسم_مشترك(48، 18)   // 6
متغير م = مضاعف_مشترك(4، 6)    // 12
متغير ف = عاملي(5)             // 120
```

---

### Trigonometry | المثلثات

#### Basic Trigonometric Functions

| Function | Alias | Signature | Description |
|----------|-------|-----------|-------------|
| `جا` | `جيب` | `(س: عدد_عشري) -> عدد_عشري` | Sine |
| `جتا` | `جيب_التمام` | `(س: عدد_عشري) -> عدد_عشري` | Cosine |
| `ظا` | `ظل` | `(س: عدد_عشري) -> عدد_عشري` | Tangent |
| `ظتا` | `ظل_التمام` | `(س: عدد_عشري) -> عدد_عشري` | Cotangent |
| `قا` | `قاطع` | `(س: عدد_عشري) -> عدد_عشري` | Secant |
| `قتا` | `قاطع_التمام` | `(س: عدد_عشري) -> عدد_عشري` | Cosecant |

#### Inverse Trigonometric Functions

| Function | Alias | Signature | Description |
|----------|-------|-----------|-------------|
| `جا_عكسي` | `جيب_عكسي` | `(س: عدد_عشري) -> عدد_عشري` | Arcsine |
| `جتا_عكسي` | `جيب_تمام_عكسي` | `(س: عدد_عشري) -> عدد_عشري` | Arccosine |
| `ظا_عكسي` | - | `(س: عدد_عشري) -> عدد_عشري` | Arctangent |
| `ظا_عكسي2` | - | `(ص: عدد_عشري، س: عدد_عشري) -> عدد_عشري` | Atan2 (angle from coordinates) |

#### Hyperbolic Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `جا_زائدي` | `(س: عدد_عشري) -> عدد_عشري` | Hyperbolic sine (sinh) |
| `جتا_زائدي` | `(س: عدد_عشري) -> عدد_عشري` | Hyperbolic cosine (cosh) |
| `ظا_زائدي` | `(س: عدد_عشري) -> عدد_عشري` | Hyperbolic tangent (tanh) |

#### Angle Conversion

| Function | Signature | Description |
|----------|-----------|-------------|
| `الى_راديان` | `(درجات: عدد_عشري) -> عدد_عشري` | Degrees to radians |
| `الى_درجات` | `(راديان: عدد_عشري) -> عدد_عشري` | Radians to degrees |

**Examples:**
```tarqeem
متغير زاوية = 45.0
متغير راد = الى_راديان(زاوية)
متغير جيب = جا(راد)           // ~0.707
متغير جيب_تمام = جتا(راد)     // ~0.707
```

---

### Random Numbers | الأعداد العشوائية

| Function | Alias | Signature | Description |
|----------|-------|-----------|-------------|
| `بذرة_عشوائية` | `بذرة_عشوائي` | `(بذرة: عدد)` | Seed the RNG |
| `عشوائي` | `عشوائي_عدد` | `() -> عدد` | Random integer |
| `عشوائي_بين` | `عشوائي_عدد_بين` | `(أدنى: عدد، أقصى: عدد) -> عدد` | Random int in range [min, max] |
| `عشوائي_عشري` | - | `() -> عدد_عشري` | Random float in [0, 1) |
| `عشوائي_عشري_بين` | - | `(أدنى: عدد_عشري، أقصى: عدد_عشري) -> عدد_عشري` | Random float in range |
| `عشوائي_منطقي` | - | `() -> منطقي` | Random boolean |

**Examples:**
```tarqeem
بذرة_عشوائية(42)                    // Set seed for reproducibility
متغير ع = عشوائي()                  // Random integer
متغير ن = عشوائي_بين(1، 100)        // Random int 1-100
متغير ش = عشوائي_عشري()             // Random 0.0-1.0
متغير م = عشوائي_منطقي()            // صحيح or خطأ
```

---

### String Operations | عمليات النصوص

#### Substring & Character Access

| Function | Signature | Description |
|----------|-----------|-------------|
| `قص_نص` | `(ن: نص، بداية: عدد، عدد: عدد) -> نص` | Substring by bytes |
| `قص_حروف` | `(ن: نص، بداية: عدد، عدد: عدد) -> نص` | Substring by characters |
| `حرف_في` | `(ن: نص، فهرس: عدد) -> نص` | Character at index |

#### Search & Contains

| Function | Signature | Description |
|----------|-----------|-------------|
| `يحتوي` | `(ن: نص، جزء: نص) -> منطقي` | Contains substring |
| `يبدأ_بـ` | `(ن: نص، بادئة: نص) -> منطقي` | Starts with prefix |
| `ينتهي_بـ` | `(ن: نص، لاحقة: نص) -> منطقي` | Ends with suffix |
| `موضع` | `(ن: نص، جزء: نص) -> عدد` | Index of (first occurrence) |
| `موضع_اخير` | `(ن: نص، جزء: نص) -> عدد` | Last index of |
| `عدد_مرات` | `(ن: نص، جزء: نص) -> عدد` | Count occurrences |

#### Case Conversion

| Function | Signature | Description |
|----------|-----------|-------------|
| `كبير` | `(ن: نص) -> نص` | Uppercase |
| `صغير` | `(ن: نص) -> نص` | Lowercase |
| `عنوان` | `(ن: نص) -> نص` | Title case |

#### Manipulation

| Function | Signature | Description |
|----------|-----------|-------------|
| `اعكس_نص` | `(ن: نص) -> نص` | Reverse string |
| `ازل_فراغات` | `(ن: نص) -> نص` | Trim whitespace (both ends) |
| `ازل_فراغات_يسار` | `(ن: نص) -> نص` | Trim left |
| `ازل_فراغات_يمين` | `(ن: نص) -> نص` | Trim right |
| `قسّم` | `(ن: نص، فاصل: نص) -> مصفوفة<نص>` | Split into array |
| `ادمج` | `(مصفوفة: مصفوفة<نص>، فاصل: نص) -> نص` | Join array to string |
| `استبدل` | `(ن: نص، قديم: نص، جديد: نص) -> نص` | Replace first occurrence |
| `استبدل_كل` | `(ن: نص، قديم: نص، جديد: نص) -> نص` | Replace all occurrences |
| `كرر_نص` | `(ن: نص، مرات: عدد) -> نص` | Repeat n times |
| `كرر` | `(ن: نص، مرات: عدد) -> نص` | Repeat (alias) |
| `احشو_يسار` | `(ن: نص، عرض: عدد، حشو: نص) -> نص` | Left pad |
| `احشو_يمين` | `(ن: نص، عرض: عدد، حشو: نص) -> نص` | Right pad |

#### Length & Validation

| Function | Signature | Description |
|----------|-----------|-------------|
| `طول_نص` | `(ن: نص) -> عدد` | Length in bytes |
| `طول_حروف` | `(ن: نص) -> عدد` | Length in characters |
| `رقمي` | `(ن: نص) -> منطقي` | Is numeric |
| `حروف_فقط` | `(ن: نص) -> منطقي` | Is alphabetic |
| `عربي` | `(ن: نص) -> منطقي` | Is Arabic text |

#### Comparison

| Function | Signature | Description |
|----------|-----------|-------------|
| `قارن_نص` | `(أ: نص، ب: نص) -> عدد` | Compare (-1, 0, 1) |
| `نصوص_متساوية` | `(أ: نص، ب: نص) -> منطقي` | Equality check |

**Examples:**
```tarqeem
متغير ن = "مرحباً بالعالم"
اطبع_سطر(يحتوي(ن، "بال"))       // صحيح
اطبع_سطر(طول_حروف(ن))          // 13
اطبع_سطر(قص_حروف(ن، 0، 6))     // "مرحباً"

متغير أجزاء = قسّم("أ،ب،ج"، "،")  // ["أ"، "ب"، "ج"]
متغير مدمج = ادمج(أجزاء، "-")    // "أ-ب-ج"

اطبع_سطر(كرر_نص("*"، 10))       // "**********"
اطبع_سطر(استبدل_كل(ن، "ا"، "أ")) // Replace all
```

---

### Array Operations | عمليات المصفوفات

| Function | Signature | Description |
|----------|-----------|-------------|
| `طول_مصفوفة` | `(م: مصفوفة<أي>) -> عدد` | Array length |
| `الحق` | `(م: مصفوفة<ن>، عنصر: ن)` | Append element |

**Examples:**
```tarqeem
متغير أرقام = [1، 2، 3]
اطبع_سطر(طول_مصفوفة(أرقام))  // 3
الحق(أرقام، 4)
اطبع_سطر(طول_مصفوفة(أرقام))  // 4
```

---

### File Operations | عمليات الملفات

#### File Checks

| Function | Signature | Description |
|----------|-----------|-------------|
| `ملف_موجود` | `(مسار: نص) -> منطقي` | File exists |
| `هل_ملف` | `(مسار: نص) -> منطقي` | Is a file |
| `هل_مجلد` | `(مسار: نص) -> منطقي` | Is a directory |
| `حجم_ملف` | `(مسار: نص) -> عدد` | File size in bytes |

#### Read/Write

| Function | Signature | Description |
|----------|-----------|-------------|
| `اقرأ_ملف` | `(مسار: نص) -> نص` | Read file contents |
| `اكتب_ملف` | `(مسار: نص، محتوى: نص)` | Write file (overwrite) |
| `الحق_ملف` | `(مسار: نص، محتوى: نص)` | Append to file |

#### File Management

| Function | Signature | Description |
|----------|-----------|-------------|
| `احذف_ملف` | `(مسار: نص)` | Delete file |
| `انسخ_ملف` | `(من: نص، الى: نص)` | Copy file |
| `انقل_ملف` | `(من: نص، الى: نص)` | Move/rename file |

**Examples:**
```tarqeem
إذا (ملف_موجود("بيانات.txt")) {
    متغير محتوى = اقرأ_ملف("بيانات.txt")
    اطبع_سطر(محتوى)
}

اكتب_ملف("ناتج.txt"، "مرحباً!")
الحق_ملف("ناتج.txt"، "\nسطر جديد")
```

---

### Directory Operations | عمليات المجلدات

| Function | Signature | Description |
|----------|-----------|-------------|
| `انشئ_مجلد` | `(مسار: نص)` | Create directory |
| `قائمة_مجلد` | `(مسار: نص) -> مصفوفة<نص>` | List directory contents |
| `احذف_مجلد` | `(مسار: نص)` | Delete directory |
| `مجلد_حالي` | `() -> نص` | Current working directory |
| `مجلد_مستخدم` | `() -> نص` | User home directory |
| `مجلد_مؤقت` | `() -> نص` | Temp directory |

**Examples:**
```tarqeem
متغير حالي = مجلد_حالي()
اطبع_سطر("المجلد الحالي: " + حالي)

متغير ملفات = قائمة_مجلد(".")
لكل ملف في ملفات {
    اطبع_سطر(ملف)
}

انشئ_مجلد("مجلد_جديد")
```

---

### Path Operations | عمليات المسارات

| Function | Signature | Description |
|----------|-----------|-------------|
| `ادمج_مسار` | `(أ: نص، ب: نص) -> نص` | Join paths |
| `مسار_اب` | `(مسار: نص) -> نص` | Parent directory |
| `اسم_ملف` | `(مسار: نص) -> نص` | Filename from path |
| `امتداد_ملف` | `(مسار: نص) -> نص` | File extension |
| `فاصل_مسار` | `() -> نص` | Path separator (`/` or `\`) |

**Examples:**
```tarqeem
متغير مسار = ادمج_مسار("/home"، "user")  // "/home/user"
متغير اسم = اسم_ملف("/home/user/file.txt")  // "file.txt"
متغير امتداد = امتداد_ملف("file.txt")        // "txt"
متغير أب = مسار_اب("/home/user/file.txt")    // "/home/user"
```

---

### Time Functions | دوال الوقت

| Function | Signature | Description |
|----------|-----------|-------------|
| `نم` | `(مللي_ثانية: عدد)` | Sleep for milliseconds |
| `وقت_الآن` | `() -> عدد` | Current time (ms since epoch) |
| `وقت_أداء` | `() -> عدد` | High-resolution timer (ns) |

**Examples:**
```tarqeem
متغير بداية = وقت_أداء()
// ... عملية ما
متغير نهاية = وقت_أداء()
اطبع_سطر("الوقت: " + نص((نهاية - بداية) / 1000000) + " مللي ثانية")

نم(1000)  // انتظر ثانية واحدة
```

---

### Utility Functions | دوال مساعدة

| Function | Signature | Description |
|----------|-----------|-------------|
| `توقف` | `(رسالة: نص)` | Panic/abort with message |
| `تأكد` | `(شرط: منطقي)` | Assert condition |
| `تأكد_رسالة` | `(شرط: منطقي، رسالة: نص)` | Assert with message |

**Examples:**
```tarqeem
تأكد(س > 0)
تأكد_رسالة(س > 0، "يجب أن يكون س موجباً")

إذا (خطأ_حرج) {
    توقف("حدث خطأ حرج!")
}
```

---

## Part 2: Standard Library | المكتبة القياسية

The standard library is written in Tarqeem and provides higher-level abstractions. It requires importing before use.

- **Location**: `stdlib_trq/`
- **Total**: ~7,710 lines across 8 modules

### Import Syntax | صياغة الاستيراد

```tarqeem
// Import specific items
استورد { قائمة، قاموس } من "مجموعات"

// Import all with alias
استورد * كـ رياضيات من "رياضيات"

// Import default
استورد وقت من "وقت"
```

---

### Module: مجموعات (Collections)

**Location**: `stdlib_trq/مجموعات/`
**Lines**: ~1,120

Provides data structures for organizing and manipulating collections of data.

#### قائمة (List)

A dynamic array with generic type support.

```tarqeem
استورد { قائمة } من "مجموعات"

متغير أرقام = جديد قائمة<عدد>()
أرقام.أضف(1)
أرقام.أضف(2)
أرقام.أضف_في(1، 10)  // Insert at index 1

اطبع_سطر(أرقام.طول())      // 3
اطبع_سطر(أرقام.احصل(0))    // 1
اطبع_سطر(أرقام.فارغة())    // خطأ

أرقام.احذف_في(1)
أرقام.مسح()
```

**Methods:**
| Method | Description |
|--------|-------------|
| `أضف(عنصر)` | Append element |
| `أضف_في(فهرس، عنصر)` | Insert at index |
| `احصل(فهرس)` | Get element at index |
| `عيّن(فهرس، قيمة)` | Set element at index |
| `احذف_في(فهرس)` | Remove at index |
| `طول()` | Get length |
| `فارغة()` | Check if empty |
| `مسح()` | Clear all elements |
| `يحتوي(عنصر)` | Contains element |
| `فهرس(عنصر)` | Index of element |

#### قاموس (Map/Dictionary)

A key-value dictionary with generic types.

```tarqeem
استورد { قاموس } من "مجموعات"

متغير أعمار = جديد قاموس<نص، عدد>()
أعمار.عيّن("أحمد"، 25)
أعمار.عيّن("فاطمة"، 30)

اطبع_سطر(أعمار.احصل("أحمد"))     // 25
اطبع_سطر(أعمار.يحتوي("محمد"))    // خطأ
اطبع_سطر(أعمار.طول())            // 2

أعمار.احذف("أحمد")
```

**Methods:**
| Method | Description |
|--------|-------------|
| `عيّن(مفتاح، قيمة)` | Set key-value pair |
| `احصل(مفتاح)` | Get value by key |
| `يحتوي(مفتاح)` | Check if key exists |
| `احذف(مفتاح)` | Remove by key |
| `طول()` | Get size |
| `فارغ()` | Check if empty |
| `مسح()` | Clear all |
| `مفاتيح()` | Get all keys |
| `قيم()` | Get all values |

#### مجموعة (Set)

A collection of unique elements.

```tarqeem
استورد { مجموعة } من "مجموعات"

متغير فريدة = جديد مجموعة<عدد>()
فريدة.أضف(1)
فريدة.أضف(2)
فريدة.أضف(1)  // لا يُضاف (موجود مسبقاً)

اطبع_سطر(فريدة.طول())  // 2
```

#### طابور (Queue) & مكدس (Stack)

```tarqeem
استورد { طابور، مكدس } من "مجموعات"

// Queue (FIFO)
متغير ط = جديد طابور<نص>()
ط.أدخل("أول")
ط.أدخل("ثاني")
اطبع_سطر(ط.أخرج())  // "أول"

// Stack (LIFO)
متغير م = جديد مكدس<عدد>()
م.ادفع(1)
م.ادفع(2)
اطبع_سطر(م.أخرج())  // 2
```

---

### Module: رياضيات (Math)

**Location**: `stdlib_trq/رياضيات/`
**Lines**: ~666

Mathematical constants and utility functions.

```tarqeem
استورد { ط، هـ، ذهبي } من "رياضيات"

اطبع_سطر(ط)      // 3.141592653589793 (π)
اطبع_سطر(هـ)     // 2.718281828459045 (e)
اطبع_سطر(ذهبي)   // 1.618033988749895 (φ)
```

**Constants:**
| Constant | Value | Description |
|----------|-------|-------------|
| `ط` | 3.14159... | Pi (π) |
| `هـ` | 2.71828... | Euler's number (e) |
| `ذهبي` | 1.61803... | Golden ratio (φ) |

---

### Module: نص (String Utilities)

**Location**: `stdlib_trq/نص/`
**Lines**: ~567

String building and formatting utilities.

```tarqeem
استورد { بناء_نص } من "نص"

متغير ب = جديد بناء_نص()
ب.أضف("مرحباً")
ب.أضف(" ")
ب.أضف("بالعالم")
اطبع_سطر(ب.ابني())  // "مرحباً بالعالم"
```

---

### Module: ملفات (Files)

**Location**: `stdlib_trq/ملفات/`
**Lines**: ~530

File and path manipulation wrappers.

```tarqeem
استورد { ملف، مسار } من "ملفات"

متغير م = جديد ملف("بيانات.txt")
إذا (م.موجود()) {
    متغير محتوى = م.اقرأ()
    اطبع_سطر(محتوى)
}

متغير س = جديد مسار("/home/user/file.txt")
اطبع_سطر(س.اسم())      // "file.txt"
اطبع_سطر(س.امتداد())   // "txt"
اطبع_سطر(س.أب())       // "/home/user"
```

---

### Module: طرفية (Console/Terminal)

**Location**: `stdlib_trq/طرفية/`
**Lines**: ~967

Terminal output with colors and formatting.

```tarqeem
استورد { لون، تنسيق } من "طرفية"

اطبع_سطر(لون.أحمر("خطأ!"))
اطبع_سطر(لون.أخضر("نجاح!"))
اطبع_سطر(لون.أصفر("تحذير!"))

اطبع_سطر(تنسيق.عريض("نص عريض"))
اطبع_سطر(تنسيق.مائل("نص مائل"))
```

**Available Colors:**
- `أحمر`, `أخضر`, `أزرق`, `أصفر`, `بنفسجي`, `سماوي`, `أبيض`, `أسود`

---

### Module: وقت (Time)

**Location**: `stdlib_trq/وقت/`
**Lines**: ~1,406

Date, time, and duration handling.

```tarqeem
استورد { تاريخ، وقت، مدة } من "وقت"

// Current date
متغير اليوم = تاريخ.الآن()
اطبع_سطر(اليوم.سنة())
اطبع_سطر(اليوم.شهر())
اطبع_سطر(اليوم.يوم())

// Create specific date
متغير تاريخ_محدد = جديد تاريخ(2024، 1، 15)

// Duration
متغير م = جديد مدة.ثواني(90)
اطبع_سطر(م.دقائق())  // 1
اطبع_سطر(م.ثواني())  // 30
```

---

### Module: شبكة (Network)

**Location**: `stdlib_trq/شبكة/`
**Lines**: ~1,204

TCP, UDP, and HTTP networking.

```tarqeem
استورد { طلب_HTTP } من "شبكة"

متغير استجابة = انتظر طلب_HTTP.احصل("https://api.example.com/data")
إذا (استجابة.ناجح()) {
    اطبع_سطر(استجابة.نص())
}
```

---

### Module: أخطاء (Error Handling)

**Location**: `stdlib_trq/أخطاء/`
**Lines**: ~710

Result and Option types for safe error handling.

```tarqeem
استورد { نتيجة، نجاح، فشل، اختياري، بعض، لا_شيء } من "أخطاء"

// Result type
دالة قسمة(أ: عدد، ب: عدد) -> نتيجة<عدد، نص> {
    إذا (ب == 0) {
        أرجع فشل("لا يمكن القسمة على صفر")
    }
    أرجع نجاح(أ / ب)
}

متغير ن = قسمة(10، 2)
إذا (ن.نجح()) {
    اطبع_سطر("النتيجة: " + نص(ن.احصل()))
} وإلا {
    اطبع_سطر("خطأ: " + ن.خطأ())
}

// Option type
دالة ابحث(قائمة: مصفوفة<عدد>، قيمة: عدد) -> اختياري<عدد> {
    لكل (متغير ف = 0؛ ف < طول(قائمة)؛ ف++) {
        إذا (قائمة[ف] == قيمة) {
            أرجع بعض(ف)
        }
    }
    أرجع لا_شيء()
}
```

**نتيجة (Result) Methods:**
| Method | Description |
|--------|-------------|
| `نجح()` | Returns true if success |
| `فشل()` | Returns true if failure |
| `احصل()` | Get success value |
| `خطأ()` | Get error value |
| `أو(افتراضي)` | Get value or default |

**اختياري (Option) Methods:**
| Method | Description |
|--------|-------------|
| `موجود()` | Has value |
| `فارغ()` | No value |
| `احصل()` | Get value |
| `أو(افتراضي)` | Get value or default |

---

## Summary Comparison | ملخص المقارنة

| Feature | Built-in (مدمج) | Standard Library (قياسي) |
|---------|-----------------|-------------------------|
| **Count** | 67 functions | 100+ across 8 modules |
| **Import** | Not needed | Required |
| **Implementation** | C/Rust | Tarqeem |
| **Modifiable** | No | Yes |
| **Performance** | Fastest | Good |
| **Abstraction** | Low-level | High-level |
| **Examples** | `اطبع()`, `جذر()` | `قائمة<>`, `نتيجة<>` |

---

## Quick Reference Card | بطاقة مرجعية سريعة

### Most Used Built-ins

```tarqeem
// I/O
اطبع_سطر(قيمة)
متغير س = ادخل_رسالة("أدخل: ")

// Types
متغير ع = عدد("42")
متغير ن = نص(3.14)

// Math
جذر(16.0)           // 4.0
قوة(2.0، 8.0)       // 256.0
عشوائي_بين(1، 100)  // Random 1-100

// Strings
يحتوي(نص، "بحث")
قسّم(نص، "،")
استبدل_كل(نص، "أ"، "ب")

// Files
اقرأ_ملف("ملف.txt")
اكتب_ملف("ملف.txt"، "محتوى")
```

### Most Used Standard Library

```tarqeem
// Collections
استورد { قائمة، قاموس } من "مجموعات"
متغير ق = جديد قائمة<عدد>()
متغير د = جديد قاموس<نص، عدد>()

// Error Handling
استورد { نتيجة، نجاح، فشل } من "أخطاء"

// Time
استورد { تاريخ } من "وقت"
متغير اليوم = تاريخ.الآن()

// Console Colors
استورد { لون } من "طرفية"
اطبع_سطر(لون.أخضر("نجاح!"))
```

---

*Last updated: 2024*
*Tarqeem Version: 1.1.x*
