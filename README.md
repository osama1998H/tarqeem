<div dir="rtl">

# ترقيم - Tarqeem

**أول لغة برمجة عربية مُترجَمة للأغراض العامة**

</div>

## Overview

Tarqeem (ترقيم) is a compiled, general-purpose programming language with full Arabic syntax support. It combines the best features from Python's readability, PHP's flexibility, and JavaScript's modern capabilities.

### Why Tarqeem?

- **Native Arabic Support**: Write code entirely in Arabic with RTL text direction
- **Compiled Performance**: Compiles to native machine code for optimal performance
- **Modern Syntax**: Combines the best of Python, PHP, and JavaScript
- **Full OOP**: Classes, interfaces, traits, generics, and more
- **Type Safety**: Strong static typing with type inference

## Syntax Examples

### Hello World

```tarqeem
// English mode
print("Hello, World!")

// Arabic mode
اطبع("مرحباً بالعالم!")
```

### Variables and Types

```tarqeem
// Variable declaration (inspired by JavaScript's let/const)
متغير اسم = "أحمد"          // mutable variable (let)
ثابت عمر = 25              // constant (const)

// Type annotations (inspired by TypeScript)
متغير راتب: عدد_عشري = 5000.50
متغير متزوج: منطقي = صحيح

// Arabic type names
// عدد = integer
// عدد_عشري = float
// نص = string
// منطقي = boolean
// صحيح/خطأ = true/false
```

### Functions

```tarqeem
// Function definition (inspired by Python's def + JavaScript's arrow)
دالة جمع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب
}

// Arrow function style
ثابت مربع = (س: عدد) => س * س

// Function call
متغير نتيجة = جمع(5، 3)
اطبع(نتيجة)  // 8
```

### Control Flow

```tarqeem
// If-else (إذا-وإلا)
إذا (عمر >= 18) {
    اطبع("بالغ")
} وإلا إذا (عمر >= 13) {
    اطبع("مراهق")
} وإلا {
    اطبع("طفل")
}

// Match/Switch (تطابق)
تطابق (يوم) {
    حالة "السبت"، "الأحد" => اطبع("عطلة")
    حالة "الجمعة" => اطبع("نهاية الأسبوع")
    غير_ذلك => اطبع("يوم عمل")
}
```

### Loops

```tarqeem
// For loop (لكل)
لكل (متغير ع = 0؛ ع < 10؛ ع++) {
    اطبع(ع)
}

// For-in loop (لكل-في)
ثابت أرقام = [1، 2، 3، 4، 5]
لكل رقم في أرقام {
    اطبع(رقم)
}

// While loop (طالما)
متغير عداد = 0
طالما (عداد < 5) {
    اطبع(عداد)
    عداد++
}

// Do-while (افعل-طالما)
افعل {
    اطبع("مرة واحدة على الأقل")
} طالما (شرط)
```

### Classes and OOP

```tarqeem
// Interface (واجهة)
واجهة قابل_للطباعة {
    دالة اطبع_معلومات()
}

// Class (صنف)
صنف شخص يطبق قابل_للطباعة {
    // Properties
    خاص اسم: نص
    خاص عمر: عدد

    // Constructor (منشئ)
    منشئ(اسم: نص، عمر: عدد) {
        هذا.اسم = اسم
        هذا.عمر = عمر
    }

    // Method
    عام دالة اطبع_معلومات() {
        اطبع("الاسم: " + هذا.اسم + "، العمر: " + هذا.عمر)
    }

    // Getter
    عام دالة احصل_اسم() -> نص {
        أرجع هذا.اسم
    }
}

// Inheritance (يرث)
صنف موظف يرث شخص {
    خاص راتب: عدد_عشري

    منشئ(اسم: نص، عمر: عدد، راتب: عدد_عشري) {
        أساس(اسم، عمر)
        هذا.راتب = راتب
    }
}

// Usage
متغير شخص١ = جديد شخص("أحمد"، 30)
شخص١.اطبع_معلومات()
```

### Generics

```tarqeem
// Generic class (صنف معمم)
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

متغير أرقام = جديد قائمة<عدد>()
أرقام.أضف(1)
أرقام.أضف(2)
```

### Error Handling

```tarqeem
// Try-catch (حاول-التقط)
حاول {
    متغير نتيجة = قسمة(10، 0)
} التقط (خطأ) {
    اطبع("حدث خطأ: " + خطأ.رسالة)
} أخيراً {
    اطبع("تم الانتهاء")
}

// Throw (ارمِ)
دالة قسمة(أ: عدد، ب: عدد) -> عدد {
    إذا (ب == 0) {
        ارمِ خطأ_جديد("لا يمكن القسمة على صفر")
    }
    أرجع أ / ب
}
```

### Modules and Imports

```tarqeem
// Import (استورد)
استورد { قائمة، قاموس } من "مجموعات"
استورد * كـ رياضيات من "رياضيات"
استورد ملف_محلي من "./مساعدات"

// Export (صدّر)
صدّر دالة مساعدة() {
    // ...
}

صدّر صنف أداة {
    // ...
}
```

### Async/Await

```tarqeem
// Async function (دالة_متزامنة)
غير_متزامن دالة احضر_بيانات(رابط: نص) -> نص {
    متغير استجابة = انتظر طلب_شبكة(رابط)
    أرجع استجابة.نص()
}

// Usage
غير_متزامن دالة رئيسية() {
    متغير بيانات = انتظر احضر_بيانات("https://api.example.com")
    اطبع(بيانات)
}
```

## Language Keywords Reference

| Arabic | English | Description |
|--------|---------|-------------|
| متغير | let/var | Mutable variable |
| ثابت | const | Immutable constant |
| دالة | function | Function definition |
| أرجع | return | Return statement |
| إذا | if | Conditional |
| وإلا | else | Else clause |
| طالما | while | While loop |
| لكل | for | For loop |
| في | in | In operator |
| صنف | class | Class definition |
| واجهة | interface | Interface definition |
| يرث | extends | Inheritance |
| يطبق | implements | Interface implementation |
| عام | public | Public access |
| خاص | private | Private access |
| محمي | protected | Protected access |
| ثابت_صنف | static | Static member |
| منشئ | constructor | Constructor |
| هذا | this | This reference |
| أساس | super | Parent reference |
| جديد | new | Object instantiation |
| حاول | try | Try block |
| التقط | catch | Catch block |
| أخيراً | finally | Finally block |
| ارمِ | throw | Throw exception |
| استورد | import | Import module |
| صدّر | export | Export module |
| من | from | From clause |
| كـ | as | Alias |
| صحيح | true | Boolean true |
| خطأ | false | Boolean false |
| عدم | null/none | Null value |
| غير_متزامن | async | Async function |
| انتظر | await | Await expression |
| تطابق | match/switch | Pattern matching |
| حالة | case | Case clause |
| غير_ذلك | default | Default clause |

## Type System

| Arabic | English | Description |
|--------|---------|-------------|
| عدد | int | Integer |
| عدد_عشري | float | Floating point |
| نص | string | String |
| منطقي | bool | Boolean |
| مصفوفة | array | Array |
| قاموس | map/dict | Dictionary |
| فراغ | void | No return |
| أي | any | Any type |

## Installation

```bash
# Clone the repository
git clone https://github.com/osama1998H/tarqeem.git
cd tarqeem

# Build from source (requires Rust)
cargo build --release

# Install globally
cargo install --path .

# Verify installation
tarqeem --version
```

## Usage

```bash
# Compile a Tarqeem file
tarqeem compile برنامج.trq -o برنامج

# Run directly (compile and execute)
tarqeem run برنامج.trq

# Check syntax without compiling
tarqeem check برنامج.trq

# Format code
tarqeem fmt برنامج.trq

# Start REPL
tarqeem repl
```

## File Extensions

- `.trq` - Tarqeem source files
- `.trqh` - Tarqeem header/interface files

## Editor Support

RTL support depends on the editor. Recommended editors with RTL configuration:
- VS Code with Arabic support extensions
- Sublime Text with RTL plugins
- Custom Tarqeem IDE (planned)

## Roadmap

- [x] Language specification
- [ ] Lexer and tokenizer
- [ ] Parser and AST
- [ ] Semantic analyzer
- [ ] Type checker
- [ ] IR generation
- [ ] Code optimizer
- [ ] Native code generation (LLVM)
- [ ] Standard library
- [ ] Package manager
- [ ] LSP server
- [ ] VS Code extension
- [ ] Documentation generator

## Contributing

Contributions are welcome! Please read the ARCHITECTURE.md for technical details and CLAUDE.md for development guidelines.

## License

MIT License - See LICENSE file for details.

---

<div dir="rtl">

**ترقيم** - لغة البرمجة العربية 🇸🇦

</div>
