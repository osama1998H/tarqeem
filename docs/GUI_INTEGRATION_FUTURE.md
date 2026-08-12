# GUI Integration Future for Tarqeem

**Document Version**: 1.0
**Date**: 2025-12-27
**Status**: Proposal / Feasibility Study

---

## Executive Summary

This document assesses the feasibility of adding GUI (Graphical User Interface) capabilities to the Tarqeem programming language. The analysis concludes that GUI support is **highly feasible** and should be implemented as a standard library module (`رسومات`).

**Key Findings:**
- Tarqeem's LLVM backend already supports C library linking
- FFI (Foreign Function Interface) syntax is the only missing piece
- SDL2 is the recommended graphics library for initial implementation
- Estimated effort: 3-4 engineer-months across 3 phases

---

## Table of Contents

1. [Current State](#1-current-state)
2. [Recommended Architecture](#2-recommended-architecture)
3. [Implementation Phases](#3-implementation-phases)
4. [Technical Requirements](#4-technical-requirements)
5. [Graphics Library Comparison](#5-graphics-library-comparison)
6. [Impact Analysis](#6-impact-analysis)
7. [Risk Assessment](#7-risk-assessment)
8. [Proposed API Design](#8-proposed-api-design)
9. [Timeline & Resources](#9-timeline--resources)
10. [Conclusion](#10-conclusion)

---

## 1. Current State

### Can GUI Applications Be Built Today?

**No** - Tarqeem currently has no native GUI support. However, the foundational infrastructure for adding graphics capabilities is excellent.

### Infrastructure Readiness

| Capability | Status | Notes |
|------------|--------|-------|
| C library linking | ✅ Ready | `-lSDL2`, `-lGL` can be added to linker |
| LLVM external function calls | ✅ Ready | 90+ C runtime functions already work |
| WebAssembly output | ✅ Ready | Browser-based graphics possible |
| Memory management | ✅ Ready | Reference counting compatible with C |
| FFI syntax (`خارجي` keyword) | ❌ Missing | Parser enhancement needed |
| Graphics bindings | ❌ Missing | Runtime library additions needed |
| High-level Arabic API | ❌ Missing | Standard library module needed |

### Existing FFI Infrastructure

The compiler already interfaces with C code extensively:

```
src/codegen/llvm/codegen.rs  → Declares 90+ external C functions
src/codegen/linker.rs        → Links with -lc -lm flags
runtime/tarqeem_rt.h         → 1,605 lines of C FFI interface
```

**Key Insight**: The codegen can already emit LLVM `declare` statements for external functions. We only need parser syntax to expose this to users.

---

## 2. Recommended Architecture

### Two-Layer Design

```
┌─────────────────────────────────────────────────────────────┐
│                    رسومات (stdlib)                      │
│              High-level Arabic GUI API                       │
│         نافذة، زر، عنوان، صورة، لوحة، قائمة، الخ            │
├─────────────────────────────────────────────────────────────┤
│                   FFI Bindings (runtime/)                    │
│              C code calling SDL2/OpenGL/Cairo                │
│                  Low-level, performance-critical             │
├─────────────────────────────────────────────────────────────┤
│                   Native Graphics Library                    │
│                    SDL2 / OpenGL / Cairo                     │
└─────────────────────────────────────────────────────────────┘
```

### Design Principles

1. **Arabic-First API**: All user-facing classes and methods use Arabic names
2. **Performance**: Rendering logic in C, application logic in Tarqeem
3. **Cross-Platform**: SDL2 provides Windows, macOS, Linux support
4. **Extensibility**: FFI syntax enables future library bindings

---

## 3. Implementation Phases

### Phase 1: FFI Syntax (v1.6)

**Duration**: 2-3 weeks
**Complexity**: Low
**Goal**: Enable calling external C functions from Tarqeem

**Deliverables**:
- `خارجي` / `extern` keyword in lexer
- External function declaration parsing
- LLVM `declare` emission (already works)
- Documentation and examples

**Example Syntax**:
```tarqeem
// Declare external C function
خارجي دالة SDL_Init(flags: عدد) -> عدد

// Use it
متغير نتيجة = SDL_Init(0x00000020)
```

### Phase 2: SDL2 Bindings (v1.7)

**Duration**: 3-4 weeks
**Complexity**: Medium
**Goal**: Provide low-level graphics primitives

**Deliverables**:
- C runtime bindings for SDL2 core functions
- Basic Tarqeem wrapper module
- Window creation, event handling, basic drawing
- Build system integration (`-lSDL2` linking)

**Example**:
```tarqeem
استورد { نافذة_خام، حدث } من "رسومات/منخفض"

متغير نافذة = نافذة_خام.أنشئ("عنوان"، 800، 600)
طالما (صحيح) {
    متغير حدث = نافذة.انتظر_حدث()
    إذا (حدث.نوع == "إغلاق") {
        أوقف
    }
}
نافذة.أغلق()
```

### Phase 3: High-Level Widget Toolkit (v2.0)

**Duration**: 6-8 weeks
**Complexity**: High
**Goal**: Full Arabic GUI framework

**Deliverables**:
- Widget classes: `نافذة`، `زر`، `عنوان`، `حقل_نص`، `قائمة`، `صورة`
- Layout system: `صف`، `عمود`، `شبكة`
- Event system: `عند_نقر`، `عند_تغيير`، `عند_إغلاق`
- Styling support
- Comprehensive documentation

---

## 4. Technical Requirements

### 4.1 Lexer Changes

**File**: `src/lexer/keywords.rs`

Add new keyword:
```rust
// Arabic form
"خارجي" => Token::Extern,
// English alias
"extern" => Token::Extern,
```

### 4.2 Parser Changes

**File**: `src/parser/parser.rs`

Add external function declaration parsing:
```rust
fn parse_extern_declaration(&mut self) -> Result<Stmt, ParseError> {
    self.expect(Token::Extern)?;
    self.expect(Token::Function)?;
    let name = self.parse_identifier()?;
    let params = self.parse_parameters()?;
    let return_type = self.parse_optional_return_type()?;

    Ok(Stmt::ExternFunc { name, params, return_type })
}
```

### 4.3 Semantic Analyzer Changes

**File**: `src/semantic/analyzer.rs`

- Register external functions in scope
- Type-check calls to external functions
- Validate parameter types are FFI-compatible

### 4.4 Codegen Changes

**File**: `src/codegen/llvm/codegen.rs`

Emit LLVM declarations (pattern already exists):
```llvm
declare i64 @SDL_Init(i64)
declare ptr @SDL_CreateWindow(ptr, i64, i64, i64, i64, i64)
```

### 4.5 Linker Changes

**File**: `src/codegen/linker.rs`

Add graphics library flags:
```rust
// Add SDL2 linking
cmd.arg("-lSDL2");
cmd.arg("-lSDL2main");
```

### 4.6 Runtime Library Additions

**Directory**: `runtime/`

New files:
- `graphics.c` - SDL2 wrapper functions
- `graphics.h` - Header declarations

---

## 5. Graphics Library Comparison

| Library | Type | Pros | Cons | Complexity |
|---------|------|------|------|------------|
| **SDL2** | 2D + OpenGL | Simple C API, cross-platform, mature, gaming-ready | Lower-level than widget toolkits | ⭐⭐ |
| **Cairo** | 2D Vector | Clean API, excellent for drawing, PDF export | No widgets, 2D only | ⭐⭐⭐ |
| **OpenGL** | 3D | Industry standard, powerful, hardware-accelerated | Very low-level, steep learning curve | ⭐⭐⭐⭐ |
| **GTK+** | Widgets | Full-featured, mature, Linux-native | Complex API, heavy dependencies | ⭐⭐⭐⭐ |
| **Qt** | Widgets | Professional, powerful, cross-platform | C++ (ABI complexity), licensing | ⭐⭐⭐⭐⭐ |

### Recommendation: SDL2

**Rationale**:
1. Pure C API - easiest FFI binding
2. Cross-platform (Windows, macOS, Linux, mobile)
3. Can combine with OpenGL for advanced graphics
4. Well-documented and actively maintained
5. Suitable for both applications and games

---

## 6. Impact Analysis

### Positive Impacts

| Impact Area | Description |
|-------------|-------------|
| **Adoption** | GUI applications = real-world usage = more developers |
| **Education** | Visual programming is engaging for Arabic-speaking learners |
| **Ecosystem** | Opens door to game development, desktop apps, developer tools |
| **Arabic Computing** | First compiled Arabic language with native GUI capabilities |
| **FFI Benefit** | Same syntax enables database drivers, crypto libraries, etc. |

### Ecosystem Growth Potential

```
GUI Support Enables:
├── Desktop Applications
│   ├── Text editors
│   ├── Calculators
│   ├── File managers
│   └── Educational tools
├── Games
│   ├── 2D games
│   ├── Puzzle games
│   └── Educational games
├── Developer Tools
│   ├── IDE components
│   ├── Debugger UI
│   └── Package manager UI
└── Creative Applications
    ├── Drawing programs
    ├── Music applications
    └── Presentation tools
```

---

## 7. Risk Assessment

### Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| FFI complexity | Low | Medium | Leverage existing LLVM infrastructure |
| Cross-platform issues | Medium | High | Use SDL2 (proven cross-platform) |
| Performance problems | Low | Medium | Keep rendering in C, logic in Tarqeem |
| Memory leaks at FFI boundary | Medium | Medium | Careful reference counting at boundaries |

### Project Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Scope creep | High | High | Start minimal (SDL2 only), expand later |
| Maintenance burden | Medium | High | Bind one library well, not many poorly |
| Documentation gaps | Medium | Medium | Document alongside implementation |
| Breaking changes | Low | Medium | Design stable API before v2.0 release |

---

## 8. Proposed API Design

### 8.1 Low-Level API (`رسومات/منخفض`)

Direct SDL2 bindings with Arabic names:

```tarqeem
استورد { نافذة_خام، سطح، حدث، لون } من "رسومات/منخفض"

// Create window
متغير نافذة = نافذة_خام.أنشئ("تطبيقي"، 800، 600)

// Get drawing surface
متغير سطح = نافذة.احصل_سطح()

// Draw rectangle
سطح.ارسم_مستطيل(10، 10، 100، 50، لون.أحمر)

// Update display
نافذة.حدّث()
```

### 8.2 High-Level Widget API (`رسومات`)

Idiomatic Arabic widget toolkit:

```tarqeem
استورد { نافذة، زر، عنوان، حقل_نص، صف، عمود } من "رسومات"

// Create main window
متغير تطبيق = جديد نافذة("برنامجي"، 400، 300)

// Create widgets
متغير عنوان_رئيسي = جديد عنوان("مرحباً بك!")
متغير حقل_اسم = جديد حقل_نص("أدخل اسمك")
متغير زر_ترحيب = جديد زر("رحّب بي")

// Event handling
زر_ترحيب.عند_نقر(() => {
    متغير اسم = حقل_اسم.احصل_نص()
    عنوان_رئيسي.عيّن_نص("مرحباً يا " + اسم + "!")
})

// Layout
متغير تخطيط = جديد عمود([
    عنوان_رئيسي،
    حقل_اسم،
    زر_ترحيب
])

// Display
تطبيق.عيّن_محتوى(تخطيط)
تطبيق.اعرض()
```

### 8.3 Widget Class Hierarchy

```
عنصر (Base Widget)
├── نافذة (Window)
├── حاوية (Container)
│   ├── صف (Row)
│   ├── عمود (Column)
│   └── شبكة (Grid)
├── زر (Button)
├── عنوان (Label)
├── حقل_نص (TextField)
├── منطقة_نص (TextArea)
├── صورة (Image)
├── قائمة (List)
├── قائمة_منسدلة (Dropdown)
├── مربع_اختيار (Checkbox)
├── زر_راديو (RadioButton)
├── شريط_تقدم (ProgressBar)
└── منزلق (Slider)
```

### 8.4 Event System

```tarqeem
// Click events
زر.عند_نقر(() => { ... })

// Change events
حقل.عند_تغيير((قيمة_جديدة) => { ... })

// Window events
نافذة.عند_إغلاق(() => { ... })
نافذة.عند_تغيير_حجم((عرض، ارتفاع) => { ... })

// Keyboard events
نافذة.عند_ضغط_مفتاح((مفتاح) => { ... })

// Mouse events
عنصر.عند_دخول_فأرة(() => { ... })
عنصر.عند_خروج_فأرة(() => { ... })
```

---

## 9. Timeline & Resources

### Detailed Timeline

| Phase | Version | Duration | Start | End |
|-------|---------|----------|-------|-----|
| Phase 1: FFI Syntax | v1.6 | 2-3 weeks | TBD | TBD |
| Phase 2: SDL2 Bindings | v1.7 | 3-4 weeks | TBD | TBD |
| Phase 3: Widget Toolkit | v2.0 | 6-8 weeks | TBD | TBD |
| **Total** | | **11-15 weeks** | | |

### Resource Requirements

| Role | Phase 1 | Phase 2 | Phase 3 | Total |
|------|---------|---------|---------|-------|
| Compiler Engineer | 1 | 0.5 | 0.5 | ~2 months |
| Runtime Engineer | 0 | 1 | 1 | ~2 months |
| API Designer | 0.5 | 0.5 | 1 | ~2 months |
| Documentation | 0.5 | 0.5 | 1 | ~2 months |

**Total Effort**: ~3-4 engineer-months

### Dependencies

```
Phase 1 (FFI) ──┬──> Phase 2 (SDL2) ──> Phase 3 (Widgets)
                │
                └──> Other FFI uses (databases, crypto, etc.)
```

---

## 10. Conclusion

### Feasibility Summary

| Question | Answer |
|----------|--------|
| Can we build GUI today? | No |
| Is it technically feasible? | **Yes - HIGH feasibility** |
| Recommended approach? | Standard library module (`رسومات`) |
| Effort required | 3-4 engineer-months |
| Complexity | Medium (FFI infrastructure exists) |
| Impact on language | High - enables real applications |

### Recommendation

**Proceed with GUI implementation** following the phased approach:

1. **v1.6**: Implement `خارجي` FFI keyword (benefits beyond graphics)
2. **v1.7**: Ship SDL2 bindings with basic `رسومات/منخفض` module
3. **v2.0**: Release full widget toolkit with Arabic-first API

### Strategic Value

Adding GUI capabilities to Tarqeem would:

- Position it as the **first compiled Arabic language with native graphics**
- Enable **real-world application development**
- Attract developers interested in **Arabic-language desktop/game development**
- Demonstrate that **Arabic can be a first-class programming language**

---

## Appendix A: FFI Keyword Specification

### Syntax

```ebnf
extern_decl := 'خارجي' 'دالة' IDENTIFIER '(' [params] ')' ['->' type]
params      := param ('،' param)*
param       := IDENTIFIER ':' type
```

### Examples

```tarqeem
// No parameters, no return
خارجي دالة SDL_Quit()

// With parameters and return
خارجي دالة SDL_Init(flags: عدد) -> عدد

// Multiple parameters
خارجي دالة SDL_CreateWindow(
    title: نص،
    x: عدد،
    y: عدد،
    w: عدد،
    h: عدد،
    flags: عدد
) -> مؤشر
```

---

## Appendix B: SDL2 Function Mapping

| SDL2 Function | Arabic Wrapper | Description |
|---------------|----------------|-------------|
| `SDL_Init` | `هيئ` | Initialize SDL |
| `SDL_Quit` | `أنهِ` | Cleanup SDL |
| `SDL_CreateWindow` | `أنشئ_نافذة` | Create window |
| `SDL_DestroyWindow` | `أغلق_نافذة` | Destroy window |
| `SDL_PollEvent` | `استطلع_حدث` | Check for events |
| `SDL_WaitEvent` | `انتظر_حدث` | Wait for event |
| `SDL_GetWindowSurface` | `احصل_سطح` | Get drawing surface |
| `SDL_UpdateWindowSurface` | `حدّث_سطح` | Update display |
| `SDL_FillRect` | `املأ_مستطيل` | Fill rectangle |
| `SDL_BlitSurface` | `انسخ_سطح` | Copy surface |

---

## Appendix C: Related Documents

- [ROADMAP_V1.1-V1.5.md](./ROADMAP_V1.1-V1.5.md) - Current hardening roadmap
- [ARCHITECTURE.md](../ARCHITECTURE.md) - Compiler architecture
- [LANGUAGE_SPEC.md](../LANGUAGE_SPEC.md) - Language specification

---

**Document Author**: Claude AI
**Review Status**: Draft
**Next Review**: Before v1.6 planning
