<div dir="rtl" align="right">

# معمارية ترقيم

هذه الوثيقة تصف المعمارية التقنية لمترجم ترقيم ووقت التشغيل.

---

## فهرس المحتويات

١. [لماذا رست؟](#لماذا-رست)
٢. [هيكل المشروع](#هيكل-المشروع)
٣. [خط أنابيب الترجمة](#خط-أنابيب-الترجمة)
٤. [تفاصيل المكونات](#تفاصيل-المكونات)
٥. [إدارة الذاكرة](#إدارة-الذاكرة)
٦. [معالجة الأخطاء](#معالجة-الأخطاء)
٧. [نموذج التزامن](#نموذج-التزامن)
٨. [معمارية المكتبة القياسية](#معمارية-المكتبة-القياسية)
٩. [نظام البناء](#نظام-البناء)
١٠. [الاعتماديات](#الاعتماديات)
١١. [استراتيجية الاختبار](#استراتيجية-الاختبار)
١٢. [أهداف الأداء](#أهداف-الأداء)

---

## لماذا رست؟

اخترنا **رست** لتنفيذ ترقيم للأسباب التالية:

| السبب | الشرح |
|-------|-------|
| **الأداء** | تُنتج ثنائيات أصلية بأداء مستوى C/C++ |
| **أمان الذاكرة** | آمنة للذاكرة بدون جامع قمامة عبر نظام الملكية |
| **أدوات ممتازة** | كارجو لإدارة الحزم، إطار اختبار قوي |
| **دعم يونيكود** | دعم UTF-8 من الدرجة الأولى ضروري للعربية |
| **معالجة الأخطاء** | أنماط Result تجعل معالجة أخطاء المترجم قوية |

---

## هيكل المشروع

```
tarqeem/
├── Cargo.toml                 # بيان حزمة رست
├── Cargo.lock                 # ملف قفل الاعتماديات
├── README.md                  # نظرة عامة على المشروع
├── ARCHITECTURE.md            # هذا الملف
├── CLAUDE.md                  # إرشادات التطوير
├── LANGUAGE_SPEC.md           # مواصفات اللغة
│
├── src/                       # الكود المصدري (~٣٤٬٠٠٠ سطر)
│   ├── main.rs               # نقطة دخول واجهة سطر الأوامر
│   ├── lib.rs                # جذر المكتبة
│   │
│   ├── lexer/                # التحليل اللغوي
│   │   ├── mod.rs
│   │   ├── token.rs          # تعريفات الرموز
│   │   ├── lexer.rs          # المحلل اللغوي الرئيسي
│   │   ├── keywords.rs       # خرائط الكلمات المفتاحية العربية
│   │   └── token_tests.rs    # اختبارات الرموز
│   │
│   ├── parser/               # التحليل النحوي
│   │   ├── mod.rs
│   │   ├── ast.rs            # تعريفات عُقد شجرة الصياغة
│   │   ├── precedence.rs     # أولوية العوامل
│   │   ├── parser/           # وحدات المحلل النحوي
│   │   └── parser_tests.rs   # اختبارات المحلل
│   │
│   ├── semantic/             # التحليل الدلالي
│   │   ├── mod.rs
│   │   ├── analyzer/         # المحلل الدلالي الرئيسي
│   │   ├── scope.rs          # جدول الرموز والنطاقات
│   │   ├── types.rs          # نظام الأنماط
│   │   ├── generics.rs       # الأنماط المعممة
│   │   ├── modules.rs        # نظام الوحدات
│   │   ├── class_resolver.rs # حل الأصناف
│   │   ├── method_resolver.rs # حل الدوال
│   │   ├── scope_tests.rs    # اختبارات النطاق
│   │   └── types_tests.rs    # اختبارات الأنماط
│   │
│   ├── ir/                   # التمثيل الوسيط
│   │   ├── mod.rs
│   │   ├── instruction.rs    # تعليمات التمثيل الوسيط
│   │   ├── builder/          # بانٍ التمثيل الوسيط
│   │   ├── opt/              # التحسينات
│   │   │   ├── const_fold.rs # طي الثوابت
│   │   │   ├── dce.rs        # حذف الكود الميت
│   │   │   ├── cse.rs        # حذف التعبيرات الفرعية المشتركة
│   │   │   ├── inline.rs     # التضمين
│   │   │   └── loop_opt.rs   # تحسين الحلقات
│   │   └── instruction_tests.rs
│   │
│   ├── codegen/              # توليد الكود
│   │   ├── mod.rs
│   │   ├── llvm/             # مولد كود LLVM
│   │   │   ├── codegen.rs    # التوليد الرئيسي
│   │   │   ├── types.rs      # تحويل الأنماط
│   │   │   └── codegen_tests.rs
│   │   ├── target.rs         # إعدادات الآلة المستهدفة
│   │   └── linker.rs         # أدوات الربط
│   │
│   ├── interpreter/          # المفسر المتجول
│   │   ├── mod.rs
│   │   ├── executor/         # المنفذ الرئيسي
│   │   ├── value.rs          # تمثيل القيم
│   │   ├── error.rs          # أخطاء التفسير
│   │   └── executor_tests.rs
│   │
│   ├── jit/                  # الترجمة الفورية (اختياري)
│   │   ├── mod.rs
│   │   ├── baseline/         # مترجم أساسي سريع
│   │   ├── optimizing/       # مترجم محسن
│   │   ├── runtime/          # وقت تشغيل JIT
│   │   ├── executor.rs       # منفذ JIT
│   │   ├── cache.rs          # ذاكرة تخزين مؤقت
│   │   ├── config.rs         # إعدادات JIT
│   │   ├── memory.rs         # إدارة الذاكرة
│   │   ├── profile.rs        # التنميط
│   │   └── tests.rs
│   │
│   ├── cli/                  # واجهة سطر الأوامر
│   │   ├── mod.rs
│   │   ├── commands/         # أوامر CLI
│   │   │   ├── compile.rs    # أمر الترجمة
│   │   │   ├── debug.rs      # أمر التنقيح
│   │   │   └── explain.rs    # أمر شرح الأخطاء
│   │   └── pm/               # مدير الحزم
│   │
│   ├── lsp/                  # بروتوكول خادم اللغة
│   │   ├── mod.rs
│   │   ├── server.rs         # الخادم الرئيسي
│   │   ├── capabilities.rs   # القدرات المدعومة
│   │   ├── state.rs          # حالة الخادم
│   │   ├── analysis/         # التحليل
│   │   ├── handlers/         # معالجات الطلبات (+٢٠ معالج)
│   │   │   ├── completion.rs    # الإكمال التلقائي
│   │   │   ├── hover.rs         # معلومات التمرير
│   │   │   ├── definition.rs    # الانتقال للتعريف
│   │   │   ├── references.rs    # البحث عن المراجع
│   │   │   ├── rename.rs        # إعادة التسمية
│   │   │   ├── formatting.rs    # التنسيق
│   │   │   ├── diagnostics.rs   # التشخيصات
│   │   │   ├── code_actions.rs  # إجراءات الكود
│   │   │   ├── folding.rs       # الطي
│   │   │   ├── inlay_hints.rs   # تلميحات السياق
│   │   │   ├── semantic_tokens.rs # الرموز الدلالية
│   │   │   ├── signature_help.rs  # مساعدة التوقيع
│   │   │   └── document_symbol.rs # رموز المستند
│   │   └── utils/
│   │
│   ├── debug/                # بروتوكول محول التنقيح (DAP)
│   │   ├── mod.rs
│   │   ├── adapter.rs        # المحول الرئيسي
│   │   ├── server.rs         # خادم DAP
│   │   ├── commands.rs       # أوامر التنقيح
│   │   ├── context.rs        # سياق التنفيذ
│   │   ├── source_map.rs     # خريطة المصدر
│   │   ├── state.rs          # حالة المنقح
│   │   ├── interpreter/      # مفسر التنقيح
│   │   └── tests.rs
│   │
│   ├── fmt/                  # منسق الكود
│   │   ├── mod.rs
│   │   ├── formatter.rs      # المنسق الرئيسي
│   │   ├── config.rs         # إعدادات التنسيق
│   │   └── printer.rs        # طابعة الكود
│   │
│   ├── doc/                  # مولد التوثيق
│   │   ├── mod.rs
│   │   ├── extractor.rs      # مستخرج التوثيق
│   │   ├── comment.rs        # معالج التعليقات
│   │   ├── model.rs          # نموذج التوثيق
│   │   └── generator/        # مولد الإخراج
│   │
│   ├── package/              # مدير الحزم
│   │   ├── mod.rs
│   │   ├── manifest.rs       # بيان الحزمة (ترقيم.حزمة)
│   │   ├── lockfile.rs       # ملف القفل (ترقيم.قفل)
│   │   ├── resolver.rs       # حل الاعتماديات
│   │   ├── cache.rs          # ذاكرة التخزين المؤقت
│   │   ├── error.rs          # أخطاء الحزم
│   │   └── format/           # صيغ الحزم
│   │
│   ├── error/                # معالجة الأخطاء
│   │   ├── mod.rs
│   │   ├── diagnostic.rs     # التشخيصات ثنائية اللغة
│   │   ├── codes.rs          # رموز الأخطاء العربية
│   │   ├── span.rs           # مواقع المصدر
│   │   ├── diagnostic_tests.rs
│   │   └── span_tests.rs
│   │
│   └── utils/                # أدوات مساعدة
│       ├── mod.rs
│       ├── interner.rs       # احتباس النصوص
│       ├── context.rs        # سياق الترجمة
│       └── extensions.rs     # امتدادات الأنماط
│
├── runtime/                  # مكتبة وقت التشغيل (C)
│   ├── tarqeem_rt.h         # الترويسة الرئيسية
│   ├── builtins.c           # الدوال المدمجة
│   ├── string.c             # عمليات النصوص
│   ├── array.c              # عمليات المصفوفات
│   ├── memory.c             # إدارة الذاكرة
│   ├── io.c                 # الإدخال/الإخراج
│   ├── crypto.c             # التشفير
│   ├── compress.c           # الضغط
│   └── wasm/                # دعم ويب أسمبلي
│       ├── runtime_wasm.c
│       └── tarqeem_wasm.h
│
├── stdlib_trq/               # المكتبة القياسية (كود ترقيم)
│   ├── README.md
│   ├── مجموعات/             # هياكل البيانات
│   │   ├── فهرس.ترقيم
│   │   ├── قائمة.ترقيم      # القائمة المتغيرة
│   │   ├── قاموس.ترقيم      # خريطة مفتاح-قيمة
│   │   ├── مجموعة.ترقيم     # المجموعة
│   │   ├── طابور.ترقيم      # الطابور
│   │   ├── مكدس.ترقيم       # المكدس
│   │   └── متكرر.ترقيم      # التكرار
│   ├── رياضيات/             # الدوال الرياضية
│   │   └── ...
│   ├── نص/                  # أدوات النصوص
│   │   └── ...
│   ├── ملفات/               # نظام الملفات
│   │   └── ...
│   ├── شبكة/                # الشبكات
│   │   └── ...
│   ├── وقت/                 # التاريخ والوقت
│   │   ├── فهرس.ترقيم
│   │   ├── تاريخ.ترقيم
│   │   └── وقت.ترقيم
│   ├── طرفية/               # أدوات الطرفية
│   │   ├── فهرس.ترقيم
│   │   ├── اساسي.ترقيم
│   │   └── الوان.ترقيم
│   ├── أخطاء/               # معالجة الأخطاء
│   │   └── ...
│   └── اختبار/              # إطار الاختبار
│       ├── فهرس.ترقيم
│       ├── توكيدات.ترقيم
│       ├── نتائج.ترقيم
│       └── مشغل.ترقيم
│
├── tests/                    # مجموعات الاختبار
│   └── integration/          # اختبارات التكامل
│
├── examples/                 # برامج أمثلة (+١٧ مثال)
│   ├── مرحبا.ترقيم           # مرحباً بالعالم
│   ├── حاسبة.ترقيم           # آلة حاسبة
│   ├── صنف.ترقيم             # البرمجة الكائنية
│   ├── لعبة_الحياة.ترقيم     # لعبة الحياة لكونواي
│   ├── تعداد.ترقيم           # التعدادات
│   └── ...
│
├── benches/                  # اختبارات الأداء
│   ├── lexer.rs
│   ├── parser.rs
│   ├── semantic.rs
│   ├── ir_generation.rs
│   ├── optimizer.rs
│   ├── end_to_end.rs
│   └── jit.rs
│
└── docs/                     # التوثيق
    ├── AI_NOTES.md
    ├── ROADMAP_V1.1-V1.5.md
    ├── PROFILING.md
    └── رموز_الأخطاء/
```

---

## خط أنابيب الترجمة

```
┌─────────────────────────────────────────────────────────────────────────┐
│                       خط أنابيب مترجم ترقيم                             │
└─────────────────────────────────────────────────────────────────────────┘

الكود المصدري (.ترقيم)
      │
      ▼
┌─────────────────┐
│  المحلل اللغوي  │  التقسيم إلى رموز: أحرف ← رموز
│    (Lexer)      │  - دعم يونيكود كامل (عربي)
│                 │  - معالجة صحيحة للنص ثنائي الاتجاه
│                 │  - تطبيع NFC للمعرِّفات
└─────────────────┘
      │
      │  تيار الرموز
      ▼
┌─────────────────┐
│  المحلل النحوي  │  التحليل النحوي: رموز ← شجرة صياغة
│    (Parser)     │  - تحليل تنازلي تكراري
│                 │  - تحليل برات للتعبيرات
│                 │  - استعادة الأخطاء
└─────────────────┘
      │
      │  شجرة الصياغة المجردة (AST)
      ▼
┌─────────────────┐
│ المحلل الدلالي  │  التحليل الدلالي: شجرة ← شجرة منمطة
│   (Semantic)    │  - حل الأسماء
│                 │  - فحص الأنماط
│                 │  - تحليل النطاقات
│                 │  - حل الأصناف والمواثيق
└─────────────────┘
      │
      │  شجرة صياغة منمطة
      ▼
┌─────────────────┐
│ مولد التمثيل    │  توليد التمثيل الوسيط: شجرة ← IR
│    الوسيط       │  - كود ثلاثي العناوين
│     (IR)        │  - صيغة SSA
│                 │  - رسم تدفق التحكم
└─────────────────┘
      │
      │  التمثيل الوسيط
      ▼
┌─────────────────┐
│    المحسِّن     │  التحسين: IR ← IR محسَّن
│  (Optimizer)    │  - طي الثوابت
│                 │  - حذف الكود الميت
│                 │  - حذف التعبيرات المشتركة
│                 │  - التضمين
│                 │  - تحسين الحلقات
└─────────────────┘
      │
      │  تمثيل وسيط محسَّن
      ▼
┌─────────────────┐
│  مولد الكود     │  توليد الكود: IR ← LLVM IR
│    (LLVM)       │  - توليد LLVM IR
│                 │  - تحسين خاص بالهدف
│                 │  - إصدار كود أصلي
└─────────────────┘
      │
      │  ملفات كائنية (.o)
      ▼
┌─────────────────┐
│    الرابط       │  الربط: ملفات ← تنفيذي
│    (Linker)     │  - ربط مكتبة وقت التشغيل
│                 │  - ربط المكتبة القياسية
└─────────────────┘
      │
      ▼
ملف تنفيذي أصلي
```

### المسارات البديلة

```
شجرة الصياغة ────┐
                 │
                 ▼
          ┌──────────────┐
          │   المفسر     │  للتنقيح و REPL
          │ (Interpreter)│
          └──────────────┘
                 │
                 ▼
            التنفيذ المباشر

التمثيل الوسيط ──┐
                 │
                 ▼
          ┌──────────────┐
          │  مترجم JIT   │  ترجمة فورية (Cranelift)
          │    (JIT)     │  - مترجم أساسي سريع
          └──────────────┘  - مترجم محسن
                 │
                 ▼
            تنفيذ فوري
```

---

## تفاصيل المكونات

### ١. المحلل اللغوي (Lexer)

المحلل اللغوي يحول الكود المصدري إلى رموز مع معالجة:

- **دعم يونيكود**: دعم UTF-8 كامل للأحرف العربية
- **النص ثنائي الاتجاه**: معالجة صحيحة للعربية (يمين-يسار) مع الأرقام (يسار-يمين)
- **تطبيع NFC**: تطبيع المعرِّفات قبل المقارنة

```rust
// أنواع الرموز
pub enum TokenKind {
    // القيم الحرفية
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),

    // المعرِّفات
    Identifier(String),

    // الكلمات المفتاحية العربية
    Mutaghayir,     // متغير
    Thabit,         // ثابت
    Dalah,          // دالة
    Irji,           // أرجع
    Itha,           // إذا
    WaIlla,         // وإلا
    Talama,         // طالما
    Likul,          // لكل
    Sinf,           // صنف
    Mithaq,         // ميثاق
    // ... المزيد

    // العوامل
    Plus, Minus, Star, Slash,
    Equal, EqualEqual, BangEqual,
    Less, LessEqual, Greater, GreaterEqual,
    // ...

    // المحددات
    LeftParen, RightParen,      // ( )
    LeftBrace, RightBrace,      // { }
    LeftBracket, RightBracket,  // [ ]
    Comma,                      // , أو ،
    Semicolon,                  // ; أو ؛
    Colon,                      // :
    Arrow,                      // ->
    FatArrow,                   // =>

    // خاص
    Newline, Whitespace, Comment,
    EOF, Error,
}
```

### ٢. المحلل النحوي (Parser)

محلل تنازلي تكراري مع تحليل برات للتعبيرات:

```rust
// عُقد التعبيرات
pub enum Expr {
    Literal(Literal),
    Identifier(String),
    Binary { left: Box<Expr>, op: BinaryOp, right: Box<Expr> },
    Unary { op: UnaryOp, operand: Box<Expr> },
    Call { callee: Box<Expr>, args: Vec<Expr> },
    Member { object: Box<Expr>, property: String },
    Index { object: Box<Expr>, index: Box<Expr> },
    Lambda { params: Vec<Param>, body: Box<Expr> },
    New { class: Box<Expr>, args: Vec<Expr> },
    Await(Box<Expr>),
    Ternary { cond: Box<Expr>, then_expr: Box<Expr>, else_expr: Box<Expr> },
}

// عُقد الجمل
pub enum Stmt {
    VarDecl { name: String, mutable: bool, ty: Option<Type>, init: Option<Expr> },
    FuncDecl { name: String, params: Vec<Param>, ret_ty: Option<Type>, body: Block },
    ClassDecl { name: String, extends: Option<String>, implements: Vec<String>, body: Vec<ClassMember> },
    InterfaceDecl { name: String, methods: Vec<MethodSignature> },
    If { cond: Expr, then_branch: Block, else_branch: Option<Block> },
    While { cond: Expr, body: Block },
    For { init: Option<Box<Stmt>>, cond: Option<Expr>, update: Option<Expr>, body: Block },
    ForIn { var: String, iterable: Expr, body: Block },
    Match { expr: Expr, arms: Vec<MatchArm> },
    Return(Option<Expr>),
    Try { body: Block, catch: Option<CatchClause>, finally: Option<Block> },
    Throw(Expr),
    Import { items: ImportItems, from: String },
    Export(Box<Stmt>),
    Expr(Expr),
}
```

### ٣. نظام الأنماط

كتابة ثابتة قوية مع استنتاج الأنماط:

```rust
pub enum Type {
    // الأنماط الأولية
    Int,            // عدد
    Float,          // عدد_عشري
    String,         // نص
    Bool,           // منطقي

    // الأنماط المركبة
    Array(Box<Type>),               // مصفوفة<ن>
    Map(Box<Type>, Box<Type>),      // قاموس<م، ق>
    Function { params: Vec<Type>, ret: Box<Type> },
    Optional(Box<Type>),            // ن?

    // الأنماط المُعرَّفة من المستخدم
    Class(String),
    Interface(String),
    Generic { name: String, constraints: Vec<Type> },

    // الأنماط الخاصة
    Any,            // أي
    Never,          // أبداً
    Unknown,        // مؤقت للاستنتاج
}
```

### ٤. التمثيل الوسيط (IR)

تمثيل وسيط بصيغة SSA للتحسين:

```rust
pub enum IRInstruction {
    // الثوابت
    Const { dest: VarId, value: Constant },

    // الحساب
    Add { dest: VarId, left: VarId, right: VarId },
    Sub { dest: VarId, left: VarId, right: VarId },
    Mul { dest: VarId, left: VarId, right: VarId },
    Div { dest: VarId, left: VarId, right: VarId },

    // المقارنة
    Eq { dest: VarId, left: VarId, right: VarId },
    Lt { dest: VarId, left: VarId, right: VarId },
    // ...

    // تدفق التحكم
    Jump { target: BlockId },
    Branch { cond: VarId, then_block: BlockId, else_block: BlockId },
    Return { value: Option<VarId> },

    // الدوال
    Call { dest: Option<VarId>, func: FuncId, args: Vec<VarId> },

    // الذاكرة
    Alloc { dest: VarId, ty: Type },
    Load { dest: VarId, ptr: VarId },
    Store { ptr: VarId, value: VarId },

    // الكائنات
    NewObject { dest: VarId, class: ClassId },
    GetField { dest: VarId, object: VarId, field: FieldId },
    SetField { object: VarId, field: FieldId, value: VarId },
    CallMethod { dest: Option<VarId>, object: VarId, method: MethodId, args: Vec<VarId> },
}
```

### ٥. التحسينات المُنفَّذة

| التحسين | الوصف | الموقع |
|---------|-------|--------|
| **طي الثوابت** | حساب التعبيرات الثابتة وقت الترجمة | `ir/opt/const_fold.rs` |
| **حذف الكود الميت** | إزالة الكود الذي لا يُنفَّذ | `ir/opt/dce.rs` |
| **حذف التعبيرات المشتركة** | إعادة استخدام الحسابات المتكررة | `ir/opt/cse.rs` |
| **التضمين** | استبدال استدعاءات الدوال الصغيرة بجسمها | `ir/opt/inline.rs` |
| **تحسين الحلقات** | تحسينات متعددة للحلقات | `ir/opt/loop_opt.rs` |

### ٦. توليد الكود (LLVM)

استخدام واجهة C لـ LLVM:

```rust
pub struct CodeGen {
    context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,

    // جداول الرموز
    variables: HashMap<String, LLVMValueRef>,
    functions: HashMap<String, LLVMValueRef>,
    classes: HashMap<String, LLVMTypeRef>,
}

impl CodeGen {
    pub fn compile_program(&mut self, program: &Program) -> Result<(), CodeGenError> {
        // ١. الإعلان المسبق عن جميع الدوال والأصناف
        self.forward_declare(program)?;

        // ٢. توليد أجسام الدوال
        for func in &program.functions {
            self.compile_function(func)?;
        }

        // ٣. توليد دوال الأصناف
        for class in &program.classes {
            self.compile_class(class)?;
        }

        // ٤. التحقق من الوحدة
        self.verify_module()?;

        Ok(())
    }
}
```

---

## إدارة الذاكرة

ترقيم تستخدم نهجاً هجيناً:

| الطريقة | الاستخدام |
|---------|-----------|
| **تخصيص المكدس** | الأوليات والقيم ثابتة الحجم الصغيرة |
| **عد المراجع** | الافتراضي لكائنات الكومة |
| **جامع القمامة** | اختياري للبيانات الدائرية |

```rust
// عد المراجع في وقت التشغيل
pub struct TrqObject {
    ref_count: AtomicUsize,
    type_info: &'static TypeInfo,
    data: [u8],  // مصفوفة مرنة
}

impl TrqObject {
    pub fn retain(&self) {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn release(&self) {
        if self.ref_count.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.drop_contents();
            // تحرير الذاكرة
        }
    }
}
```

---

## معالجة الأخطاء

تقارير أخطاء شاملة مع دعم العربية:

```rust
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub code: ErrorCode,           // رمز الخطأ العربي (مثل: د٠٣٠١)
    pub message: String,           // الرسالة بالإنجليزية
    pub message_ar: String,        // الرسالة بالعربية
    pub span: Span,
    pub notes: Vec<Note>,
    pub suggestions: Vec<Suggestion>,
}
```

### نظام رموز الأخطاء

| الحرف | الفئة | الوصف |
|-------|-------|-------|
| ق | قراءة | أخطاء المحلل اللغوي |
| ب | بناء | أخطاء المحلل النحوي |
| د | دلالة | أخطاء دلالية |
| ن | نوع | أخطاء الأنماط |
| ص | صنف | أخطاء البرمجة الكائنية |
| و | وحدة | أخطاء الاستيراد/التصدير |
| ت | توليد | أخطاء توليد الكود |
| ح | تحذير | تحذيرات |

---

## نموذج التزامن

متوازي/انتظر مع حلقة أحداث:

```tarqeem
متوازي دالة احضر_بيانات(رابط: نص) -> نص {
    متغير استجابة = انتظر طلب_شبكة(رابط)
    أرجع استجابة.نص()
}

متوازي دالة رئيسية() {
    متغير بيانات = انتظر احضر_بيانات("https://api.example.com")
    اطبع(بيانات)
}
```

---

## معمارية المكتبة القياسية

المكتبة القياسية مزيج من وقت تشغيل C وكود ترقيم:

```
runtime/                    # C: البدائيات منخفضة المستوى
├── memory.c               # تخصيص الذاكرة
├── string.c               # العمليات الداخلية للنصوص
├── array.c                # عمليات المصفوفات
├── io.c                   # الإدخال/الإخراج
├── crypto.c               # التشفير
└── compress.c             # الضغط

stdlib_trq/                 # ترقيم: واجهات عالية المستوى
├── مجموعات/               # قائمة، قاموس، مجموعة، طابور، مكدس
├── رياضيات/               # الدوال الرياضية
├── نص/                    # أدوات النصوص
├── ملفات/                 # نظام الملفات
├── شبكة/                  # الشبكات
├── وقت/                   # التاريخ والوقت
├── طرفية/                 # أدوات الطرفية
├── أخطاء/                 # معالجة الأخطاء
└── اختبار/                # إطار الاختبار
```

---

## نظام البناء

```bash
# بناء التطوير
cargo build

# بناء الإصدار مع التحسينات
cargo build --release

# تشغيل الاختبارات (+٩٢١ اختبار)
cargo test

# تشغيل اختبار معين
cargo test lexer

# توليد التوثيق
cargo doc --open

# تنسيق الكود
cargo fmt

# التدقيق
cargo clippy

# اختبارات الأداء
cargo bench

# بناء مع JIT (اختياري)
cargo build --features jit
```

### أوامر ترقيم

```bash
# ترجمة ملف ترقيم
tarqeem compile برنامج.ترقيم -o برنامج

# ترجمة وتشغيل
tarqeem run برنامج.ترقيم

# فحص الصياغة فقط
tarqeem check برنامج.ترقيم

# تنسيق الكود
tarqeem fmt برنامج.ترقيم

# الوضع التفاعلي
tarqeem repl

# التنقيح مع DAP
tarqeem debug برنامج.ترقيم

# توليد التوثيق
tarqeem doc برنامج.ترقيم

# إدارة الحزم
tarqeem pkg init              # إنشاء حزمة جديدة

# شرح رمز خطأ
tarqeem اشرح د٠٣٠١
```

---

## الاعتماديات

```toml
[dependencies]
# واجهة سطر الأوامر
clap = { version = "4.5", features = ["derive"] }
colored = "3.0"

# معالجة يونيكود
unicode-segmentation = "1.12"
unicode-normalization = "0.1"

# أدوات مساعدة
indexmap = "2.12"
phf = { version = "0.13", features = ["macros"] }

# التسلسل (لمدير الحزم)
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.9"

# إدارة الحزم
semver = "1.0"
sha2 = "0.10"
hex = "0.4"
flate2 = "1.0"
tar = "0.4"
dirs = "6.0"
walkdir = "2.5"

# معالجة الأخطاء
thiserror = "2.0"

# بروتوكول خادم اللغة (LSP)
tower-lsp = "0.20"
async-trait = "0.1"
tokio = { version = "1", features = ["full"] }
futures = "0.3"
dashmap = "6.0"

# الترجمة الفورية (اختياري)
cranelift-codegen = { version = "0.113", optional = true }
cranelift-frontend = { version = "0.113", optional = true }
cranelift-jit = { version = "0.113", optional = true }
cranelift-module = { version = "0.113", optional = true }
cranelift-native = { version = "0.113", optional = true }
target-lexicon = { version = "0.12", optional = true }

[dev-dependencies]
pretty_assertions = "1.4"
tempfile = "3.14"
criterion = { version = "0.5", features = ["html_reports"] }

[features]
profiling = []   # تمكين أدوات التنميط
concurrent = []  # محتبس نصوص متزامن
jit = [...]      # تمكين JIT
```

---

## استراتيجية الاختبار

| النوع | الوصف | الموقع |
|-------|-------|--------|
| **اختبارات الوحدة** | كل وحدة لها اختبارات مضمنة | `*_tests.rs` |
| **اختبارات التكامل** | ترجمة كاملة لملفات `.ترقيم` | `tests/integration/` |
| **اختبارات اللقطات** | مقارنة مخرجات شجرة الصياغة/التمثيل الوسيط | متنوع |
| **اختبارات الأداء** | قياس أداء المكونات | `benches/` |

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arabic_keywords() {
        let source = "متغير س = ٥";
        let tokens = Lexer::new(source).collect::<Vec<_>>();

        assert_eq!(tokens[0].kind, TokenKind::Mutaghayir);
        assert_eq!(tokens[1].kind, TokenKind::Identifier("س".into()));
        assert_eq!(tokens[2].kind, TokenKind::Equal);
        assert_eq!(tokens[3].kind, TokenKind::Integer(5));
    }
}
```

---

## أهداف الأداء

| المقياس | الهدف |
|---------|-------|
| **سرعة الترجمة** | < ١٠٠ مللي ثانية لـ ١٠ آلاف سطر |
| **أداء وقت التشغيل** | ضمن ٢ ضعف أداء C المكافئ |
| **استخدام الذاكرة** | < ١٠٠ ميغابايت للبرامج النموذجية |
| **وقت البدء** | < ١٠ مللي ثانية لمرحباً بالعالم |

---

## قرارات التصميم

### لماذا محلل تنازلي تكراري؟

- أسهل في التنفيذ والفهم
- رسائل خطأ واستعادة أفضل
- كافٍ لقواعدنا النحوية (ليست غامضة جداً)
- موسَّع بتحليل برات للتعبيرات

### لماذا LLVM؟

- واجهة خلفية ناضجة ومُختبَرة
- تمريرات تحسين ممتازة
- توليد كود متعدد المنصات
- واجهة C مستقرة

### لماذا عد المراجع؟

- أبسط من جامع القمامة المتتبع
- تدمير حتمي
- زمن انتقال منخفض (لا توقفات جامع القمامة)
- يمكن تحسينه بواسطة المترجم

### لماذا مفسر متجول للشجرة؟

- يُستخدم لـ REPL والتنقيح
- أسهل في تنفيذ تنقيح مستوى المصدر
- يشغِّل خادم DAP لتنقيح VS Code

---

## ملاحظات إضافية

### دعم ويب أسمبلي

ملفات دعم ويب أسمبلي موجودة في:
- `runtime/wasm/` - وقت تشغيل WASM بلغة C
- `examples/wasm/` - أمثلة WASM

### الميزات الاختيارية

| الميزة | الوصف |
|--------|-------|
| `profiling` | أدوات التنميط |
| `concurrent` | محتبس نصوص متزامن |
| `jit` | الترجمة الفورية عبر Cranelift |

---

**ترقيم** - لغة البرمجة العربية الأصيلة

</div>
