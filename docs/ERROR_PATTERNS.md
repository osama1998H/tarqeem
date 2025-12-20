# Common Error Patterns and Solutions

This document catalogues common Tarqeem compiler errors with explanations and solutions. All errors are bilingual (Arabic + English).

---

## Table of Contents

1. [Type Errors](#type-errors)
2. [Scope Errors](#scope-errors)
3. [Function Errors](#function-errors)
4. [Class Errors](#class-errors)
5. [Import/Module Errors](#importmodule-errors)
6. [Parser Errors](#parser-errors)
7. [IR/Codegen Errors](#ircodegen-errors)

---

## Type Errors

### Type Mismatch in Assignment

**Error (Arabic)**: `لا يمكن تعيين قيمة من نوع X إلى Y`
**Error (English)**: `Cannot assign value of type X to Y`

**Cause**: Attempting to assign a value of one type to a variable of another incompatible type.

**Example**:
```tarqeem
متغير س: عدد = "نص"  // Error: Cannot assign String to Int
```

**Solution**: Use type conversion functions or correct the type annotation:
```tarqeem
متغير س: عدد = عدد("123")  // Convert string to int
// or
متغير س: نص = "نص"  // Use correct type
```

---

### Type Mismatch in Return Statement

**Error (Arabic)**: `نوع الإرجاع غير متوافق مع تعريف الدالة`
**Error (English)**: `Return type does not match function declaration`

**Cause**: Function returns a value of different type than declared.

**Example**:
```tarqeem
دالة احصل_اسم() -> عدد {
    أرجع "أحمد"  // Error: returning String, expected Int
}
```

**Solution**: Return correct type or fix function signature:
```tarqeem
دالة احصل_اسم() -> نص {
    أرجع "أحمد"  // Correct: returning String
}
```

---

### Invalid Binary Operation

**Error (Arabic)**: `لا يمكن تطبيق العملية X على النوعين Y و Z`
**Error (English)**: `Cannot apply operation X to types Y and Z`

**Cause**: Using an operator on incompatible types.

**Example**:
```tarqeem
متغير نتيجة = "نص" - 5  // Error: Cannot subtract from string
```

**Solution**: Convert types appropriately or use correct operator:
```tarqeem
متغير نتيجة = عدد("10") - 5  // Convert string to number first
// or
متغير نتيجة = "نص" + نص(5)  // String concatenation
```

---

## Scope Errors

### Undefined Variable

**Error (Arabic)**: `المتغير X غير معرف`
**Error (English)**: `Variable X is not defined`

**Cause**: Using a variable before it is declared.

**Example**:
```tarqeem
اطبع(س)  // Error: س is not defined
متغير س = 5
```

**Solution**: Declare variable before use:
```tarqeem
متغير س = 5
اطبع(س)  // Correct
```

---

### Undefined Function

**Error (Arabic)**: `الدالة X غير معرفة`
**Error (English)**: `Function X is not defined`

**Cause**: Calling a function that doesn't exist or hasn't been imported.

**Example**:
```tarqeem
متغير ن = دالة_غير_موجودة(5)  // Error: function not defined
```

**Solution**: Define the function or import it:
```tarqeem
استورد { الدالة } من "وحدة"

// or define it
دالة دالة_مطلوبة(س: عدد) -> عدد {
    أرجع س * 2
}
```

---

### Assignment to Constant

**Error (Arabic)**: `لا يمكن تعيين قيمة لمتغير ثابت`
**Error (English)**: `Cannot assign to constant variable`

**Cause**: Attempting to modify a constant (ثابت) after declaration.

**Example**:
```tarqeem
ثابت باي = 3.14
باي = 3.14159  // Error: cannot reassign constant
```

**Solution**: Use `متغير` for mutable values:
```tarqeem
متغير باي = 3.14
باي = 3.14159  // OK: متغير is mutable
```

---

## Function Errors

### Wrong Argument Count

**Error (Arabic)**: `عدد المعاملات غير صحيح: متوقع X، وجد Y`
**Error (English)**: `Wrong number of arguments: expected X, found Y`

**Cause**: Calling function with incorrect number of arguments.

**Example**:
```tarqeem
دالة جمع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب
}

اطبع(جمع(5))  // Error: expected 2 args, got 1
```

**Solution**: Provide all required arguments:
```tarqeem
اطبع(جمع(5، 3))  // Correct: 2 arguments
```

---

### Argument Type Mismatch

**Error (Arabic)**: `نوع المعامل غير متوافق: متوقع X، وجد Y`
**Error (English)**: `Argument type mismatch: expected X, found Y`

**Cause**: Passing argument of wrong type to function.

**Example**:
```tarqeem
دالة ضاعف(س: عدد) -> عدد {
    أرجع س * 2
}

ضاعف("خمسة")  // Error: expected Int, got String
```

**Solution**: Pass correct type or convert:
```tarqeem
ضاعف(5)          // Correct: pass Int
ضاعف(عدد("5"))   // Also correct: convert String to Int
```

---

### Missing Return Statement

**Error (Arabic)**: `الدالة تحتاج جملة إرجاع`
**Error (English)**: `Function requires return statement`

**Cause**: Function with return type doesn't always return a value.

**Example**:
```tarqeem
دالة احسب(س: عدد) -> عدد {
    إذا (س > 0) {
        أرجع س * 2
    }
    // Missing return for else case
}
```

**Solution**: Ensure all code paths return a value:
```tarqeem
دالة احسب(س: عدد) -> عدد {
    إذا (س > 0) {
        أرجع س * 2
    }
    أرجع 0  // Handle else case
}
```

---

## Class Errors

### Undefined Class

**Error (Arabic)**: `الصنف X غير معرف`
**Error (English)**: `Class X is not defined`

**Cause**: Using a class that hasn't been declared or imported.

**Example**:
```tarqeem
متغير شخص = جديد شخص()  // Error: class شخص not defined
```

**Solution**: Define or import the class:
```tarqeem
صنف شخص {
    منشئ() {}
}

متغير شخص1 = جديد شخص()  // Correct
```

---

### Super Constructor Not Called

**Error (Arabic)**: `يجب استدعاء منشئ الأساس في الصنف الوارث`
**Error (English)**: `Super constructor must be called in derived class`

**Cause**: Child class constructor doesn't call parent constructor.

**Example**:
```tarqeem
صنف حيوان {
    خاص اسم: نص
    منشئ(اسم: نص) {
        هذا.اسم = اسم
    }
}

صنف قط يرث حيوان {
    منشئ(اسم: نص) {
        // Missing أساس(اسم)
    }
}
```

**Solution**: Call super constructor first:
```tarqeem
صنف قط يرث حيوان {
    منشئ(اسم: نص) {
        أساس(اسم)  // Call parent constructor first
    }
}
```

---

### Member Not Found

**Error (Arabic)**: `العضو X غير موجود في النوع Y`
**Error (English)**: `Member X not found on type Y`

**Cause**: Accessing property or method that doesn't exist on object.

**Example**:
```tarqeem
صنف شخص {
    عام اسم: نص
    منشئ(اسم: نص) { هذا.اسم = اسم }
}

متغير ش = جديد شخص("أحمد")
اطبع(ش.عمر)  // Error: عمر not found on شخص
```

**Solution**: Use existing members or add the missing one:
```tarqeem
اطبع(ش.اسم)  // Correct: اسم exists
```

---

## Import/Module Errors

### Module Not Found

**Error (Arabic)**: `الوحدة X غير موجودة`
**Error (English)**: `Module X not found`

**Cause**: Import path doesn't resolve to a valid module file.

**Example**:
```tarqeem
استورد { قائمة } من "وحدة_غير_موجودة"  // Error
```

**Solution**: Check module path:
```tarqeem
// Relative import
استورد { قائمة } من "./مجموعات"

// Standard library import
استورد { قائمة } من "مجموعات/قائمة"
```

---

### Symbol Not Exported

**Error (Arabic)**: `الرمز X غير مصدّر من الوحدة Y`
**Error (English)**: `Symbol X not exported from module Y`

**Cause**: Trying to import something that isn't exported.

**Example**:
```tarqeem
// في ملف أدوات.trq
دالة مساعدة_خاصة() {}  // Not exported

// في ملف آخر
استورد { مساعدة_خاصة } من "./أدوات"  // Error
```

**Solution**: Export the symbol in the source module:
```tarqeem
// في ملف أدوات.trq
صدّر دالة مساعدة_عامة() {}  // Now exported
```

---

### Circular Import

**Error (Arabic)**: `استيراد دائري مكتشف: X -> Y -> X`
**Error (English)**: `Circular import detected: X -> Y -> X`

**Cause**: Two or more modules import each other in a cycle.

**Example**:
```
// أ.trq imports ب.trq
// ب.trq imports أ.trq
```

**Solution**: Restructure modules to break the cycle:
1. Move shared types to a common module
2. Use interface instead of concrete type
3. Merge modules if closely related

---

## Parser Errors

### Unexpected Token

**Error (Arabic)**: `رمز غير متوقع: X`
**Error (English)**: `Unexpected token: X`

**Cause**: Syntax error - token in unexpected position.

**Example**:
```tarqeem
متغير س = = 5  // Error: unexpected =
```

**Solution**: Fix syntax:
```tarqeem
متغير س = 5  // Correct
```

---

### Expected Token

**Error (Arabic)**: `متوقع X، وجد Y`
**Error (English)**: `Expected X, found Y`

**Cause**: Parser expected specific token but found something else.

**Example**:
```tarqeem
دالة جمع(أ عدد) {}  // Error: expected :, found عدد
```

**Solution**: Add missing syntax:
```tarqeem
دالة جمع(أ: عدد) {}  // Correct: add colon
```

---

### Unclosed Delimiter

**Error (Arabic)**: `قوس غير مغلق`
**Error (English)**: `Unclosed delimiter`

**Cause**: Missing closing bracket, brace, or parenthesis.

**Example**:
```tarqeem
متغير أرقام = [1، 2، 3  // Error: missing ]
```

**Solution**: Close all delimiters:
```tarqeem
متغير أرقام = [1، 2، 3]  // Correct
```

---

## IR/Codegen Errors

### Empty Else Block

**Status**: FIXED (2025-12-20)

**Cause**: Empty else block in if-statement caused LLVM to fail.

**Solution Applied**: When `else_branch` is `None`, IR builder now branches directly to merge block.

---

### Global Constants Not Visible

**Status**: FIXED (2025-12-20)

**Cause**: Global constants weren't visible inside functions during IR generation.

**Solution Applied**: Added first pass to collect global constants before processing functions.

---

### Type Conversion Function Calls

**Status**: FIXED (2025-12-20)

**Cause**: Type keywords (`نص`, `منطقي`, `عدد`, `عدد_عشري`) weren't parsed as function identifiers.

**Solution Applied**: Parser now treats type keywords as identifiers in expression context.

```tarqeem
// These now work:
متغير أ = نص(42)        // String conversion
متغير ب = منطقي(1)      // Bool conversion
متغير ج = عدد(3.14)     // Int conversion
متغير د = عدد_عشري(42)  // Float conversion
```

---

## Best Practices to Avoid Errors

1. **Always declare before use** - Declare variables and functions before referencing them.

2. **Explicit type annotations** - Use type annotations for clarity and better error messages:
   ```tarqeem
   متغير اسم: نص = "أحمد"
   ```

3. **Check return types** - Ensure all code paths return correct type.

4. **Use Arabic comma** - Use `،` instead of `,` for consistency:
   ```tarqeem
   دالة جمع(أ: عدد، ب: عدد) -> عدد
   ```

5. **Import explicitly** - Use named imports instead of wildcards:
   ```tarqeem
   استورد { قائمة، مجموعة } من "مجموعات"
   ```

6. **Test incrementally** - Use `tarqeem check` frequently during development.

---

## Getting More Help

- Run `tarqeem check <file>` for detailed error messages
- Check `docs/AI_NOTES.md` for implementation notes
- Review `stdlib_trq/` for usage examples
- Search tests in `tests/` for similar patterns
