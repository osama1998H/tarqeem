# Tarqeem Language Specification

**Version 1.0.0**
**مواصفات لغة ترقيم**

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Language Purpose](#2-language-purpose)
3. [Design Philosophy](#3-design-philosophy)
4. [Lexical Structure](#4-lexical-structure)
5. [Type System](#5-type-system)
6. [Expressions](#6-expressions)
7. [Statements](#7-statements)
8. [Functions](#8-functions)
9. [Object-Oriented Programming](#9-object-oriented-programming)
10. [Modules and Imports](#10-modules-and-imports)
11. [Error Handling](#11-error-handling)
12. [Concurrency](#12-concurrency)
13. [Memory Model](#13-memory-model)
14. [Standard Library](#14-standard-library)
15. [Formal Grammar](#15-formal-grammar)
16. [Appendix: Keyword Reference](#appendix-a-keyword-reference)

---

## 1. Introduction

Tarqeem (ترقيم) is a compiled, statically-typed, general-purpose programming language with native Arabic syntax support. The name "ترقيم" means "numbering" or "notation" in Arabic, reflecting the language's purpose of making programming accessible to Arabic speakers.

### 1.1 Scope

This specification defines:
- The lexical structure and syntax of Tarqeem programs
- The type system and type checking rules
- The semantics of expressions, statements, and declarations
- The object-oriented programming model
- The module system and imports
- Error handling mechanisms
- The memory model and lifetime semantics

### 1.2 Notation Conventions

- `keyword` - Language keywords shown in monospace
- `<name>` - Placeholder for user-defined names
- `[optional]` - Optional elements in brackets
- `...` - Repetition of preceding element
- `|` - Alternative choices

---

## 2. Language Purpose

### 2.1 Problem Statement

Programming has historically been dominated by English-based languages, creating barriers for Arabic speakers:
- Learning programming requires learning English terminology
- Code cannot express domain concepts in the native language
- Right-to-left (RTL) text handling is an afterthought

### 2.2 Solution

Tarqeem addresses these challenges by providing:

1. **Native Arabic Syntax**: All keywords have Arabic primary forms
2. **Bilingual Support**: English aliases for all keywords enable collaboration
3. **Unicode-First Design**: Full UTF-8 support with proper bidirectional text handling
4. **Compiled Performance**: Native machine code via LLVM backend
5. **Modern Features**: Static typing, generics, async/await, pattern matching

### 2.3 Target Audience

- Arabic-speaking developers learning programming
- Development teams in Arabic-speaking regions
- Educational institutions teaching programming in Arabic
- Projects requiring Arabic domain terminology in code

---

## 3. Design Philosophy

### 3.1 Core Principles

| Principle | Description |
|-----------|-------------|
| **Arabic-First** | Arabic is the primary language; English is an alias |
| **Bilingual** | All constructs work in both Arabic and English |
| **Safe** | Strong static typing prevents runtime errors |
| **Expressive** | Modern language features for concise code |
| **Fast** | Compiled to native code for production performance |

### 3.2 Design Influences

Tarqeem draws inspiration from:

- **Python**: Clean, readable syntax
- **TypeScript**: Static typing with inference
- **Rust**: Memory safety concepts, pattern matching
- **JavaScript**: Modern async/await, arrow functions

### 3.3 Trade-offs

| Choice | Trade-off |
|--------|-----------|
| Static typing | More verbose but catches errors at compile time |
| Reference counting | Simpler than GC, deterministic, but cyclic references need care |
| Compiled | Slower iteration but faster runtime |
| Explicit visibility | More typing but clearer intent |

---

## 4. Lexical Structure

### 4.1 Source Encoding

- Source files MUST be encoded in UTF-8
- Identifiers undergo NFC (Canonical Decomposition, followed by Canonical Composition) normalization before comparison
- File extensions: `.trq` or `.ترقيم`

### 4.2 Identifiers

Identifiers follow Unicode identifier rules:

```
identifier := identifier_start identifier_continue*
identifier_start := <Unicode Letter> | '_'
identifier_continue := <Unicode Letter> | <Unicode Digit> | '_'
```

Valid identifiers:
```tarqeem
متغير       // Arabic identifier
userName    // English identifier
مستخدم_1    // Mixed with underscore
_private    // Starting with underscore
```

### 4.3 Keywords

All keywords have Arabic and English forms. The Arabic form is primary.

#### Variable Keywords
| Arabic | English | Description |
|--------|---------|-------------|
| `متغير` | `let`, `var` | Mutable variable |
| `ثابت` | `const` | Immutable constant |

#### Function Keywords
| Arabic | English | Description |
|--------|---------|-------------|
| `دالة` | `function`, `fn` | Function declaration |
| `أرجع` / `ارجع` | `return` | Return statement |
| `غير_متزامن` | `async` | Async function |
| `انتظر` | `await` | Await expression |

#### Control Flow Keywords
| Arabic | English | Description |
|--------|---------|-------------|
| `إذا` / `اذا` | `if` | If condition |
| `وإلا` / `والا` | `else` | Else branch |
| `طالما` | `while` | While loop |
| `لكل` | `for` | For loop |
| `في` | `in` | In operator |
| `افعل` | `do` | Do-while loop |
| `أوقف` / `اوقف` | `break` | Break loop |
| `استمر` | `continue` | Continue loop |
| `تطابق` | `match`, `switch` | Pattern matching |
| `حالة` | `case` | Match case |
| `غير_ذلك` | `default` | Default case |

#### OOP Keywords
| Arabic | English | Description |
|--------|---------|-------------|
| `صنف` | `class` | Class declaration |
| `واجهة` | `interface` | Interface declaration |
| `يرث` | `extends` | Inheritance |
| `يطبق` | `implements` | Interface implementation |
| `عام` | `public` | Public visibility |
| `خاص` | `private` | Private visibility |
| `محمي` | `protected` | Protected visibility |
| `ثابت_صنف` | `static` | Static member |
| `منشئ` | `constructor` | Constructor |
| `هذا` | `this` | Self reference |
| `أساس` / `اساس` | `super` | Parent reference |
| `جديد` | `new` | Object instantiation |

#### Error Handling Keywords
| Arabic | English | Description |
|--------|---------|-------------|
| `حاول` | `try` | Try block |
| `التقط` | `catch` | Catch block |
| `أخيراً` / `اخيرا` | `finally` | Finally block |
| `ارمِ` / `ارم` | `throw` | Throw exception |

#### Module Keywords
| Arabic | English | Description |
|--------|---------|-------------|
| `استورد` | `import` | Import module |
| `صدّر` / `صدر` | `export` | Export declaration |
| `من` | `from` | From specifier |
| `كـ` / `ك` | `as` | Alias specifier |

#### Literal Keywords
| Arabic | English | Description |
|--------|---------|-------------|
| `صحيح` | `true` | Boolean true |
| `خطأ` / `خطا` | `false` | Boolean false |
| `عدم` | `null`, `none` | Null value |

#### Logical Operators (Word Form)
| Arabic | English | Description |
|--------|---------|-------------|
| `و` | `&&` | Logical AND |
| `أو` / `او` | `\|\|` | Logical OR |
| `ليس` | `not`, `!` | Logical NOT |

### 4.4 Literals

#### Integer Literals
```tarqeem
42        // Decimal
0x2A      // Hexadecimal
0b101010  // Binary
0o52      // Octal
٤٢        // Arabic-Indic numerals
```

#### Float Literals
```tarqeem
3.14159
2.5e10
1.0E-5
٣.١٤      // Arabic-Indic numerals
```

#### String Literals
```tarqeem
"مرحبا"           // Double quotes
'hello'           // Single quotes
«مرحبا»           // Arabic quotation marks
"سطر أول\nسطر ثاني"  // Escape sequences
```

Escape sequences:
| Sequence | Meaning |
|----------|---------|
| `\n` | Newline |
| `\t` | Tab |
| `\r` | Carriage return |
| `\\` | Backslash |
| `\"` | Double quote |
| `\'` | Single quote |

#### Boolean Literals
```tarqeem
صحيح      // true
خطأ       // false
true      // English alias
false     // English alias
```

#### Null Literal
```tarqeem
عدم       // null
null      // English alias
```

#### Array Literals
```tarqeem
[1, 2, 3]
[1، 2، 3]       // Arabic comma
["أ"، "ب"، "ج"]  // Strings
[]               // Empty array
```

#### Object Literals
```tarqeem
{ اسم: "أحمد"، عمر: 30 }
{ name: "Ahmed", age: 30 }
```

### 4.5 Operators

#### Arithmetic Operators
| Operator | Description |
|----------|-------------|
| `+` | Addition |
| `-` | Subtraction |
| `*` | Multiplication |
| `/` | Division |
| `%` | Modulus |
| `**` | Exponentiation |

#### Comparison Operators
| Operator | Description |
|----------|-------------|
| `==` | Equal |
| `!=` | Not equal |
| `<` | Less than |
| `<=` | Less than or equal |
| `>` | Greater than |
| `>=` | Greater than or equal |

#### Logical Operators
| Operator | Arabic Form | Description |
|----------|-------------|-------------|
| `&&` | `و` | Logical AND |
| `\|\|` | `أو` | Logical OR |
| `!` | `ليس` | Logical NOT |

#### Assignment Operators
| Operator | Description |
|----------|-------------|
| `=` | Assignment |
| `+=` | Add and assign |
| `-=` | Subtract and assign |
| `*=` | Multiply and assign |
| `/=` | Divide and assign |
| `%=` | Modulus and assign |

#### Increment/Decrement
| Operator | Description |
|----------|-------------|
| `++` | Increment |
| `--` | Decrement |

#### Other Operators
| Operator | Description |
|----------|-------------|
| `?:` | Ternary conditional |
| `?.` | Optional chaining (planned) |
| `??` | Nullish coalescing (planned) |

### 4.6 Punctuation

Both ASCII and Arabic punctuation are accepted:

| ASCII | Arabic | Usage |
|-------|--------|-------|
| `,` | `،` | Separator |
| `;` | `؛` | Statement terminator |
| `(` `)` | | Grouping |
| `{` `}` | | Block |
| `[` `]` | | Index/Array |
| `:` | | Type annotation |
| `->` | | Return type |
| `=>` | | Arrow function |
| `.` | | Member access |

### 4.7 Comments

```tarqeem
// Single-line comment (تعليق سطري)

/* Multi-line
   comment */

/// Documentation comment (للتوثيق)
/// Returns the sum of two numbers
دالة جمع(أ: عدد، ب: عدد) -> عدد { ... }

/** Block documentation comment */
```

### 4.8 Operator Precedence

From lowest to highest precedence:

| Level | Operators | Associativity |
|-------|-----------|---------------|
| 1 | `=`, `+=`, `-=`, `*=`, `/=`, `%=` | Right |
| 2 | `?:` (ternary) | Right |
| 3 | `\|\|`, `أو` | Left |
| 4 | `&&`, `و` | Left |
| 5 | `==`, `!=` | Left |
| 6 | `<`, `<=`, `>`, `>=` | Left |
| 7 | `+`, `-` | Left |
| 8 | `*`, `/`, `%` | Left |
| 9 | `**` | Right |
| 10 | `!`, `-` (unary), `++`, `--` | Right |
| 11 | `()`, `[]`, `.` | Left |

---

## 5. Type System

### 5.1 Overview

Tarqeem uses a **strong, static type system** with type inference. Types are checked at compile time, preventing many runtime errors.

### 5.2 Primitive Types

| Arabic | English | Description | Size |
|--------|---------|-------------|------|
| `عدد` | `int` | Signed integer | 64-bit |
| `عدد_عشري` | `float` | Floating point | 64-bit |
| `نص` | `string` | UTF-8 string | Variable |
| `منطقي` | `bool` | Boolean | 1-bit |
| `فراغ` | `void` | No value | N/A |
| `عدم` | `null` | Null value | Pointer |

### 5.3 Composite Types

#### Array Type
```tarqeem
مصفوفة<عدد>        // Array of integers
مصفوفة<نص>         // Array of strings
array<int>         // English form
```

#### Map Type
```tarqeem
قاموس<نص، عدد>     // Map from string to int
map<string, int>   // English form
```

#### Optional Type
```tarqeem
عدد?               // Optional integer (may be null)
نص?                // Optional string
```

#### Function Type
```tarqeem
(عدد، عدد) -> عدد   // Function taking two ints, returning int
() -> فراغ          // Function with no params, no return
```

### 5.4 User-Defined Types

#### Class Types
```tarqeem
صنف شخص { ... }
متغير ش: شخص       // Variable of type شخص
```

#### Interface Types
```tarqeem
واجهة قابل_للطباعة { ... }
```

#### Generic Types
```tarqeem
صنف قائمة<ن> { ... }
متغير أرقام: قائمة<عدد>
```

### 5.5 Special Types

| Type | Arabic | Description |
|------|--------|-------------|
| `any` | `أي` | Accepts any type (escape hatch) |
| `never` | `أبداً` | Function never returns |
| `unknown` | `مجهول` | Used during inference |

### 5.6 Type Compatibility

#### Implicit Conversions
- `عدد` → `عدد_عشري` (int to float)
- `T` → `T?` (value to optional)
- `عدم` → `T?` (null to optional)

#### String Concatenation Coercion
When using `+` with a string, other types are implicitly converted:
```tarqeem
"العدد: " + 42        // → "العدد: 42"
"القيمة: " + صحيح     // → "القيمة: true"
```

### 5.7 Type Inference

Types can be inferred from initialization:
```tarqeem
متغير س = 5          // س: عدد
متغير اسم = "أحمد"    // اسم: نص
ثابت قيم = [1, 2, 3] // قيم: مصفوفة<عدد>
```

### 5.8 Type Annotations

Explicit type annotations use colon syntax:
```tarqeem
متغير س: عدد = 5
ثابت ط: عدد_عشري = 3.14
متغير قائمة: مصفوفة<نص> = []
```

---

## 6. Expressions

### 6.1 Primary Expressions

```tarqeem
42                    // Integer literal
3.14                  // Float literal
"مرحبا"               // String literal
صحيح                  // Boolean true
خطأ                   // Boolean false
عدم                   // Null
متغير_اسم             // Identifier
(تعبير)               // Grouping
```

### 6.2 Arithmetic Expressions

```tarqeem
أ + ب                 // Addition
أ - ب                 // Subtraction
أ * ب                 // Multiplication
أ / ب                 // Division
أ % ب                 // Modulus
أ ** ب                // Exponentiation
-أ                    // Negation
```

### 6.3 Comparison Expressions

```tarqeem
أ == ب                // Equality
أ != ب                // Inequality
أ < ب                 // Less than
أ <= ب                // Less than or equal
أ > ب                 // Greater than
أ >= ب                // Greater than or equal
```

### 6.4 Logical Expressions

```tarqeem
أ && ب                // Logical AND
أ و ب                 // Arabic form of AND
أ || ب                // Logical OR
أ أو ب                // Arabic form of OR
!أ                    // Logical NOT
ليس أ                 // Arabic form of NOT
```

### 6.5 Assignment Expressions

```tarqeem
س = 5                 // Simple assignment
س += 1                // Add and assign
س -= 1                // Subtract and assign
س *= 2                // Multiply and assign
س /= 2                // Divide and assign
س++                   // Post-increment
++س                   // Pre-increment
س--                   // Post-decrement
--س                   // Pre-decrement
```

### 6.6 Ternary Expression

```tarqeem
شرط ? قيمة_صحيح : قيمة_خطأ

// Example
متغير حالة = عمر >= 18 ? "بالغ" : "قاصر"
```

### 6.7 Member Access

```tarqeem
كائن.خاصية            // Property access
كائن.دالة()           // Method call
```

### 6.8 Index Access

```tarqeem
مصفوفة[0]             // Array index
قاموس["مفتاح"]        // Map key access
```

### 6.9 Function Call

```tarqeem
دالة()                // No arguments
دالة(أ، ب)            // With arguments
كائن.دالة(أ)          // Method call
```

### 6.10 Object Creation

```tarqeem
جديد صنف()            // Constructor call
جديد صنف(أ، ب)        // With arguments
```

### 6.11 Lambda Expressions

```tarqeem
// Expression body
(س) => س * 2

// Block body
(س، ص) => {
    متغير نتيجة = س + ص
    أرجع نتيجة
}

// Typed parameters
(س: عدد، ص: عدد) => س + ص
```

### 6.12 Array Expressions

```tarqeem
[1, 2, 3]             // Array literal
[1، 2، 3]             // With Arabic comma
[]                    // Empty array
```

### 6.13 Await Expression

```tarqeem
انتظر وعد             // Await a promise
انتظر دالة_غير_متزامنة()
```

---

## 7. Statements

### 7.1 Variable Declaration

```tarqeem
// Mutable variable
متغير اسم = "أحمد"
متغير س: عدد = 5

// Immutable constant
ثابت PI = 3.14159
ثابت قائمة: مصفوفة<عدد> = [1, 2, 3]
```

### 7.2 Expression Statement

```tarqeem
اطبع("مرحبا");        // Function call
س = 5;                // Assignment
س++;                  // Increment
```

### 7.3 Block Statement

```tarqeem
{
    متغير س = 5
    اطبع(س)
}
```

### 7.4 If Statement

```tarqeem
// Simple if
إذا (شرط) {
    // code
}

// If-else
إذا (شرط) {
    // code
} وإلا {
    // code
}

// If-else if-else
إذا (شرط١) {
    // code
} وإلا إذا (شرط٢) {
    // code
} وإلا {
    // code
}
```

### 7.5 While Loop

```tarqeem
طالما (شرط) {
    // code
}

// With break
طالما (صحيح) {
    إذا (انتهى) {
        أوقف
    }
}

// With continue
طالما (شرط) {
    إذا (تخطي) {
        استمر
    }
    // code
}
```

### 7.6 For Loop

```tarqeem
// C-style for
لكل (متغير ع = 0؛ ع < 10؛ ع++) {
    اطبع(ع)
}

// For-in (iteration)
لكل عنصر في مجموعة {
    اطبع(عنصر)
}
```

### 7.7 Do-While Loop

```tarqeem
افعل {
    // code - executes at least once
} طالما (شرط)
```

### 7.8 Match Statement

```tarqeem
تطابق (قيمة) {
    حالة 1 => اطبع("واحد")
    حالة 2، 3 => اطبع("اثنان أو ثلاثة")
    حالة "نص" => {
        // block body
        اطبع("نص")
    }
    غير_ذلك => اطبع("شيء آخر")
}
```

### 7.9 Return Statement

```tarqeem
أرجع                  // Return void
أرجع قيمة             // Return value
ارجع نتيجة            // Alternative spelling
```

### 7.10 Break and Continue

```tarqeem
أوقف                  // Break loop
اوقف                  // Alternative spelling
استمر                 // Continue loop
```

---

## 8. Functions

### 8.1 Function Declaration

```tarqeem
// Simple function
دالة تحية() {
    اطبع("مرحبا")
}

// With parameters
دالة جمع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب
}

// With default parameters (planned)
دالة تحية(اسم: نص = "ضيف") {
    اطبع("مرحباً يا " + اسم)
}
```

### 8.2 Return Type

The return type is specified with `->`:
```tarqeem
دالة مساحة(نصف_قطر: عدد_عشري) -> عدد_عشري {
    أرجع 3.14159 * نصف_قطر * نصف_قطر
}
```

If no return type is specified, the function returns `فراغ` (void).

### 8.3 Lambda Functions

```tarqeem
// Expression lambda
ثابت مربع = (س: عدد) => س * س

// Block lambda
ثابت معالج = (س: عدد) => {
    متغير نتيجة = س * 2
    أرجع نتيجة
}

// Inferred types
ثابت جمع = (أ، ب) => أ + ب
```

### 8.4 Function Calls

```tarqeem
تحية()               // No arguments
جمع(5، 3)            // With arguments
مصفوفة.طول()         // Method call
```

### 8.5 Recursion

```tarqeem
دالة فيبوناتشي(ن: عدد) -> عدد {
    إذا (ن <= 1) {
        أرجع ن
    }
    أرجع فيبوناتشي(ن - 1) + فيبوناتشي(ن - 2)
}
```

### 8.6 Built-in Functions

| Arabic | English | Description |
|--------|---------|-------------|
| `اطبع(قيمة)` | `print(value)` | Print to stdout |
| `طول(مصفوفة)` | `len(array)` | Get array length |

---

## 9. Object-Oriented Programming

### 9.1 Class Declaration

```tarqeem
صنف شخص {
    // Fields
    خاص اسم: نص
    خاص عمر: عدد

    // Constructor
    منشئ(اسم: نص، عمر: عدد) {
        هذا.اسم = اسم
        هذا.عمر = عمر
    }

    // Methods
    عام دالة تحية() {
        اطبع("مرحباً، أنا " + هذا.اسم)
    }
}
```

### 9.2 Visibility Modifiers

| Arabic | English | Access |
|--------|---------|--------|
| `عام` | `public` | Accessible everywhere |
| `خاص` | `private` | Accessible only within class |
| `محمي` | `protected` | Accessible in class and subclasses |

Default visibility is `public`.

### 9.3 Constructors

```tarqeem
صنف شخص {
    خاص اسم: نص

    منشئ(اسم: نص) {
        هذا.اسم = اسم
    }
}
```

### 9.4 Instance Reference

Use `هذا` (this) to reference the current instance:
```tarqeem
هذا.اسم              // Access field
هذا.دالة()           // Call method
```

### 9.5 Static Members

```tarqeem
صنف رياضيات {
    ثابت_صنف PI: عدد_عشري = 3.14159

    ثابت_صنف دالة جذر(س: عدد_عشري) -> عدد_عشري {
        // implementation
    }
}

// Usage
اطبع(رياضيات.PI)
متغير نتيجة = رياضيات.جذر(16.0)
```

### 9.6 Inheritance

```tarqeem
صنف موظف يرث شخص {
    خاص راتب: عدد_عشري

    منشئ(اسم: نص، عمر: عدد، راتب: عدد_عشري) {
        أساس(اسم، عمر)      // Call parent constructor
        هذا.راتب = راتب
    }

    عام دالة تحية() {      // Override parent method
        أساس.تحية()        // Call parent method
        اطبع("أنا موظف")
    }
}
```

### 9.7 Interfaces

```tarqeem
واجهة قابل_للطباعة {
    دالة اطبع_معلومات()
}

واجهة قابل_للمقارنة {
    دالة قارن(آخر: أي) -> عدد
}

صنف شخص يطبق قابل_للطباعة {
    عام دالة اطبع_معلومات() {
        // Implementation required
    }
}
```

### 9.8 Multiple Interface Implementation

```tarqeem
صنف منتج يطبق قابل_للطباعة، قابل_للمقارنة {
    // Must implement all interface methods
}
```

### 9.9 Generics

```tarqeem
صنف قائمة<ن> {
    خاص عناصر: مصفوفة<ن>

    منشئ() {
        هذا.عناصر = []
    }

    عام دالة أضف(عنصر: ن) {
        هذا.عناصر.ألحق(عنصر)
    }

    عام دالة احصل(فهرس: عدد) -> ن {
        أرجع هذا.عناصر[فهرس]
    }
}

// Usage
متغير أرقام = جديد قائمة<عدد>()
أرقام.أضف(1)
أرقام.أضف(2)
```

### 9.10 Object Instantiation

```tarqeem
متغير شخص = جديد شخص("أحمد"، 30)
متغير موظف = جديد موظف("محمد"، 35، 10000.0)
```

---

## 10. Modules and Imports

### 10.1 Import Statement

```tarqeem
// Named imports
استورد { قائمة، قاموس } من "مجموعات"

// Wildcard import with alias
استورد * كـ رياضيات من "رياضيات"

// Default import
استورد مساعد من "./مساعدات"

// English form
import { List, Map } from "collections"
```

### 10.2 Export Statement

```tarqeem
// Export function
صدّر دالة مساعدة() { }

// Export class
صدّر صنف أداة { }

// Export variable
صدّر ثابت الإصدار = "1.0.0"
```

### 10.3 Module Resolution

- Relative paths: `"./ملف"`, `"../مجلد/ملف"`
- Standard library: `"مجموعات"`, `"رياضيات"`, `"ملفات"`
- Package modules: `"حزمة/وحدة"`

---

## 11. Error Handling

### 11.1 Try-Catch-Finally

```tarqeem
حاول {
    // Code that might throw
    متغير نتيجة = عملية_خطرة()
} التقط (خطأ) {
    // Handle error
    اطبع("حدث خطأ: " + خطأ.رسالة)
} أخيراً {
    // Always executed
    تنظيف()
}
```

### 11.2 Throw Statement

```tarqeem
دالة قسمة(أ: عدد، ب: عدد) -> عدد {
    إذا (ب == 0) {
        ارمِ "لا يمكن القسمة على صفر"
    }
    أرجع أ / ب
}
```

### 11.3 Error Propagation

Errors propagate up the call stack until caught:
```tarqeem
دالة أ() {
    ب()  // Error from ب() propagates if not caught
}

دالة ب() {
    ارمِ "خطأ"
}

حاول {
    أ()
} التقط (خ) {
    اطبع(خ)  // Catches error from ب()
}
```

---

## 12. Concurrency

### 12.1 Async Functions

```tarqeem
غير_متزامن دالة احضر_بيانات(رابط: نص) -> نص {
    متغير استجابة = انتظر طلب_شبكة(رابط)
    أرجع استجابة.نص()
}
```

### 12.2 Await Expression

```tarqeem
غير_متزامن دالة رئيسية() {
    متغير بيانات = انتظر احضر_بيانات("https://api.example.com")
    اطبع(بيانات)
}
```

### 12.3 Async Execution Model

- Async functions return a promise-like object
- `انتظر` suspends execution until the promise resolves
- The runtime manages an event loop for I/O operations

---

## 13. Memory Model

### 13.1 Overview

Tarqeem uses a hybrid memory management approach:
- **Stack Allocation**: Primitives and small fixed-size values
- **Heap Allocation**: Objects, arrays, strings
- **Reference Counting**: Automatic memory management for heap objects

### 13.2 Value Types vs Reference Types

**Value Types** (passed by copy):
- `عدد` (int)
- `عدد_عشري` (float)
- `منطقي` (bool)

**Reference Types** (passed by reference):
- `نص` (string)
- `مصفوفة` (array)
- `صنف` instances (class)
- `قاموس` (map)

### 13.3 Reference Counting

Objects are automatically freed when no references remain:
```tarqeem
{
    متغير قائمة = [1, 2, 3]  // Reference count = 1
    متغير أخرى = قائمة       // Reference count = 2
}  // Both variables go out of scope, count = 0, freed
```

### 13.4 Null Safety

Optional types must be checked before use:
```tarqeem
متغير س: عدد? = عدم

// Direct use would be an error
// اطبع(س + 1)  // Error!

// Safe usage
إذا (س != عدم) {
    اطبع(س + 1)  // OK, س is known to be non-null
}
```

---

## 14. Standard Library

### 14.1 Core Types

| Module | Contents |
|--------|----------|
| `مجموعات` | `قائمة<ن>`, `قاموس<م،ق>`, `مجموعة<ن>` |
| `رياضيات` | `جذر()`, `قوة()`, `مطلق()`, trigonometry |
| `نص` | String manipulation utilities |
| `ملفات` | File system operations |
| `شبكة` | Networking (HTTP, sockets) |

### 14.2 Built-in Functions

| Function | Arabic | Description |
|----------|--------|-------------|
| `print()` | `اطبع()` | Output to stdout |
| `len()` | `طول()` | Get collection length |
| `type()` | `نوع()` | Get type name |

### 14.3 Array Methods

| Method | Arabic | Description |
|--------|--------|-------------|
| `push()` | `ألحق()` | Add to end |
| `pop()` | `احذف_آخر()` | Remove from end |
| `length` | `طول` | Get length |
| `map()` | `عيّن()` | Transform elements |
| `filter()` | `رشّح()` | Filter elements |

---

## 15. Formal Grammar

### 15.1 Notation

```
<non-terminal>
'literal'
[optional]
{zero-or-more}
(grouping)
|   alternative
```

### 15.2 Program Structure

```
program         := {statement}

statement       := var_decl
                 | func_decl
                 | class_decl
                 | interface_decl
                 | if_stmt
                 | while_stmt
                 | for_stmt
                 | match_stmt
                 | return_stmt
                 | break_stmt
                 | continue_stmt
                 | try_stmt
                 | throw_stmt
                 | import_stmt
                 | export_stmt
                 | expr_stmt
                 | block
```

### 15.3 Declarations

```
var_decl        := ('متغير' | 'ثابت') IDENTIFIER [':' type] ['=' expr] ';'

func_decl       := ['غير_متزامن'] 'دالة' IDENTIFIER '(' [params] ')' ['->' type] block

class_decl      := 'صنف' IDENTIFIER ['<' type_params '>']
                   ['يرث' IDENTIFIER]
                   ['يطبق' IDENTIFIER {',' IDENTIFIER}]
                   '{' {class_member} '}'

interface_decl  := 'واجهة' IDENTIFIER ['<' type_params '>']
                   '{' {method_sig} '}'
```

### 15.4 Types

```
type            := simple_type
                 | array_type
                 | map_type
                 | function_type
                 | optional_type
                 | generic_type

simple_type     := 'عدد' | 'عدد_عشري' | 'نص' | 'منطقي' | 'فراغ' | IDENTIFIER

array_type      := 'مصفوفة' '<' type '>'

map_type        := 'قاموس' '<' type ',' type '>'

function_type   := '(' [type {',' type}] ')' '->' type

optional_type   := type '?'

generic_type    := IDENTIFIER '<' type {',' type} '>'
```

### 15.5 Expressions

```
expr            := assignment

assignment      := ternary [('=' | '+=' | '-=' | '*=' | '/=' | '%=') assignment]

ternary         := or ['?' expr ':' ternary]

or              := and {('||' | 'أو') and}

and             := equality {('&&' | 'و') equality}

equality        := comparison {('==' | '!=') comparison}

comparison      := term {('<' | '<=' | '>' | '>=') term}

term            := factor {('+' | '-') factor}

factor          := power {('*' | '/' | '%') power}

power           := unary {'**' unary}

unary           := ('!' | '-' | 'ليس' | '++' | '--') unary
                 | postfix

postfix         := primary {call | index | member | ('++' | '--')}

primary         := literal
                 | IDENTIFIER
                 | 'هذا'
                 | 'أساس'
                 | '(' expr ')'
                 | array_literal
                 | object_literal
                 | lambda
                 | new_expr
                 | await_expr
```

### 15.6 Statements

```
if_stmt         := 'إذا' '(' expr ')' block ['وإلا' (if_stmt | block)]

while_stmt      := 'طالما' '(' expr ')' block

for_stmt        := 'لكل' '(' [var_decl | expr] ';' [expr] ';' [expr] ')' block
                 | 'لكل' IDENTIFIER 'في' expr block

match_stmt      := 'تطابق' '(' expr ')' '{' {match_arm} '}'

match_arm       := 'حالة' expr {',' expr} '=>' (expr | block)
                 | 'غير_ذلك' '=>' (expr | block)

try_stmt        := 'حاول' block ['التقط' '(' IDENTIFIER ')' block] ['أخيراً' block]

return_stmt     := 'أرجع' [expr] ';'

break_stmt      := 'أوقف' ';'

continue_stmt   := 'استمر' ';'

throw_stmt      := 'ارمِ' expr ';'
```

---

## Appendix A: Keyword Reference

### Complete Keyword List

| Category | Arabic | English Aliases | Token |
|----------|--------|-----------------|-------|
| Variables | `متغير` | `let`, `var` | `Let` |
| Variables | `ثابت` | `const` | `Const` |
| Functions | `دالة` | `function`, `fn` | `Function` |
| Functions | `أرجع`, `ارجع` | `return` | `Return` |
| Functions | `غير_متزامن` | `async` | `Async` |
| Functions | `انتظر` | `await` | `Await` |
| Control | `إذا`, `اذا` | `if` | `If` |
| Control | `وإلا`, `والا` | `else` | `Else` |
| Control | `طالما` | `while` | `While` |
| Control | `لكل` | `for` | `For` |
| Control | `في` | `in` | `In` |
| Control | `افعل` | `do` | `Do` |
| Control | `أوقف`, `اوقف` | `break` | `Break` |
| Control | `استمر` | `continue` | `Continue` |
| Control | `تطابق` | `match`, `switch` | `Match` |
| Control | `حالة` | `case` | `Case` |
| Control | `غير_ذلك` | `default` | `Default` |
| OOP | `صنف` | `class` | `Class` |
| OOP | `واجهة` | `interface` | `Interface` |
| OOP | `يرث` | `extends` | `Extends` |
| OOP | `يطبق` | `implements` | `Implements` |
| OOP | `عام` | `public` | `Public` |
| OOP | `خاص` | `private` | `Private` |
| OOP | `محمي` | `protected` | `Protected` |
| OOP | `ثابت_صنف` | `static` | `Static` |
| OOP | `منشئ` | `constructor` | `Constructor` |
| OOP | `هذا` | `this` | `This` |
| OOP | `أساس`, `اساس` | `super` | `Super` |
| OOP | `جديد` | `new` | `New` |
| Errors | `حاول` | `try` | `Try` |
| Errors | `التقط` | `catch` | `Catch` |
| Errors | `أخيراً`, `اخيرا` | `finally` | `Finally` |
| Errors | `ارمِ`, `ارم` | `throw` | `Throw` |
| Modules | `استورد` | `import` | `Import` |
| Modules | `صدّر`, `صدر` | `export` | `Export` |
| Modules | `من` | `from` | `From` |
| Modules | `كـ`, `ك` | `as` | `As` |
| Literals | `صحيح` | `true` | `True` |
| Literals | `خطأ`, `خطا` | `false` | `False` |
| Literals | `عدم` | `null`, `none` | `Null` |
| Logical | `و` | | `And` |
| Logical | `أو`, `او` | | `Or` |
| Logical | `ليس` | `not` | `Bang` |
| Types | `عدد` | `int` | `TypeInt` |
| Types | `عدد_عشري` | `float` | `TypeFloat` |
| Types | `نص` | `string` | `TypeString` |
| Types | `منطقي` | `bool` | `TypeBool` |
| Types | `مصفوفة` | `array` | `TypeArray` |
| Types | `قاموس` | `map`, `dict` | `TypeMap` |
| Types | `فراغ` | `void` | `TypeVoid` |
| Types | `أي`, `اي` | `any` | `TypeAny` |

---

## Appendix B: Version History

| Version | Date | Description |
|---------|------|-------------|
| 1.0.0 | 2025 | Initial specification |

---

## Appendix C: References

1. Unicode Standard Annex #9: Unicode Bidirectional Algorithm
2. Unicode Technical Standard #39: Unicode Security Mechanisms
3. LLVM Language Reference Manual
4. "Crafting Interpreters" by Robert Nystrom

---

**Copyright 2025 Tarqeem Project**
**حقوق النشر ٢٠٢٥ مشروع ترقيم**
