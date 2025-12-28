# Tarqeem Arabic Purification Plan - Complete Audit

**Date:** 2025-12-28
**Version:** 1.0
**Goal:** Remove ALL English language artifacts to make Tarqeem a pure Arabic programming language

---

## Executive Summary

Tarqeem was developed with temporary bilingual (Arabic/English) support to enable faster development. Now that the language has reached stable alpha, it's time to purify the codebase to Arabic-only, fulfilling its mission as "the first compiled Arabic programming language."

### Current Status Summary

| Component | Status | Action Required |
|-----------|--------|-----------------|
| Lexer Keywords | ✅ Arabic-Only | None |
| Semantic Scope Builtins | ✅ Arabic-Only | None |
| Interpreter Builtins | ⚠️ Has English aliases | Remove English aliases |
| Error Messages | ⚠️ Bilingual | Keep Arabic, remove English |
| LSP Completions | ❌ English present | Remove English options |
| LSP Hover/Signatures | ❌ English present | Convert to Arabic |
| CLI Messages | ⚠️ Bilingual | Keep Arabic, remove English |
| REPL Interface | ⚠️ Bilingual | Keep Arabic, remove English |
| Debugger (DAP) | ❌ Mostly English | Convert to Arabic |
| Example Files | ⚠️ Has English comments | Remove English |

---

## Detailed Inventory

### 1. LEXER/KEYWORDS (Already Purified ✅)

**Location:** `/home/user/tarqeem/src/lexer/keywords.rs`

The lexer is already Arabic-only:
- 69 Arabic keyword entries
- NO English keywords supported
- English identifiers explicitly rejected with bilingual error message

**Test Verification:**
```rust
// Line 118-129 in keywords.rs
#[test]
fn test_english_keywords_not_supported() {
    assert_eq!(lookup_keyword("let"), None);       // ❌ Rejected
    assert_eq!(lookup_keyword("const"), None);     // ❌ Rejected
    assert_eq!(lookup_keyword("function"), None);  // ❌ Rejected
    // ... all English keywords return None
}
```

**Action:** ✅ None required

---

### 2. SEMANTIC SCOPE BUILTINS (Already Purified ✅)

**Location:** `/home/user/tarqeem/src/semantic/scope.rs`

All 157 built-in functions are registered with Arabic names only:
- `اطبع` (print)
- `طول` (length)
- `نوع` (type)
- `جذر` (sqrt)
- etc.

**Action:** ✅ None required

---

### 3. INTERPRETER BUILTINS (Needs Purification ⚠️)

**Location:** `/home/user/tarqeem/src/interpreter/executor/builtins.rs`

**Problem:** The interpreter accepts BOTH Arabic and English function names.

**English Aliases to Remove:**

| English | Arabic | Line(s) |
|---------|--------|---------|
| print, println | اطبع, طباعة, اطبع_سطر | 14-18, 157 |
| input | ادخل | 19, 173 |
| input_prompt | ادخل_رسالة | 22 |
| input_int | ادخل_عدد | 24 |
| input_float | ادخل_عشري | 26 |
| len, length | طول | 27, 29, 187 |
| type, typeof | نوع | 30, 32, 202 |
| int | عدد | 33, 212 |
| float | عدد_عشري | 35, 235 |
| str, string | نص | 37, 39, 257 |
| bool | منطقي | 40, 267 |
| abs | مطلق | 42, 277 |
| pow | قوة | 44, 292 |
| sqrt | جذر | 46 |
| cbrt | جذر_تكعيبي | 48 |
| log | لوغاريتم | 50 |
| log10 | لوغ10 | 52 |
| log2 | لوغ2 | 55 |
| exp | أس, أسي | 57 |
| floor | أرضية | 60 |
| ceil | سقف | 62 |
| round | قرب, تقريب | 64 |
| trunc | اقتطع | 67 |
| min | أقل, أدنى | 69 |
| max | أكبر, أقصى | 72 |
| clamp | حصر | 75 |
| sign | علامة | 77 |
| gcd | قاسم_مشترك | 79 |
| lcm | مضاعف_مشترك | 81 |
| factorial | عاملي | 83 |
| sin | جا, جيب | 85 |
| cos | جتا, جيب_التمام | 88 |
| tan | ظا, ظل | 91 |
| cot | ظتا, ظل_التمام | 94 |
| sec | قا, قاطع | 97 |
| csc | قتا, قاطع_التمام | 100 |
| asin | جا_عكسي, جيب_عكسي | 103 |
| acos | جتا_عكسي | 106 |
| atan | ظا_عكسي | 109 |
| atan2 | ظا_عكسي2 | 112 |
| sinh | جا_زائدي | 115 |
| cosh | جتا_زائدي | 118 |
| tanh | ظا_زائدي | 121 |
| to_radians | الى_راديان, راديان | 124 |
| to_degrees | الى_درجات, درجات | 127 |
| random | عشوائي | 130 |
| random_int, random_range | عشوائي_بين | 132-133 |
| random_float | عشوائي_عشري | 135 |
| random_bool | عشوائي_منطقي | 137 |
| assert | تأكد | 139 |
| assert_msg | تأكد_رسالة | 141 |
| panic | توقف | 143 |
| sleep | نم | 145 |
| time_now | وقت_الآن | 147 |

**Action:** Remove all English aliases from `is_builtin()` and `call_builtin()` match statements.

---

### 4. ERROR MESSAGES (Needs Purification ⚠️)

**Current Pattern:** All error messages use bilingual format:
```rust
Diagnostic::error(
    "English message",
    "رسالة عربية",
    span
)
```

**Locations:**
- `/home/user/tarqeem/src/lexer/lexer.rs` - 8 error types
- `/home/user/tarqeem/src/parser/parser/` - 70+ error types
- `/home/user/tarqeem/src/semantic/analyzer/` - 18+ error types
- `/home/user/tarqeem/src/error/` - Error infrastructure

**Decision Required:**
- **Option A:** Remove English field entirely (breaking change to error API)
- **Option B:** Keep bilingual internally but display Arabic-only to users
- **Option C:** Swap order (Arabic primary, English secondary)

**Recommended:** Option B - Keep infrastructure but display only Arabic

---

### 5. LSP COMPLETIONS (Needs Purification ❌)

**Location:** `/home/user/tarqeem/src/lsp/handlers/completion.rs`

**English Keywords to Remove (Lines 121-189):**

| English | Arabic | Line |
|---------|--------|------|
| let | متغير | 122 |
| const | ثابت | 123 |
| function | دالة | 125-126 |
| class | صنف | 129 |
| interface | ميثاق | 131-132 |
| import | استورد | 135 |
| export | صدّر | 136 |
| if | إذا | 172 |
| else | وإلا | 173 |
| while | طالما | 174 |
| for | لكل | 175 |
| return | أرجع | 176 |
| break | أوقف | 177 |
| continue | استمر | 178 |
| try | حاول | 180-181 |
| throw | ارمِ | 184 |
| match | تطابق | 186-187 |

**English Builtin Functions to Remove (Lines 221-231):**
- print → اطبع
- input → ادخل
- len → طول
- type → نوع
- int → عدد
- str → نص
- sqrt → جذر
- abs → مطلق
- read_file → اقرأ_ملف
- write_file → اكتب_ملف

**English Type Names to Remove (Lines 259-266):**
- int → عدد
- float → عدد_عشري
- string → نص
- bool → منطقي
- array → مصفوفة
- map → قاموس
- any → أي

**English Module Names (Lines 376-381):**
- collections → مجموعات
- math → رياضيات
- string → نص
- files → ملفات
- network → شبكة
- time → وقت

**English Member Methods (Lines 341-350):**
- length → طول
- push → ألحق
- pop → احذف_آخر
- isEmpty → فارغة
- slice → قص
- split → قسّم
- replace → استبدل
- contains → يحتوي
- toUpperCase → كبير
- toLowerCase → صغير

**Action:** Remove all English completion items, keep only Arabic.

---

### 6. LSP HOVER INFORMATION (Needs Purification ❌)

**Location:** `/home/user/tarqeem/src/lsp/handlers/hover.rs`

**English Symbol Labels (Lines 59-77):**
| English | Arabic |
|---------|--------|
| Variable | متغير |
| Function | دالة |
| Class | صنف |
| Interface | ميثاق |
| Parameter | معامل |
| Field | حقل |
| Method | دالة |
| Property | خاصية |
| Enum | تعداد |
| Enum Variant | حالة تعداد |

**English Builtin Docs (Lines 89-158):**
All builtin function descriptions need Arabic-only versions.

**Action:** Convert all hover text to Arabic-only.

---

### 7. LSP SIGNATURE HELP (Needs Purification ❌)

**Location:** `/home/user/tarqeem/src/lsp/handlers/signature_help.rs`

**English Labels:**
- Line 92: "function" → "دالة"
- Line 110: "Returns" → "يرجع"
- Line 120: "Note" → "ملاحظة"
- Line 128: "Warning" → "تحذير"

**English Function Signatures (Lines 246-378):**
All builtin function signatures shown in IDE need Arabic conversion.

**Action:** Convert all signature help to Arabic-only.

---

### 8. LSP CODE ACTIONS (Needs Purification ❌)

**Location:** `/home/user/tarqeem/src/lsp/handlers/code_actions.rs`

**English Texts:**
- Line 73: "Add declaration for '{}'" → "أضف تعريف لـ '{}'"
- Line 74: "let {} = " → "متغير {} = "
- Line 115: "Convert to mutable variable" → "حوّل إلى متغير قابل للتعديل"
- Line 200: "Extract to variable" → "استخرج إلى متغير"

**Action:** Convert all code action titles to Arabic.

---

### 9. CLI MESSAGES (Needs Purification ⚠️)

**Location:** `/home/user/tarqeem/src/cli/commands/`

**Current Pattern:** Mixed bilingual format "English / عربي"

**Example:**
```rust
"Could not read file: {} / لا يمكن قراءة الملف: {}"
```

**Files Affected:**
- `mod.rs` - Main CLI commands
- `compile.rs` - Compilation messages
- `debug.rs` - Debugger messages
- `pm/*.rs` - Package manager messages

**Action:** Convert to Arabic-only messages.

---

### 10. REPL INTERFACE (Needs Purification ⚠️)

**Location:** `/home/user/tarqeem/src/cli/commands/mod.rs` (Lines 545-632)

**Bilingual Messages:**
- Line 548: "=== Tarqeem REPL / الوضع التفاعلي لترقيم ===" → "=== الوضع التفاعلي لترقيم ==="
- Line 550: "Type 'exit' or 'خروج' to quit" → "اكتب 'خروج' للخروج"
- Line 573: "Goodbye! / مع السلامة!" → "مع السلامة!"

**English-Only Messages:**
- Line 609: "Runtime error:" → "خطأ وقت التشغيل:"
- Line 614: "IR error:" → "خطأ التمثيل الوسيط:"

**Action:** Convert to Arabic-only messages.

---

### 11. DEBUGGER (DAP) (Needs Purification ❌)

**Location:** `/home/user/tarqeem/src/cli/commands/debug.rs`

**English-Only Messages (Lines 108-544):**
- Line 108: "=== Tarqeem Debugger (trqdbg) ===" → "=== مصحح ترقيم (trqdbg) ==="
- Line 109: "Type 'help' for a list of commands" → "اكتب 'مساعدة' للحصول على قائمة الأوامر"
- Line 178: "Program terminated" → "انتهى البرنامج"
- Line 201: "trqdbg> " → "ترقيم> "
- Line 236: "Goodbye!" → "مع السلامة!"
- Line 311: "Breakpoint {} at {}:{}" → "نقطة توقف {} عند {}:{}"
- Line 544: "Unknown command" → "أمر غير معروف"

**Action:** Convert all debugger messages to Arabic.

---

### 12. EXAMPLE FILES (Needs Purification ⚠️)

**Location:** `/home/user/tarqeem/examples/`

**Files with English Comments/Text:**
- `صنف.ترقيم` - Line 4: "// Examples of classes"
- `دوال.ترقيم` - Line 4: "// Examples of functions"
- `مرحبا.ترقيم` - Lines 3,7: "Hello World" strings
- `لعبة_الحياة.ترقيم` - Line 3: "Conway's Game of Life"
- `حاسبة/اختبارات/اختبار.ترقيم` - English comments
- `planned/بصمة.ترقيم` - English description
- `planned/ضغط.ترقيم` - English description

**Action:** Remove or translate all English content.

---

## Phased Removal Plan

### Phase 1: Core Runtime (Highest Priority)
1. Remove English aliases from `interpreter/executor/builtins.rs`
2. Ensure all runtime functions only accept Arabic names

### Phase 2: LSP User Experience
1. Remove English completions from `lsp/handlers/completion.rs`
2. Convert hover info to Arabic in `lsp/handlers/hover.rs`
3. Convert signature help to Arabic in `lsp/handlers/signature_help.rs`
4. Convert code actions to Arabic in `lsp/handlers/code_actions.rs`

### Phase 3: CLI/REPL Experience
1. Convert CLI messages to Arabic-only
2. Convert REPL interface to Arabic-only
3. Convert debugger interface to Arabic

### Phase 4: Error Messages
1. Update Diagnostic display to show Arabic-only
2. Keep bilingual infrastructure for internal use

### Phase 5: Documentation & Examples
1. Translate example file comments to Arabic
2. Update all user-facing documentation

---

## Decision Points for User

1. **CLI Argument Names:** Should `--help` become `--مساعدة`?
   - Recommendation: Keep English for shell compatibility

2. **Error Codes:** Should `E0001` become `خ٠٠٠١`?
   - Recommendation: Keep alphanumeric codes for searchability

3. **File Extensions:** Should `.ترقيم` remain as-is?
   - Recommendation: Keep `.ترقيم` (already Arabic)

4. **Internal Comments:** Keep English for maintainers?
   - Recommendation: Keep internal comments in English for broader contributor access

---

## Verification Checklist

After purification, verify:
- [ ] All keywords are Arabic-only
- [ ] All builtin functions accept only Arabic names
- [ ] LSP provides only Arabic completions
- [ ] Error messages display in Arabic
- [ ] REPL interface is in Arabic
- [ ] Debugger interface is in Arabic
- [ ] Example files have no English code/comments
- [ ] Test suite passes with Arabic-only inputs

---

## Files to Modify

| File | Changes Required | Priority |
|------|------------------|----------|
| `src/interpreter/executor/builtins.rs` | Remove English aliases | High |
| `src/lsp/handlers/completion.rs` | Remove English completions | High |
| `src/lsp/handlers/hover.rs` | Convert to Arabic | High |
| `src/lsp/handlers/signature_help.rs` | Convert to Arabic | High |
| `src/lsp/handlers/code_actions.rs` | Convert to Arabic | High |
| `src/lsp/handlers/inlay_hints.rs` | Convert parameter names | Medium |
| `src/cli/commands/mod.rs` | Convert to Arabic-only | Medium |
| `src/cli/commands/compile.rs` | Convert to Arabic-only | Medium |
| `src/cli/commands/debug.rs` | Convert to Arabic-only | Medium |
| `src/cli/pm/*.rs` | Convert to Arabic-only | Medium |
| `examples/*.ترقيم` | Remove English comments | Low |

---

**ترقيم ليست ترجمة - بل لغة برمجة عربية أصيلة**
(Tarqeem is not a translation - it is an authentic Arabic programming language)
