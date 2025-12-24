# AI Implementation Notes

Persistent memory for AI agents working on Tarqeem. Update after significant changes.

---

## Current State

**Last Updated**: 2025-12-21
**Phase**: V1 Release Complete
**Tests**: 921+ passing
**Known Issues**: None critical

---

## Architectural Decisions

- **Layer Boundaries**: Lexer→Parser→Semantic→IR→Codegen (no reverse deps)
- **Bilingual Messages**: All user-facing strings have Arabic + English
- **NFC Normalization**: Arabic identifiers normalized before comparison
- **Error Recovery**: Never panic/unwrap on user input

---

## Implementation Log (Outline)

### 2024-12-20: Agent Context Awareness
- Created `.claude/rules/` modular rules system
- Created `.claude/commands/` reusable workflows
- Updated CLAUDE.md with project map

### 2025-12-20: Stress Test & Fixes
- Created Conway's Game of Life stress test
- Fixed empty else block bug
- Fixed global constants visibility
- Fixed C main entry point
- Fixed void function handling
- Fixed implicit returns

### 2025-12-20: Phase 3 Standard Library
- Implemented module system (استورد/صدّر)
- Created مجموعات (Collections): قائمة, مجموعة, خريطة, طابور, مكدس
- Created نص (String utilities)
- Created رياضيات (Math library)
- Created ملفات (File system)
- Created طرفية (Console I/O)
- Created شبكة (Networking)
- Created وقت (Date/Time)
- Created أخطاء (Error handling)

### 2025-12-20: Parser Updates
- Added generic type parameters for classes/interfaces
- Added automatic semicolon insertion
- Added generic type arguments in `new` expressions

### 2025-12-20: Phase 4 Planning
- Created comprehensive Phase 4 plan (Tooling)
- Designed package manager (trqpm)
- Planned LSP server, VS Code extension

### 2025-12-21: Global Variables
- Added GlobalLoad/GlobalStore IR instructions
- Updated LLVM codegen for globals
- Updated interpreter for globals

---

## Key Patterns

- **Token with Span**: Every token carries source location
- **Result Type Aliases**: `ParseResult<T>`, `TypeCheckResult<T>`
- **Bilingual Diagnostic**: `message` + `message_ar` fields

---

## TODOs

**Completed**:
- [x] Phase 1-3 complete
- [x] V1 release ready
- [x] All critical bugs fixed

**Pending**:
- [ ] Phase 4: Tooling implementation
- [ ] Package manager (trqpm)
- [ ] LSP server
- [ ] VS Code extension

---

### 2025-12-23: Phase 1 Arabic Philosophy Audit

**Task**: Remove `__xxx__` pattern from 170+ builtin functions

**Architecture Insight**:
- scope.rs: Registers builtin functions with Arabic names (type checking)
- runtime/: C library with `trq_*` functions (actual implementation)
- stdlib_trq/: Uses `__xxx__` pattern to call internal functions
- codegen: Generates LLVM IR, needs mapping Arabic→trq_*

**Solution Design**:
1. Add function name mapping in codegen (Arabic → `trq_*`)
2. Register all runtime functions in scope.rs
3. Update stdlib_trq to use clean Arabic function names
4. Update examples and tests

**Files Modified**:
- src/codegen/llvm/codegen.rs - Add runtime function mapping
- src/semantic/scope.rs - Register additional builtin functions
- stdlib_trq/**/*.ترقيم - Remove __xxx__ pattern
- examples/ - Update function calls
- tests/ - Update test code

---

---

### 2025-12-23: Arabic File Extensions and Package Format

**Task**: Implement fully Arabic file extensions and custom package configuration format

**Problem**:
- Current package manifest uses TOML format (`حزمة.toml`)
- TOML is an English-based format with English syntax (`[section]`, `=`, `{}`)
- This violates Arabic philosophy: "ترقيم ليست ترجمة - بل لغة برمجة عربية أصيلة"

**Solution**: Create custom Arabic configuration format "صيغة حزمة"

**New File Extensions**:
| Extension | Purpose |
|-----------|---------|
| `.حزمة` | Package manifest (replaces .toml) |
| `.قفل` | Lock file (replaces .trqlock) |

**Format Design**:
```
# تعليق
حزمة:
    اسم: مكتبتي
    نسخة: ١.٠.٠
    رخصة: MIT

اعتماديات:
    json: ٢.٠.٠
```

**Key Features**:
- Indentation-based (like YAML but Arabic)
- Arabic numerals support (٠-٩)
- Arabic booleans (نعم/لا)
- Arabic comments (#)
- No English syntax required

**Implementation Phases**:
1. Add extensions to `src/utils/extensions.rs`
2. Create parser module `src/package/format/`
3. Update manifest.rs for new format
4. Update init.rs to generate new format
5. Maintain TOML backward compatibility

**Files to Create**:
- `src/package/format/mod.rs`
- `src/package/format/lexer.rs`
- `src/package/format/parser.rs`
- `src/package/format/value.rs`
- `src/package/format/error.rs`

**Files to Modify**:
- `src/utils/extensions.rs`
- `src/package/mod.rs`
- `src/package/manifest.rs`
- `src/package/lockfile.rs`
- `src/cli/pm/init.rs`
- `README.md`

---

---

### 2025-12-23: Arabic Enums Implementation (تعداد)

**Task**: Design and implement enums following Arabic philosophy

## 1. Arabic Keyword Design

Following the four rules from `arabic-philosophy.md`:

| Rule | Application |
|------|-------------|
| **الوصف لا الترجمة** | "enum" = enumeration = تعداد (counting/listing items) |
| **الصحة النحوية** | `تعداد الألوان { ... }` reads naturally in Arabic |
| **الترتيب العربي** | Name follows keyword (like صنف، ميثاق) |
| **الاكتمال الذاتي** | تعداد is a complete, self-explanatory word |

**Chosen Keyword**: **تعداد** (ta'dād) - means "enumeration" or "numbered list"

**Rationale**:
- Proper Arabic word (not transliteration)
- Describes exactly what an enum is
- Flows naturally: `تعداد الحجم { صغير، متوسط، كبير }`
- Similar structure to existing keywords (صنف، ميثاق)

## 2. Proposed Syntax

### Simple Enum (Unit Variants)
```tarqeem
تعداد اللون {
    أحمر
    أخضر
    أزرق
}
```

### Enum with Explicit Values
```tarqeem
تعداد الحجم {
    صغير = 1
    متوسط = 2
    كبير = 3
}
```

### Enum with Associated Data (Tagged Unions)
```tarqeem
تعداد الرسالة {
    نص(محتوى: نص)
    رقم(قيمة: عدد)
    مركب(اسم: نص، عمر: عدد)
    فارغ
}
```

### Usage
```tarqeem
متغير لوني = اللون.أحمر
متغير رسالتي = الرسالة.نص("مرحباً")

تطابق (رسالتي) {
    حالة الرسالة.نص(م) => اطبع(م)
    حالة الرسالة.رقم(ق) => اطبع(ق)
    حالة الرسالة.فارغ => اطبع("فارغ")
}
```

## 3. Implementation Phases

### Phase 1: Lexer
- Add `Enum` to TokenKind in `src/lexer/token.rs`
- Add `"تعداد" => TokenKind::Enum` in `src/lexer/keywords.rs`

### Phase 2: Parser
- Add `EnumDecl`, `EnumVariant`, `EnumVariantField` to `src/parser/ast.rs`
- Add `parse_enum_declaration()` to `src/parser/parser.rs`

### Phase 3: Semantic Analysis
- Add `Type::Enum(String)` to `src/semantic/types.rs`
- Create `src/semantic/enum_resolver.rs` with EnumInfo, EnumVariantInfo
- Update `src/semantic/analyzer.rs` with enum handling

### Phase 4: IR Generation
- Add `EnumId`, `IrType::Enum` to `src/ir/instruction.rs`
- Add `EnumVariant`, `EnumVariantData`, `EnumDiscriminant`, `EnumIs`, `EnumField` instructions
- Update `src/ir/builder.rs`

### Phase 5: Code Generation
- Simple enums: represent as i64 discriminants
- Tagged unions: struct with tag + max-size payload
- Update `src/codegen/llvm/codegen.rs`

### Phase 6: Testing
- Lexer tests for keyword
- Parser tests for all syntax forms
- Semantic tests for type resolution
- Integration tests for full programs

## 4. Error Messages (Bilingual)

| Scenario | English | Arabic |
|----------|---------|--------|
| Expected enum name | Expected enum name | متوقع اسم التعداد |
| Duplicate variant | Duplicate enum variant '{name}' | حالة مكررة في التعداد '{name}' |
| Unknown variant | Unknown variant '{variant}' in enum '{enum}' | حالة غير معروفة '{variant}' في التعداد '{enum}' |
| Type mismatch | Expected enum type '{expected}' | النوع المتوقع '{expected}' |

## 5. Files to Modify

| File | Changes |
|------|---------|
| `src/lexer/token.rs` | Add `Enum` variant |
| `src/lexer/keywords.rs` | Add `تعداد` mapping |
| `src/parser/ast.rs` | Add EnumDecl, EnumVariant, EnumVariantField |
| `src/parser/parser.rs` | Add parse_enum_declaration |
| `src/semantic/types.rs` | Add Type::Enum |
| `src/semantic/enum_resolver.rs` | NEW: EnumInfo, EnumResolver |
| `src/semantic/analyzer.rs` | Add enum handling |
| `src/ir/instruction.rs` | Add EnumId, IrType::Enum, instructions |
| `src/ir/builder.rs` | Add enum IR generation |
| `src/codegen/llvm/codegen.rs` | Add enum codegen |

---

### 2025-12-24: DAP Server Implementation (Phase 1)

**Task**: Implement Debug Adapter Protocol server transport layer

**Context**:
- Debugger was 95% complete (~5,100 lines in src/debug/)
- DapAdapter existed with 20+ request handlers
- Missing: TCP/stdio transport layer for IDE integration

**Implementation**:

**New Files**:
| File | Lines | Purpose |
|------|-------|---------|
| `src/debug/server.rs` | ~500 | DAP server with TCP and stdio transports |

**Key Components**:
- `DapMessage`: Request/Response/Event envelope
- `DapProtocol`: Synchronous wire protocol (Content-Length headers)
- `DapProtocolAsync`: Async wire protocol for tokio
- `DapServer`: Main server with run_tcp(), run_stdio(), run_tcp_async(), run_stdio_async()
- `TransportError`: Bilingual error type (Arabic/English)

**CLI Changes**:
- Added `--dap-stdio` flag for VS Code integration
- Replaced TODO with actual DAP server implementation
- TCP mode: `tarqeem debug file.trq --dap-port 4711`
- Stdio mode: `tarqeem debug file.trq --dap-stdio`

**Files Modified**:
- `src/debug/server.rs` - NEW: Transport layer
- `src/debug/mod.rs` - Export server module
- `src/cli/mod.rs` - Add --dap-stdio flag
- `src/cli/commands.rs` - Implement DAP server startup

**Testing**:
- 10 new unit tests in server.rs
- All 61 debug module tests pass
- Full test suite passes (921+ tests)

**Design Decisions**:
- Followed LSP module patterns for consistency
- Both sync and async transports for flexibility
- Bilingual error messages throughout
- Atomic shutdown flag for graceful termination

**Next Steps (Phase 2)**:
- Async execution in adapter (non-blocking continue)
- Pause support
- SetVariable support
- VS Code extension scaffold

---

## Session Template

```markdown
### YYYY-MM-DD: Title
- What was changed
- Why
- Files modified
- Test results
```
