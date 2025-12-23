# خطة تنفيذ الخواص (Property Accessors) في ترقيم

## الفلسفة والتصميم

### المبدأ الأساسي

وفقاً لـ `arabic-philosophy.md`:
- **الوصف لا الترجمة**: لا نترجم `getter/setter` حرفياً
- **الصحة النحوية**: الصياغة تُقرأ كجملة عربية صحيحة
- **الترتيب العربي**: الترتيب يتبع قواعد اللغة العربية
- **الاكتمال الذاتي**: لا اختصارات غامضة

---

## اختيار الكلمات المفتاحية

### تحليل المصطلحات

| الإنجليزية | الترجمة الحرفية | المصطلح المختار | السبب |
|------------|----------------|-----------------|-------|
| property | ملكية | **خاصية** | الخاصية تصف صفة الكائن، الملكية للحقوق القانونية |
| getter | مُحصِّل | **احصل** | فعل أمر واضح ومباشر |
| setter | مُعيِّن | **عيّن** | فعل أمر واضح ومباشر |
| backing field | حقل داعم | **_اسم** (بادئة شرطة) | نمط شائع ومفهوم |

### الكلمات المفتاحية الجديدة

```rust
// في src/lexer/keywords.rs
"خاصية" => TokenKind::Property,    // property declaration
"احصل" => TokenKind::Get,          // getter block
"عيّن" => TokenKind::Set,          // setter block
```

---

## الصياغة المقترحة

### الخيار ١: خاصية مع كتل الوصول (مُفضّل)

```tarqeem
صنف شخص {
    // حقل داعم خاص
    خاص _اسم: نص = ""
    خاص _عمر: عدد = 0

    // خاصية كاملة مع احصل وعيّن
    خاصية اسم: نص {
        احصل {
            أرجع هذا._اسم
        }
        عيّن(قيمة) {
            إذا (قيمة.طول() > 0) {
                هذا._اسم = قيمة
            }
        }
    }

    // خاصية للقراءة فقط
    خاصية عمر: عدد {
        احصل {
            أرجع هذا._عمر
        }
    }

    // خاصية محسوبة (computed property)
    خاصية وصف: نص {
        احصل {
            أرجع هذا._اسم + " - " + هذا._عمر
        }
    }

    // خاصية تلقائية (auto-property)
    خاصية معرّف: نص
}

// الاستخدام
متغير ش = جديد شخص()
ش.اسم = "أحمد"        // يستدعي عيّن تلقائياً
اطبع(ش.اسم)           // يستدعي احصل تلقائياً
```

### الخيار ٢: خاصية مع تحديد الرؤية

```tarqeem
صنف حساب {
    خاص _رصيد: عدد_عشري = 0.0

    // خاصية مع رؤية مختلفة للقراءة والكتابة
    خاصية رصيد: عدد_عشري {
        عام احصل {
            أرجع هذا._رصيد
        }
        خاص عيّن(قيمة) {
            هذا._رصيد = قيمة
        }
    }
}
```

### الخيار ٣: خاصية مختصرة

```tarqeem
صنف نقطة {
    // خاصية تلقائية مع قيمة افتراضية
    خاصية س: عدد = 0
    خاصية ص: عدد = 0

    // خاصية للقراءة فقط (بدون عيّن)
    خاصية مسافة: عدد_عشري {
        احصل => جذر(هذا.س * هذا.س + هذا.ص * هذا.ص)
    }
}
```

---

## التغييرات التقنية المطلوبة

### المرحلة ١: الـ Lexer

#### ملف: `src/lexer/token.rs`

```rust
// إضافة TokenKind جديدة
pub enum TokenKind {
    // ... existing tokens ...

    // Property accessors
    Property,    // خاصية
    Get,         // احصل
    Set,         // عيّن
}
```

#### ملف: `src/lexer/keywords.rs`

```rust
// إضافة الكلمات المفتاحية
pub fn get_keyword(word: &str) -> Option<TokenKind> {
    match word {
        // ... existing keywords ...

        // Property accessors
        "خاصية" => Some(TokenKind::Property),
        "احصل" => Some(TokenKind::Get),
        "عيّن" => Some(TokenKind::Set),

        _ => None,
    }
}
```

---

### المرحلة ٢: الـ AST

#### ملف: `src/parser/ast.rs`

```rust
/// Property accessor kind
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyAccessor {
    /// Get accessor: احصل { ... }
    Get {
        visibility: Visibility,
        body: Block,
    },
    /// Set accessor: عيّن(قيمة) { ... }
    Set {
        visibility: Visibility,
        param_name: String,  // Usually "قيمة" (value)
        body: Block,
    },
}

/// Property declaration
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyDecl {
    pub name: String,
    pub ty: TypeAnnotation,
    pub visibility: Visibility,
    pub is_static: bool,
    pub accessors: Vec<PropertyAccessor>,
    pub default_value: Option<Expr>,
    pub doc_comment: Option<String>,
}

// تحديث ClassMember
pub enum ClassMember {
    Field { ... },
    Method { ... },
    Constructor { ... },
    Property(PropertyDecl),  // جديد
}
```

---

### المرحلة ٣: الـ Parser

#### ملف: `src/parser/parser.rs`

```rust
/// Parse a property declaration
/// Grammar: خاصية <name>: <type> { <accessors> }
///      or: خاصية <name>: <type> = <default>
///      or: خاصية <name>: <type>
fn parse_property_declaration(&mut self, visibility: Visibility, is_static: bool) -> ParseResult<ClassMember> {
    // Consume خاصية keyword
    self.advance();

    // Property name
    let name = self.expect_identifier(
        "متوقع اسم الخاصية",
        "Expected property name"
    )?;

    // Type annotation (required for properties)
    self.expect(&TokenKind::Colon, "متوقع ':' بعد اسم الخاصية", "Expected ':' after property name")?;
    let ty = self.parse_type_annotation()?;

    // Check for accessor block, default value, or auto-property
    let (accessors, default_value) = if self.match_token(&TokenKind::LeftBrace) {
        // Property with accessor block
        let accessors = self.parse_property_accessors()?;
        self.expect(&TokenKind::RightBrace, "متوقع '}'", "Expected '}'")?;
        (accessors, None)
    } else if self.match_token(&TokenKind::Equal) {
        // Auto-property with default value
        let default = self.parse_expression()?;
        self.consume_semicolon()?;
        (self.generate_auto_accessors(), Some(default))
    } else {
        // Auto-property without default
        self.consume_semicolon()?;
        (self.generate_auto_accessors(), None)
    };

    Ok(ClassMember::Property(PropertyDecl {
        name,
        ty,
        visibility,
        is_static,
        accessors,
        default_value,
        doc_comment: None,
    }))
}

/// Parse property accessors (احصل and عيّن blocks)
fn parse_property_accessors(&mut self) -> ParseResult<Vec<PropertyAccessor>> {
    let mut accessors = Vec::new();

    while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
        // Optional visibility modifier
        let accessor_visibility = self.parse_visibility();

        if self.match_token(&TokenKind::Get) {
            // احصل block
            let body = if self.match_token(&TokenKind::FatArrow) {
                // Short form: احصل => expression
                let expr = self.parse_expression()?;
                Block { statements: vec![Stmt::Return(Some(expr))] }
            } else {
                // Full form: احصل { ... }
                self.parse_block()?
            };

            accessors.push(PropertyAccessor::Get {
                visibility: accessor_visibility,
                body,
            });
        } else if self.match_token(&TokenKind::Set) {
            // عيّن block
            // Optional parameter name: عيّن(قيمة) or just عيّن
            let param_name = if self.match_token(&TokenKind::LeftParen) {
                let name = self.expect_identifier("متوقع اسم المعامل", "Expected parameter name")?;
                self.expect(&TokenKind::RightParen, "متوقع ')'", "Expected ')'")?;
                name
            } else {
                "قيمة".to_string()  // Default parameter name
            };

            let body = self.parse_block()?;

            accessors.push(PropertyAccessor::Set {
                visibility: accessor_visibility,
                param_name,
                body,
            });
        } else {
            return Err(self.error(
                "متوقع 'احصل' أو 'عيّن'",
                "Expected 'احصل' (get) or 'عيّن' (set)"
            ));
        }
    }

    Ok(accessors)
}

/// Generate automatic accessors for auto-properties
fn generate_auto_accessors(&self) -> Vec<PropertyAccessor> {
    // Will be handled during semantic analysis / lowering
    // For now, return empty to indicate auto-property
    vec![]
}
```

---

### المرحلة ٤: التحليل الدلالي

#### ملف: `src/semantic/class_resolver.rs`

```rust
/// Property info for resolved classes
pub struct PropertyInfo {
    pub name: String,
    pub ty: Type,
    pub visibility: Visibility,
    pub is_static: bool,
    pub has_getter: bool,
    pub has_setter: bool,
    pub getter_visibility: Visibility,
    pub setter_visibility: Visibility,
    pub is_auto: bool,  // Auto-property generates backing field
}

// Update ClassInfo
pub struct ClassInfo {
    // ... existing fields ...
    pub properties: IndexMap<String, PropertyInfo>,
}
```

#### ملف: `src/semantic/analyzer.rs`

```rust
/// Analyze property declaration
fn analyze_property(&mut self, prop: &PropertyDecl, class_name: &str) -> Result<(), SemanticError> {
    let ty = self.resolve_type(&prop.ty)?;

    // Check accessor consistency
    let has_getter = prop.accessors.iter().any(|a| matches!(a, PropertyAccessor::Get { .. }));
    let has_setter = prop.accessors.iter().any(|a| matches!(a, PropertyAccessor::Set { .. }));

    // Auto-property: generate backing field
    if prop.accessors.is_empty() {
        let backing_field = format!("_{}", prop.name);
        self.add_synthetic_field(class_name, &backing_field, ty.clone(), Visibility::Private)?;
    }

    // Analyze accessor bodies
    for accessor in &prop.accessors {
        match accessor {
            PropertyAccessor::Get { body, .. } => {
                // Return type must match property type
                self.analyze_block_with_expected_return(body, &ty)?;
            }
            PropertyAccessor::Set { param_name, body, .. } => {
                // Add parameter to scope
                self.scope.define(param_name.clone(), ty.clone())?;
                self.analyze_block(body)?;
            }
        }
    }

    Ok(())
}
```

---

### المرحلة ٥: توليد الكود الوسيط (IR)

#### ملف: `src/ir/builder.rs`

Properties are lowered to synthetic methods:

```rust
/// Lower property to getter/setter methods
fn lower_property(&mut self, prop: &PropertyDecl, class_name: &str) -> Vec<IRFunction> {
    let mut functions = Vec::new();

    // Generate getter method: __get_<property_name>
    if let Some(getter) = prop.accessors.iter().find(|a| matches!(a, PropertyAccessor::Get { .. })) {
        functions.push(self.generate_getter_method(prop, getter, class_name));
    }

    // Generate setter method: __set_<property_name>
    if let Some(setter) = prop.accessors.iter().find(|a| matches!(a, PropertyAccessor::Set { .. })) {
        functions.push(self.generate_setter_method(prop, setter, class_name));
    }

    // For auto-properties, generate default implementations
    if prop.accessors.is_empty() {
        functions.push(self.generate_auto_getter(prop, class_name));
        functions.push(self.generate_auto_setter(prop, class_name));
    }

    functions
}
```

---

### المرحلة ٦: توليد كود LLVM

#### ملف: `src/codegen/llvm.rs`

```rust
/// Handle property access in expressions
fn compile_member_access(&mut self, object: &Expr, member: &str) -> Result<LLVMValue> {
    let obj_val = self.compile_expr(object)?;
    let class_info = self.get_class_info(object)?;

    // Check if member is a property
    if let Some(prop_info) = class_info.properties.get(member) {
        // Call getter method
        let getter_name = format!("__get_{}", member);
        self.compile_method_call(obj_val, &getter_name, &[])
    } else {
        // Direct field access
        self.compile_field_access(obj_val, member)
    }
}

/// Handle property assignment
fn compile_assignment(&mut self, target: &Expr, value: &Expr) -> Result<LLVMValue> {
    if let Expr::Member { object, property } = target {
        let obj_val = self.compile_expr(object)?;
        let class_info = self.get_class_info(object)?;

        // Check if target is a property
        if let Some(prop_info) = class_info.properties.get(property) {
            if !prop_info.has_setter {
                return Err(self.error(
                    format!("الخاصية '{}' للقراءة فقط", property),
                    format!("Property '{}' is read-only", property)
                ));
            }

            // Call setter method
            let setter_name = format!("__set_{}", property);
            let val = self.compile_expr(value)?;
            return self.compile_method_call(obj_val, &setter_name, &[val]);
        }
    }

    // Regular assignment
    self.compile_regular_assignment(target, value)
}
```

---

## رسائل الخطأ (ثنائية اللغة)

| الحالة | العربية | English |
|--------|---------|---------|
| خاصية بدون نوع | متوقع تحديد نوع الخاصية | Expected property type annotation |
| كتابة لخاصية للقراءة فقط | الخاصية '{name}' للقراءة فقط | Property '{name}' is read-only |
| قراءة من خاصية للكتابة فقط | الخاصية '{name}' للكتابة فقط | Property '{name}' is write-only |
| تكرار الـ احصل | لا يمكن تعريف أكثر من 'احصل' واحد | Cannot define more than one getter |
| تكرار الـ عيّن | لا يمكن تعريف أكثر من 'عيّن' واحد | Cannot define more than one setter |
| نوع الإرجاع لا يطابق | نوع إرجاع 'احصل' لا يطابق نوع الخاصية | Getter return type doesn't match property type |

---

## الاختبارات المطلوبة

### اختبارات الـ Lexer

```rust
#[test]
fn test_property_keywords() {
    let tokens = tokenize("خاصية احصل عيّن");
    assert_eq!(tokens[0].kind, TokenKind::Property);
    assert_eq!(tokens[1].kind, TokenKind::Get);
    assert_eq!(tokens[2].kind, TokenKind::Set);
}
```

### اختبارات الـ Parser

```rust
#[test]
fn test_parse_full_property() {
    let source = r#"
        صنف شخص {
            خاص _اسم: نص

            خاصية اسم: نص {
                احصل {
                    أرجع هذا._اسم
                }
                عيّن(قيمة) {
                    هذا._اسم = قيمة
                }
            }
        }
    "#;

    let ast = parse(source).unwrap();
    // Assert property is parsed correctly
}

#[test]
fn test_parse_auto_property() {
    let source = r#"
        صنف نقطة {
            خاصية س: عدد = 0
            خاصية ص: عدد
        }
    "#;

    let ast = parse(source).unwrap();
    // Assert auto-properties are parsed
}

#[test]
fn test_parse_readonly_property() {
    let source = r#"
        صنف دائرة {
            خاص _نصف_قطر: عدد_عشري

            خاصية مساحة: عدد_عشري {
                احصل => ط * هذا._نصف_قطر * هذا._نصف_قطر
            }
        }
    "#;

    let ast = parse(source).unwrap();
    // Assert readonly computed property
}
```

### اختبارات التحليل الدلالي

```rust
#[test]
fn test_property_type_checking() {
    let source = r#"
        صنف اختبار {
            خاصية س: عدد {
                احصل {
                    أرجع "نص"  // خطأ: نوع غير متطابق
                }
            }
        }
    "#;

    let result = analyze(source);
    assert!(result.is_err());
}

#[test]
fn test_readonly_property_assignment() {
    let source = r#"
        صنف اختبار {
            خاصية س: عدد {
                احصل { أرجع 5 }
            }
        }

        متغير ت = جديد اختبار()
        ت.س = 10  // خطأ: الخاصية للقراءة فقط
    "#;

    let result = analyze(source);
    assert!(result.is_err());
}
```

---

## خطة التنفيذ

### الترتيب المقترح

| # | المهمة | الملفات | الأولوية | الوقت المقدر |
|---|--------|---------|----------|--------------|
| 1 | إضافة التوكنات الجديدة | `token.rs`, `keywords.rs` | عالية | صغير |
| 2 | تعريف الـ AST | `ast.rs` | عالية | صغير |
| 3 | تنفيذ الـ Parser | `parser.rs` | عالية | متوسط |
| 4 | اختبارات الـ Parser | `parser_tests.rs` | عالية | صغير |
| 5 | تحديث ClassResolver | `class_resolver.rs` | عالية | متوسط |
| 6 | التحليل الدلالي | `analyzer.rs` | عالية | متوسط |
| 7 | اختبارات دلالية | `semantic_tests.rs` | متوسطة | صغير |
| 8 | توليد IR | `builder.rs` | عالية | متوسط |
| 9 | توليد LLVM | `llvm.rs` | عالية | متوسط |
| 10 | اختبارات تكامل | `integration/` | متوسطة | صغير |
| 11 | تحديث التوثيق | `README.md`, `docs/` | منخفضة | صغير |

---

## أمثلة استخدام كاملة

### مثال ١: صنف مع خواص متعددة

```tarqeem
صنف موظف {
    // حقول داعمة خاصة
    خاص _اسم: نص
    خاص _راتب: عدد_عشري
    خاص _معرّف: عدد

    // منشئ
    منشئ(اسم: نص، راتب: عدد_عشري) {
        هذا._اسم = اسم
        هذا._راتب = راتب
        هذا._معرّف = توليد_معرّف()
    }

    // خاصية للقراءة والكتابة مع تحقق
    خاصية اسم: نص {
        احصل {
            أرجع هذا._اسم
        }
        عيّن(قيمة) {
            إذا (قيمة.طول() >= 2) {
                هذا._اسم = قيمة
            }
        }
    }

    // خاصية مع رؤية مختلفة
    خاصية راتب: عدد_عشري {
        عام احصل {
            أرجع هذا._راتب
        }
        محمي عيّن(قيمة) {
            إذا (قيمة > 0) {
                هذا._راتب = قيمة
            }
        }
    }

    // خاصية للقراءة فقط
    خاصية معرّف: عدد {
        احصل {
            أرجع هذا._معرّف
        }
    }

    // خاصية محسوبة
    خاصية راتب_سنوي: عدد_عشري {
        احصل => هذا._راتب * 12
    }
}

// الاستخدام
متغير م = جديد موظف("أحمد", 5000.0)
اطبع(م.اسم)              // أحمد
اطبع(م.راتب_سنوي)        // 60000.0
م.اسم = "محمد"           // تحديث الاسم
اطبع(م.معرّف)            // قراءة المعرّف (لا يمكن تغييره)
```

### مثال ٢: خاصية تلقائية بسيطة

```tarqeem
صنف نقطة {
    خاصية س: عدد = 0
    خاصية ص: عدد = 0

    منشئ(س: عدد، ص: عدد) {
        هذا.س = س
        هذا.ص = ص
    }

    خاصية مسافة_من_المركز: عدد_عشري {
        احصل => جذر(هذا.س * هذا.س + هذا.ص * هذا.ص)
    }
}

متغير ن = جديد نقطة(3، 4)
اطبع(ن.مسافة_من_المركز)  // 5.0
```

---

## الخلاصة

هذه الخطة تقدم نظام خواص (properties) متكامل يتبع فلسفة اللغة العربية:

1. **خاصية** بدلاً من property - تصف صفة الكائن
2. **احصل** بدلاً من get - فعل أمر واضح
3. **عيّن** بدلاً من set - فعل أمر واضح
4. صياغة تُقرأ بشكل طبيعي في العربية
5. دعم الخواص التلقائية والمحسوبة والمخصصة
