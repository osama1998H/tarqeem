# فلسفة اللغة العربية في ترقيم
# Arabic Language Philosophy in Tarqeem

هذا الملف يحدد الفلسفة والقواعد التقنية لدعم اللغة العربية في ترقيم.

---

## المبدأ الأساسي | Core Principle

**ترقيم ليست ترجمة للغة برمجة إنجليزية - بل هي لغة برمجة عربية أصيلة.**
**Tarqeem is NOT a translation of an English programming language - it is an authentic Arabic programming language.**

- الكلمات المفتاحية الإنجليزية **غير مدعومة**
- English keywords are **NOT supported**
- الهدف: أن يفهم أي عربي الكود دون الحاجة لترجمته ذهنياً إلى الإنجليزية

---

## القواعد الأربع لاختيار الكلمات المفتاحية

### 1. الوصف لا الترجمة

❌ **خطأ**: ترجمة الكلمة الإنجليزية حرفياً
✅ **صحيح**: اختيار كلمة تصف ما تفعله في السياق العربي

| الإنجليزية | الترجمة الحرفية | المصطلح الصحيح | السبب |
|------------|----------------|----------------|-------|
| interface | واجهة | ميثاق | الواجهة تعني الوجه، لكن interface هو عقد يُلزم بتنفيذ دوال |
| void | فراغ | (محذوفة) | الدوال تحذف `-> نوع` إن لم تُرجع قيمة |
| static | ثابت_صنف | مشترك | العضو مشترك بين كل نسخ الصنف |
| handle | مقبض | معرّف | المقبض هو مقبض الباب! handle هو معرّف يُشير إلى مورد |
| thread | خيط | مسار_تنفيذ | الخيط ترجمة حرفية لا معنى لها برمجياً |
| buffer | مخزن_مؤقت | ذاكرة_وسيطة | الذاكرة الوسيطة تصف الغرض |

### 2. الصحة النحوية

الجملة البرمجية يجب أن تُقرأ كجملة عربية صحيحة:

❌ `منتهية دالة حساب()` → ✅ `دالة منتهية حساب()`
❌ `ميثاق قابل_للطباعة` → ✅ `ميثاق الطباعة`

**القاعدة**: النعت يأتي بعد المنعوت في العربية.

### 3. الترتيب العربي

| الإنجليزية | الترتيب الحالي | الترتيب العربي الصحيح |
|------------|---------------|---------------------|
| `void function calc()` | ~~فراغ دالة حساب()~~ | `دالة حساب()` (بدون نوع إرجاع) |
| `static int count` | ~~ثابت_صنف عدد عداد~~ | `مشترك عدد عداد` |

### 4. الاكتمال الذاتي (لا اختصارات غامضة)

| الاختصار | المعنى المقصود | البديل الواضح |
|----------|---------------|--------------|
| جا | جيب الزاوية | جيب |
| جتا | جيب التمام | جيب_التمام |
| ظا | ظل الزاوية | ظل |
| باي | π | ط (النسبة التقريبية) |

---

## الكلمات المفتاحية المعتمدة

### كلمات جيدة (تحتفظ بها)

| الكلمة | السبب |
|--------|-------|
| متغير | تصف أن القيمة قابلة للتغيير |
| ثابت | تصف أن القيمة لا تتغير |
| دالة | مصطلح رياضي عربي أصيل |
| صنف | يصف القالب الذي تُنشأ منه الكائنات |
| ميثاق | العقد الذي يلتزم به الصنف |
| صحيح/خطأ | قيم منطقية واضحة |
| إذا/وإلا | كلمات شرطية عربية طبيعية |
| طالما | تصف الاستمرار بشرط |
| لكل | تصف التكرار على مجموعة |

---

## قواعد التسمية في المكتبة القياسية

### التسمية العامة

- **الفعل المضارع للدوال**: `احسب`، `أضف`، `احذف`
- **الاسم للدوال التي تُرجع قيمة**: `طول`، `حجم`، `نوع`
- **صيغة السؤال للدوال المنطقية**: `هل_فارغ`، `هل_موجود`

### التسمية في الرياضيات

| المفهوم | الصحيح | الاختصار المدعوم |
|---------|--------|-----------------|
| π | ط | باي |
| e | هـ | - |
| sin | جيب | جا |
| cos | جيب_التمام | جتا |
| tan | ظل | ظا |
| arcsin | جيب_عكسي | جا_عكسي |

---

## معايير التحقق

قبل إضافة أي كلمة مفتاحية، اسأل:

1. **هل يفهمها العربي بدون معرفة الإنجليزية؟**
2. **هل تُقرأ بشكل طبيعي في جملة عربية؟**
3. **هل تصف ما تفعله لا ما تُترجم منه؟**
4. **هل الترتيب نحوي صحيح؟**

---

## رسائل الأخطاء | Error Messages

### صيغة ثنائية اللغة (مطلوبة)

```rust
pub struct Diagnostic {
    pub message: String,   // Arabic (required)
    pub span: Span,
    pub level: DiagnosticLevel,
}
```

### رسائل الخطأ الشائعة

| English | Arabic |
|---------|--------|
| Undefined variable '{name}' | المتغير '{name}' غير معرّف |
| Type mismatch: expected {a}, found {b} | عدم تطابق الأنماط: متوقع {a}، وُجد {b} |
| Cannot assign to constant | لا يمكن تعيين قيمة لمتغير ثابت |
| Missing semicolon | فاصلة منقوطة مفقودة |
| Unexpected token | رمز غير متوقع |
| Function '{name}' not found | الدالة '{name}' غير موجودة |

---

## معالجة يونيكود | Unicode Handling

### تطبيع NFC (مطلوب)

Arabic text must be NFC-normalized before comparison:

```rust
use unicode_normalization::UnicodeNormalization;

fn normalize_identifier(s: &str) -> String {
    s.nfc().collect()
}

// ALWAYS normalize before:
// - Identifier comparison
// - Symbol table lookup
// - Error message formatting
```

### التعرف على الحروف العربية

```rust
fn is_arabic_letter(c: char) -> bool {
    matches!(c, '\u{0600}'..='\u{06FF}' |  // Arabic
                '\u{0750}'..='\u{077F}' |  // Arabic Supplement
                '\u{08A0}'..='\u{08FF}' |  // Arabic Extended-A
                '\u{FB50}'..='\u{FDFF}' |  // Arabic Presentation Forms-A
                '\u{FE70}'..='\u{FEFF}')   // Arabic Presentation Forms-B
}
```

### علامات الترقيم العربية

The lexer must accept both Arabic and ASCII punctuation:

| ASCII | Arabic | Name |
|-------|--------|------|
| `,` | `،` (U+060C) | Comma |
| `;` | `؛` (U+061B) | Semicolon |
| `?` | `؟` (U+061F) | Question mark |
| `"` | `«` `»` | Quotation marks |

```rust
fn is_comma(c: char) -> bool {
    c == ',' || c == '،'
}

fn is_semicolon(c: char) -> bool {
    c == ';' || c == '؛'
}
```

---

## معالجة RTL | RTL Text Handling

### String Literals

String literals preserve their content exactly:

```rust
// The lexer should NOT modify RTL ordering inside strings
let source = r#"اطبع("مرحباً")"#;
// The string content is exactly: مرحباً
```

### Mixed Identifiers

Identifiers can mix Arabic and ASCII (numbers, underscores):

```rust
// Valid identifiers
متغير
متغير1
متغير_اختبار
_متغير
```

---

## اختبارات دعم العربية | Testing Arabic Support

### الاختبارات المطلوبة

```rust
#[test]
fn test_arabic_identifier() {
    let source = "متغير اسم = \"أحمد\"";
    assert!(parse(source).is_ok());
}

#[test]
fn test_arabic_comma() {
    let source = "دالة(أ، ب)";  // Arabic comma
    assert!(parse(source).is_ok());
}

#[test]
fn test_normalization() {
    let a = "متغير";
    let b = "متغير";  // May have different byte sequence
    assert_eq!(normalize(a), normalize(b));
}

#[test]
fn test_mixed_direction() {
    let source = r#"متغير x = 5"#;  // Mixed Arabic/English
    assert!(parse(source).is_ok());
}
```

---

## التوثيق | Documentation

### تعليقات الكود

Write code comments in English (for broader accessibility), but document Arabic behavior:

```rust
// Handles Arabic keyword "متغير" (mutable variable declaration)
fn parse_let(&mut self) -> ParseResult<Stmt> { }
```

### توثيق المستخدم

User documentation should have:
- Arabic examples with Arabic keywords
- Clear mapping tables

---

## أمثلة تطبيقية | Examples

### مثال ١: تعريف ميثاق

```tarqeem
// ✅ صحيح
ميثاق الطباعة {
    دالة اطبع()
}
```

### مثال ٢: دالة بدون إرجاع

```tarqeem
// ✅ صحيح - فراغ محذوفة
دالة سجّل_الخطأ(رسالة: نص) { }
```

### مثال ٣: الدوال المثلثية

```tarqeem
// ✅ صحيح - أسماء كاملة
متغير س = جيب(زاوية)
متغير ص = جيب_التمام(زاوية)
```

---

## المراجع

- قواعد النحو العربي
- معجم المصطلحات الرياضية (مجمع اللغة العربية)
- Unicode Standard Annex #9: Unicode Bidirectional Algorithm
