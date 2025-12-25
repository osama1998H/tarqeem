//! Comprehensive tests for the Parser module
//!
//! These tests verify correct parsing of all Tarqeem syntax constructs
//! including loops, match statements, imports/exports, and complex expressions.

use super::ast::*;
use super::parser::Parser;

fn wrap_with_markers(source: &str) -> String {
    format!("بسم_الله\n{}\nالحمد_لله", source.trim())
}

fn parser_with_markers(source: &str) -> Parser {
    Parser::new(&wrap_with_markers(source))
}

#[test]
fn test_parse_while_loop() {
    let source = r#"
        طالما (س < 10) {
            س = س + 1;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    assert_eq!(ast.statements.len(), 1);
    match &ast.statements[0].kind {
        StmtKind::While { condition, body } => {
            assert!(!body.statements.is_empty());
            match &condition.kind {
                ExprKind::Binary { op, .. } => assert_eq!(*op, BinaryOp::Lt),
                _ => panic!("Expected binary expression"),
            }
        }
        _ => panic!("Expected While statement"),
    }
}

#[test]
fn test_parse_while_loop_arabic_alt() {
    let source = r#"
        طالما (ص < 10) {
            ص = ص + 1;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    assert_eq!(ast.statements.len(), 1);
    assert!(matches!(&ast.statements[0].kind, StmtKind::While { .. }));
}

#[test]
fn test_parse_for_loop_c_style() {
    let source = r#"
        لكل (متغير ع = 0؛ ع < 10؛ ع++) {
            اطبع(ع);
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    assert_eq!(ast.statements.len(), 1);
    match &ast.statements[0].kind {
        StmtKind::For {
            init,
            condition,
            update,
            body,
        } => {
            assert!(init.is_some());
            assert!(condition.is_some());
            assert!(update.is_some());
            assert!(!body.statements.is_empty());
        }
        _ => panic!("Expected For statement"),
    }
}

#[test]
fn test_parse_for_loop_arabic_alt() {
    let source = r#"
        لكل (متغير م = 0؛ م < 10؛ م++) {
            اطبع(م);
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    assert_eq!(ast.statements.len(), 1);
    assert!(matches!(&ast.statements[0].kind, StmtKind::For { .. }));
}

#[test]
fn test_parse_for_in_loop() {
    let source = r#"
        لكل عنصر في قائمة_أرقام {
            اطبع(عنصر);
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    assert_eq!(ast.statements.len(), 1);
    match &ast.statements[0].kind {
        StmtKind::ForIn {
            variable,
            iterable,
            body,
        } => {
            assert_eq!(variable, "عنصر");
            assert!(!body.statements.is_empty());
            match &iterable.kind {
                ExprKind::Identifier(name) => assert_eq!(name, "قائمة_أرقام"),
                _ => panic!("Expected identifier"),
            }
        }
        _ => panic!("Expected ForIn statement"),
    }
}

#[test]
fn test_parse_for_in_loop_arabic_alt() {
    let source = r#"
        لكل بند في بنود {
            اطبع(بند);
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    assert_eq!(ast.statements.len(), 1);
    match &ast.statements[0].kind {
        StmtKind::ForIn { variable, .. } => {
            assert_eq!(variable, "بند");
        }
        _ => panic!("Expected ForIn statement"),
    }
}

#[test]
fn test_parse_for_loop_no_init() {
    let source = r#"
        لكل (؛ ع < 10؛ ع++) {
            س = س + 1;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::For {
            init,
            condition,
            update,
            ..
        } => {
            assert!(init.is_none());
            assert!(condition.is_some());
            assert!(update.is_some());
        }
        _ => panic!("Expected For statement"),
    }
}

#[test]
fn test_parse_for_loop_no_update() {
    let source = r#"
        لكل (متغير ع = 0؛ ع < 10؛) {
            ع = ع + 2;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::For {
            init,
            condition,
            update,
            ..
        } => {
            assert!(init.is_some());
            assert!(condition.is_some());
            assert!(update.is_none());
        }
        _ => panic!("Expected For statement"),
    }
}

#[test]
fn test_parse_match_statement() {
    let source = r#"
        تطابق (يوم) {
            حالة 1 => اطبع("الأحد")
            حالة 2 => اطبع("الاثنين")
            غير_ذلك => اطبع("يوم آخر")
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    assert_eq!(ast.statements.len(), 1);
    match &ast.statements[0].kind {
        StmtKind::Match { expr, arms } => {
            assert_eq!(arms.len(), 3);
            match &expr.kind {
                ExprKind::Identifier(name) => assert_eq!(name, "يوم"),
                _ => panic!("Expected identifier"),
            }
        }
        _ => panic!("Expected Match statement"),
    }
}

#[test]
fn test_parse_match_with_multiple_patterns() {
    let source = r#"
        تطابق (يوم) {
            حالة 1، 2، 3 => اطبع("يوم عمل")
            حالة 6، 7 => اطبع("عطلة")
            غير_ذلك => اطبع("غير معروف")
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Match { arms, .. } => {
            assert_eq!(arms.len(), 3);
            assert_eq!(arms[0].patterns.len(), 3);
            assert_eq!(arms[1].patterns.len(), 2);
        }
        _ => panic!("Expected Match statement"),
    }
}

#[test]
fn test_parse_match_with_block_body() {
    let source = r#"
        تطابق (س) {
            حالة 1 => {
                ص = 10؛
                ع = 20؛
            }
            غير_ذلك => {
                ص = 0؛
            }
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Match { arms, .. } => {
            assert_eq!(arms[0].body.statements.len(), 2);
            assert_eq!(arms[1].body.statements.len(), 1);
        }
        _ => panic!("Expected Match statement"),
    }
}

#[test]
fn test_parse_named_import() {
    let source = r#"
        استورد { مساعد، أداة } من "مجموعات";
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Import { items, from } => {
            match items {
                ImportItems::Named(imports) => {
                    assert_eq!(imports.len(), 2);
                    assert_eq!(imports[0].name, "مساعد");
                    assert_eq!(imports[1].name, "أداة");
                }
                _ => panic!("Expected named imports"),
            }
            assert_eq!(from, "مجموعات");
        }
        _ => panic!("Expected Import statement"),
    }
}

#[test]
fn test_parse_named_import_with_alias() {
    let source = r#"
        استورد { قائمة كـ قائمتي، خريطة كـ خريطتي } من "مجموعات";
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Import { items, .. } => match items {
            ImportItems::Named(imports) => {
                assert_eq!(imports[0].name, "قائمة");
                assert_eq!(imports[0].alias, Some("قائمتي".to_string()));
                assert_eq!(imports[1].name, "خريطة");
                assert_eq!(imports[1].alias, Some("خريطتي".to_string()));
            }
            _ => panic!("Expected named imports"),
        },
        _ => panic!("Expected Import statement"),
    }
}

#[test]
fn test_parse_wildcard_import() {
    let source = r#"
        استورد * كـ رياضيات من "رياضيات";
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Import { items, from } => {
            match items {
                ImportItems::Wildcard(alias) => {
                    assert_eq!(alias, "رياضيات");
                }
                _ => panic!("Expected wildcard import"),
            }
            assert_eq!(from, "رياضيات");
        }
        _ => panic!("Expected Import statement"),
    }
}

#[test]
fn test_parse_default_import() {
    let source = r#"
        استورد وحدتي من "وحدة";
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Import { items, .. } => match items {
            ImportItems::Default(name) => {
                assert_eq!(name, "وحدتي");
            }
            _ => panic!("Expected default import"),
        },
        _ => panic!("Expected Import statement"),
    }
}

#[test]
fn test_parse_export_function() {
    let source = r#"
        صدّر دالة مساعدة() {
            أرجع 42;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Export(inner) => match &inner.kind {
            StmtKind::FuncDecl { name, .. } => {
                assert_eq!(name, "مساعدة");
            }
            _ => panic!("Expected FuncDecl inside export"),
        },
        _ => panic!("Expected Export statement"),
    }
}

#[test]
fn test_parse_export_class() {
    let source = r#"
        صدّر صنف مساعد {
            عام دالة ساعد() {
                أرجع 1;
            }
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Export(inner) => match &inner.kind {
            StmtKind::ClassDecl { name, .. } => {
                assert_eq!(name, "مساعد");
            }
            _ => panic!("Expected ClassDecl inside export"),
        },
        _ => panic!("Expected Export statement"),
    }
}

#[test]
fn test_parse_try_catch() {
    let source = r#"
        حاول {
            خطر();
        } التقط (استثناء) {
            اطبع(استثناء);
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Try {
            body,
            catch,
            finally,
        } => {
            assert!(!body.statements.is_empty());
            assert!(catch.is_some());
            assert!(finally.is_none());

            let catch_clause = catch.as_ref().unwrap();
            assert_eq!(catch_clause.param, "استثناء");
        }
        _ => panic!("Expected Try statement"),
    }
}

#[test]
fn test_parse_try_catch_finally() {
    let source = r#"
        حاول {
            خطير();
        } التقط (خطأ_م) {
            سجل(خطأ_م);
        } أخيراً {
            نظف();
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Try {
            body,
            catch,
            finally,
        } => {
            assert!(!body.statements.is_empty());
            assert!(catch.is_some());
            assert!(finally.is_some());
        }
        _ => panic!("Expected Try statement"),
    }
}

#[test]
fn test_parse_try_finally_no_catch() {
    let source = r#"
        حاول {
            نفذ();
        } أخيراً {
            نظف();
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Try { catch, finally, .. } => {
            assert!(catch.is_none());
            assert!(finally.is_some());
        }
        _ => panic!("Expected Try statement"),
    }
}

#[test]
fn test_parse_throw_statement() {
    let source = r#"
        ارمِ "خطأ في البرنامج";
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Throw(expr) => match &expr.kind {
            ExprKind::Literal(Literal::String(s)) => {
                assert_eq!(s, "خطأ في البرنامج");
            }
            _ => panic!("Expected string literal"),
        },
        _ => panic!("Expected Throw statement"),
    }
}

#[test]
fn test_parse_break_statement() {
    let source = r#"
        طالما (صحيح) {
            أوقف;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::While { body, .. } => {
            assert!(matches!(&body.statements[0].kind, StmtKind::Break));
        }
        _ => panic!("Expected While statement"),
    }
}

#[test]
fn test_parse_continue_statement() {
    let source = r#"
        طالما (صحيح) {
            استمر;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::While { body, .. } => {
            assert!(matches!(&body.statements[0].kind, StmtKind::Continue));
        }
        _ => panic!("Expected While statement"),
    }
}

#[test]
fn test_precedence_multiplication_over_addition() {
    let source = "1 + 2 * 3;";
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Expr(expr) => match &expr.kind {
            ExprKind::Binary { left, op, right } => {
                assert_eq!(*op, BinaryOp::Add);
                match &left.kind {
                    ExprKind::Literal(Literal::Int(1)) => {}
                    _ => panic!("Expected literal 1"),
                }
                match &right.kind {
                    ExprKind::Binary { op, .. } => {
                        assert_eq!(*op, BinaryOp::Mul);
                    }
                    _ => panic!("Expected multiplication"),
                }
            }
            _ => panic!("Expected binary expression"),
        },
        _ => panic!("Expected expression statement"),
    }
}

#[test]
fn test_precedence_power_right_associative() {
    let source = "2 ** 3 ** 2;";
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Expr(expr) => match &expr.kind {
            ExprKind::Binary { left, op, right } => {
                assert_eq!(*op, BinaryOp::Pow);
                match &left.kind {
                    ExprKind::Literal(Literal::Int(2)) => {}
                    _ => panic!("Expected literal 2"),
                }
                match &right.kind {
                    ExprKind::Binary { left, op, right } => {
                        assert_eq!(*op, BinaryOp::Pow);
                        match &left.kind {
                            ExprKind::Literal(Literal::Int(3)) => {}
                            _ => panic!("Expected literal 3"),
                        }
                        match &right.kind {
                            ExprKind::Literal(Literal::Int(2)) => {}
                            _ => panic!("Expected literal 2"),
                        }
                    }
                    _ => panic!("Expected power expression"),
                }
            }
            _ => panic!("Expected binary expression"),
        },
        _ => panic!("Expected expression statement"),
    }
}

#[test]
fn test_precedence_comparison_and_logical() {
    let source = "أ > ب && ج < د;";
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Expr(expr) => match &expr.kind {
            ExprKind::Binary { op, left, right } => {
                assert_eq!(*op, BinaryOp::And);
                match &left.kind {
                    ExprKind::Binary { op, .. } => assert_eq!(*op, BinaryOp::Gt),
                    _ => panic!("Expected comparison"),
                }
                match &right.kind {
                    ExprKind::Binary { op, .. } => assert_eq!(*op, BinaryOp::Lt),
                    _ => panic!("Expected comparison"),
                }
            }
            _ => panic!("Expected binary expression"),
        },
        _ => panic!("Expected expression statement"),
    }
}

#[test]
fn test_precedence_parentheses() {
    let source = "(1 + 2) * 3;";
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Expr(expr) => match &expr.kind {
            ExprKind::Binary { op, left, .. } => {
                assert_eq!(*op, BinaryOp::Mul);
                match &left.kind {
                    ExprKind::Grouping(inner) => match &inner.kind {
                        ExprKind::Binary { op, .. } => {
                            assert_eq!(*op, BinaryOp::Add);
                        }
                        _ => panic!("Expected addition inside grouping"),
                    },
                    _ => panic!("Expected grouping"),
                }
            }
            _ => panic!("Expected binary expression"),
        },
        _ => panic!("Expected expression statement"),
    }
}

#[test]
fn test_parse_ternary_expression() {
    let source = "س > 0 ? 1 : -1;";
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Expr(expr) => match &expr.kind {
            ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                match &condition.kind {
                    ExprKind::Binary { op, .. } => assert_eq!(*op, BinaryOp::Gt),
                    _ => panic!("Expected comparison"),
                }
                match &then_expr.kind {
                    ExprKind::Literal(Literal::Int(1)) => {}
                    _ => panic!("Expected literal 1"),
                }
                match &else_expr.kind {
                    ExprKind::Unary {
                        op: UnaryOp::Neg, ..
                    } => {}
                    _ => panic!("Expected negation"),
                }
            }
            _ => panic!("Expected ternary expression"),
        },
        _ => panic!("Expected expression statement"),
    }
}

#[test]
fn test_parse_member_access() {
    let source = "كائن.حقل.حقل_فرعي;";
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Expr(expr) => match &expr.kind {
            ExprKind::Member { object, property } => {
                assert_eq!(property, "حقل_فرعي");
                match &object.kind {
                    ExprKind::Member { property, .. } => {
                        assert_eq!(property, "حقل");
                    }
                    _ => panic!("Expected member access"),
                }
            }
            _ => panic!("Expected member access"),
        },
        _ => panic!("Expected expression statement"),
    }
}

#[test]
fn test_parse_index_access() {
    let source = "أرقام[0][1];";
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Expr(expr) => match &expr.kind {
            ExprKind::Index { object, index } => {
                match &index.kind {
                    ExprKind::Literal(Literal::Int(1)) => {}
                    _ => panic!("Expected literal 1"),
                }
                match &object.kind {
                    ExprKind::Index { index, .. } => match &index.kind {
                        ExprKind::Literal(Literal::Int(0)) => {}
                        _ => panic!("Expected literal 0"),
                    },
                    _ => panic!("Expected index access"),
                }
            }
            _ => panic!("Expected index access"),
        },
        _ => panic!("Expected expression statement"),
    }
}

#[test]
fn test_parse_function_call_chain() {
    let source = "أ().ب().ج();";
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Expr(expr) => match &expr.kind {
            ExprKind::Call { callee, .. } => match &callee.kind {
                ExprKind::Member { property, .. } => {
                    assert_eq!(property, "ج");
                }
                _ => panic!("Expected member access"),
            },
            _ => panic!("Expected call expression"),
        },
        _ => panic!("Expected expression statement"),
    }
}

#[test]
fn test_parse_compound_assignment() {
    let source = "س += 5;";
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Expr(expr) => match &expr.kind {
            ExprKind::CompoundAssignment { target, op, value } => {
                assert_eq!(*op, BinaryOp::Add);
                match &target.kind {
                    ExprKind::Identifier(name) => assert_eq!(name, "س"),
                    _ => panic!("Expected identifier"),
                }
                match &value.kind {
                    ExprKind::Literal(Literal::Int(5)) => {}
                    _ => panic!("Expected literal 5"),
                }
            }
            _ => panic!("Expected compound assignment"),
        },
        _ => panic!("Expected expression statement"),
    }
}

#[test]
fn test_parse_all_compound_assignments() {
    let operators = vec![
        ("س += 1;", BinaryOp::Add),
        ("س -= 1;", BinaryOp::Sub),
        ("س *= 1;", BinaryOp::Mul),
        ("س /= 1;", BinaryOp::Div),
        ("س %= 1;", BinaryOp::Mod),
    ];

    for (source, expected_op) in operators {
        let mut parser = parser_with_markers(source);
        let ast = parser.parse().unwrap();

        match &ast.statements[0].kind {
            StmtKind::Expr(expr) => match &expr.kind {
                ExprKind::CompoundAssignment { op, .. } => {
                    assert_eq!(*op, expected_op, "Failed for: {}", source);
                }
                _ => panic!("Expected compound assignment for: {}", source),
            },
            _ => panic!("Expected expression statement"),
        }
    }
}

#[test]
fn test_parse_prefix_increment() {
    let source = "++س;";
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Expr(expr) => match &expr.kind {
            ExprKind::Unary {
                op: UnaryOp::PreInc,
                ..
            } => {}
            _ => panic!("Expected prefix increment"),
        },
        _ => panic!("Expected expression statement"),
    }
}

#[test]
fn test_parse_postfix_increment() {
    let source = "س++;";
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Expr(expr) => match &expr.kind {
            ExprKind::Unary {
                op: UnaryOp::PostInc,
                ..
            } => {}
            _ => panic!("Expected postfix increment"),
        },
        _ => panic!("Expected expression statement"),
    }
}

#[test]
fn test_parse_class_with_inheritance() {
    let source = r#"
        صنف طالب يرث شخص {
            خاص معدل: عدد_عشري;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl { name, extends, .. } => {
            assert_eq!(name, "طالب");
            assert_eq!(extends, &Some("شخص".to_string()));
        }
        _ => panic!("Expected ClassDecl"),
    }
}

#[test]
fn test_parse_class_with_interface() {
    let source = r#"
        صنف كلب يلتزم حيوان {
            عام دالة تكلم() {
                اطبع("هاو");
            }
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl {
            name, implements, ..
        } => {
            assert_eq!(name, "كلب");
            assert_eq!(implements.len(), 1);
            assert_eq!(implements[0], "حيوان");
        }
        _ => panic!("Expected ClassDecl"),
    }
}

#[test]
fn test_parse_class_with_multiple_interfaces() {
    let source = r#"
        صنف صنفي يلتزم ميثاق١، ميثاق٢، ميثاق٣ {
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl { implements, .. } => {
            assert_eq!(implements.len(), 3);
        }
        _ => panic!("Expected ClassDecl"),
    }
}

#[test]
fn test_parse_interface_with_methods() {
    let source = r#"
        ميثاق قابل_للمقارنة {
            دالة قارن(آخر: قابل_للمقارنة) -> عدد
            دالة يساوي(آخر: قابل_للمقارنة) -> منطقي
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::InterfaceDecl { name, methods, .. } => {
            assert_eq!(name, "قابل_للمقارنة");
            assert_eq!(methods.len(), 2);
            assert_eq!(methods[0].name, "قارن");
            assert_eq!(methods[1].name, "يساوي");
        }
        _ => panic!("Expected InterfaceDecl"),
    }
}

#[test]
fn test_parse_class_static_members() {
    let source = r#"
        صنف عداد_م {
            مشترك قيمة: عدد;

            مشترك دالة زد() {
                عداد_م.قيمة = عداد_م.قيمة + 1;
            }
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl { members, .. } => {
            match &members[0] {
                ClassMember::Field {
                    is_static, name, ..
                } => {
                    assert!(*is_static);
                    assert_eq!(name, "قيمة");
                }
                _ => panic!("Expected static field"),
            }
            match &members[1] {
                ClassMember::Method {
                    is_static, name, ..
                } => {
                    assert!(*is_static);
                    assert_eq!(name, "زد");
                }
                _ => panic!("Expected static method"),
            }
        }
        _ => panic!("Expected ClassDecl"),
    }
}

#[test]
fn test_parse_generic_class() {
    let source = r#"
        صنف قائمة<ن> {
            خاص عناصر: مصفوفة<ن>;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl {
            name, type_params, ..
        } => {
            assert_eq!(name, "قائمة");
            assert_eq!(type_params.len(), 1);
            assert_eq!(type_params[0], "ن");
        }
        _ => panic!("Expected ClassDecl"),
    }
}

#[test]
fn test_parse_generic_class_multiple_params() {
    let source = r#"
        صنف خريطة<م، ق> {
            خاص مفاتيح: مصفوفة<م>;
            خاص قيم: مصفوفة<ق>;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl { type_params, .. } => {
            assert_eq!(type_params.len(), 2);
            assert_eq!(type_params[0], "م");
            assert_eq!(type_params[1], "ق");
        }
        _ => panic!("Expected ClassDecl"),
    }
}

#[test]
fn test_parse_generic_interface() {
    let source = r#"
        ميثاق قابل_للمقارنة<ن> {
            دالة قارن(آخر: ن) -> عدد
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::InterfaceDecl { type_params, .. } => {
            assert_eq!(type_params.len(), 1);
            assert_eq!(type_params[0], "ن");
        }
        _ => panic!("Expected InterfaceDecl"),
    }
}

#[test]
fn test_parse_new_expression_with_generics() {
    let source = r#"
        متغير قائمة = جديد قائمة<عدد>();
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { init, .. } => {
            let init_expr = init.as_ref().unwrap();
            match &init_expr.kind {
                ExprKind::New {
                    class,
                    type_args,
                    args,
                } => {
                    match &class.kind {
                        ExprKind::Identifier(name) => assert_eq!(name, "قائمة"),
                        _ => panic!("Expected identifier"),
                    }
                    assert_eq!(type_args.len(), 1);
                    assert!(args.is_empty());
                }
                _ => panic!("Expected new expression"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_async_function() {
    let source = r#"
        متوازي دالة احضر_بيانات() {
            أرجع 42;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { is_async, name, .. } => {
            assert!(*is_async);
            assert_eq!(name, "احضر_بيانات");
        }
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_await_expression() {
    let source = r#"
        متوازي دالة احضر_بيانات() {
            متغير بيانات = انتظر احصل_بيانات();
            أرجع بيانات;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, is_async, .. } => {
            assert!(*is_async);
            match &body.statements[0].kind {
                StmtKind::VarDecl { init, .. } => {
                    let init_expr = init.as_ref().unwrap();
                    assert!(matches!(&init_expr.kind, ExprKind::Await(_)));
                }
                _ => panic!("Expected VarDecl"),
            }
        }
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_object_literal() {
    let source = r#"
        متغير شخص = { اسم: "أحمد"، عمر: 25 };
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { init, .. } => {
            let init_expr = init.as_ref().unwrap();
            match &init_expr.kind {
                ExprKind::Object(pairs) => {
                    assert_eq!(pairs.len(), 2);
                    assert_eq!(pairs[0].0, "اسم");
                    assert_eq!(pairs[1].0, "عمر");
                }
                _ => panic!("Expected object literal"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_empty_object_literal() {
    let source = "متغير كائن = {};";
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { init, .. } => {
            let init_expr = init.as_ref().unwrap();
            match &init_expr.kind {
                ExprKind::Object(pairs) => {
                    assert!(pairs.is_empty());
                }
                _ => panic!("Expected object literal"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_optional_type() {
    let source = r#"
        متغير اسم: نص?;
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { ty, .. } => {
            let type_ann = ty.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Optional(_) => {}
                _ => panic!("Expected optional type"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_generic_type() {
    let source = r#"
        متغير عناصر: مصفوفة<عدد>;
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { ty, .. } => {
            let type_ann = ty.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Generic { base, args } => {
                    assert_eq!(base, "مصفوفة");
                    assert_eq!(args.len(), 1);
                }
                _ => panic!("Expected generic type"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_nested_generic_type() {
    let source = r#"
        متغير بيانات: خريطة<نص، مصفوفة<عدد>>;
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { ty, .. } => {
            let type_ann = ty.as_ref().unwrap();
            match &type_ann.kind {
                TypeKind::Generic { base, args } => {
                    assert_eq!(base, "خريطة");
                    assert_eq!(args.len(), 2);
                    match &args[1].kind {
                        TypeKind::Generic { base, .. } => {
                            assert_eq!(base, "مصفوفة");
                        }
                        _ => panic!("Expected nested generic type"),
                    }
                }
                _ => panic!("Expected generic type"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_this_expression() {
    let source = r#"
        صنف شخص {
            خاص اسم: نص;

            عام دالة احصل_اسم() -> نص {
                أرجع هذا.اسم;
            }
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl { members, .. } => match &members[1] {
            ClassMember::Method { body, .. } => match &body.statements[0].kind {
                StmtKind::Return(Some(expr)) => match &expr.kind {
                    ExprKind::Member { object, property } => {
                        assert!(matches!(&object.kind, ExprKind::This));
                        assert_eq!(property, "اسم");
                    }
                    _ => panic!("Expected member access"),
                },
                _ => panic!("Expected return"),
            },
            _ => panic!("Expected method"),
        },
        _ => panic!("Expected ClassDecl"),
    }
}

#[test]
fn test_parse_super_expression() {
    let source = r#"
        صنف طالب يرث شخص {
            منشئ(اسم: نص) {
                الأصل(اسم);
            }
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl { members, .. } => match &members[0] {
            ClassMember::Constructor { body, .. } => match &body.statements[0].kind {
                StmtKind::Expr(expr) => match &expr.kind {
                    ExprKind::Call { callee, args } => {
                        assert!(matches!(&callee.kind, ExprKind::Super));
                        assert_eq!(args.len(), 1);
                    }
                    _ => panic!("Expected call"),
                },
                _ => panic!("Expected expression"),
            },
            _ => panic!("Expected constructor"),
        },
        _ => panic!("Expected ClassDecl"),
    }
}

#[test]
fn test_parse_visibility_modifiers() {
    let source = r#"
        صنف صنفي {
            عام س: عدد;
            خاص ص: عدد;
            محمي ع: عدد;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl { members, .. } => {
            match &members[0] {
                ClassMember::Field { visibility, .. } => {
                    assert_eq!(*visibility, Visibility::Public);
                }
                _ => panic!("Expected field"),
            }
            match &members[1] {
                ClassMember::Field { visibility, .. } => {
                    assert_eq!(*visibility, Visibility::Private);
                }
                _ => panic!("Expected field"),
            }
            match &members[2] {
                ClassMember::Field { visibility, .. } => {
                    assert_eq!(*visibility, Visibility::Protected);
                }
                _ => panic!("Expected field"),
            }
        }
        _ => panic!("Expected ClassDecl"),
    }
}

#[test]
fn test_parse_empty_block() {
    let source = r#"
        دالة فارغة() {
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, .. } => {
            assert!(body.statements.is_empty());
        }
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_semicolon_insertion() {
    let source = r#"
        متغير س = 1
        متغير ص = 2
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    assert_eq!(ast.statements.len(), 2);
}

#[test]
fn test_parse_return_without_value() {
    let source = r#"
        دالة بدون_قيمة() {
            أرجع;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, .. } => match &body.statements[0].kind {
            StmtKind::Return(value) => {
                assert!(value.is_none());
            }
            _ => panic!("Expected return"),
        },
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_function_with_default_params() {
    let source = r#"
        دالة رحب(اسم: نص = "العالم") {
            اطبع("مرحبا، " + اسم);
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { params, .. } => {
            assert_eq!(params.len(), 1);
            assert!(params[0].default.is_some());
        }
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_const_declaration() {
    let source = "ثابت باي = 3.14159;";
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { mutable, name, .. } => {
            assert!(!*mutable);
            assert_eq!(name, "باي");
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_if_else_if() {
    let source = r#"
        إذا (س > 10) {
            ص = 1;
        } وإلا إذا (س > 5) {
            ص = 2;
        } وإلا إذا (س > 0) {
            ص = 3;
        } وإلا {
            ص = 0;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::If { else_branch, .. } => {
            assert!(else_branch.is_some());
            let else_block = else_branch.as_ref().unwrap();
            assert_eq!(else_block.statements.len(), 1);
            assert!(matches!(
                &else_block.statements[0].kind,
                StmtKind::If { .. }
            ));
        }
        _ => panic!("Expected If statement"),
    }
}

#[test]
fn test_parse_do_while_loop_arabic() {
    let source = r#"
        افعل {
            عداد = عداد + 1;
        } طالما (عداد < 5)
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::DoWhile { body, condition } => {
            assert!(!body.statements.is_empty());
            match &condition.kind {
                ExprKind::Binary { op, .. } => assert_eq!(*op, BinaryOp::Lt),
                _ => panic!("Expected binary expression"),
            }
        }
        _ => panic!("Expected DoWhile statement"),
    }
}

#[test]
fn test_parse_do_while_loop_arabic_alt() {
    let source = r#"
        افعل {
            عداد_م = عداد_م + 1;
        } طالما (عداد_م < 5)
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    assert!(matches!(&ast.statements[0].kind, StmtKind::DoWhile { .. }));
}

#[test]
fn test_parse_do_while_with_break() {
    let source = r#"
        افعل {
            إذا (شرط) {
                أوقف;
            }
        } طالما (صحيح)
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::DoWhile { body, .. } => {
            assert!(matches!(&body.statements[0].kind, StmtKind::If { .. }));
        }
        _ => panic!("Expected DoWhile statement"),
    }
}

#[test]
fn test_parse_do_while_with_semicolon() {
    let source = r#"
        افعل {
            س++;
        } طالما (س < 5);
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    assert!(matches!(&ast.statements[0].kind, StmtKind::DoWhile { .. }));
}

#[test]
fn test_parse_nested_do_while() {
    let source = r#"
        افعل {
            افعل {
                س++;
            } طالما (س < 5)
        } طالما (ص < 10)
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::DoWhile { body, .. } => {
            assert!(matches!(&body.statements[0].kind, StmtKind::DoWhile { .. }));
        }
        _ => panic!("Expected DoWhile statement"),
    }
}

#[test]
fn test_parse_arrow_function_single_param() {
    let source = r#"
        ثابت مربع = (س) => س * س;
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { name, init, .. } => {
            assert_eq!(name, "مربع");
            let init = init.as_ref().expect("Expected initializer");
            match &init.kind {
                ExprKind::Lambda { params, body } => {
                    assert_eq!(params.len(), 1);
                    assert_eq!(params[0].name, "س");
                    assert!(matches!(body, LambdaBody::Expr(_)));
                }
                _ => panic!("Expected Lambda expression"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_arrow_function_multiple_params() {
    let source = r#"
        ثابت جمع = (أ، ب) => أ + ب;
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { init, .. } => {
            let init = init.as_ref().expect("Expected initializer");
            match &init.kind {
                ExprKind::Lambda { params, .. } => {
                    assert_eq!(params.len(), 2);
                    assert_eq!(params[0].name, "أ");
                    assert_eq!(params[1].name, "ب");
                }
                _ => panic!("Expected Lambda expression"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_arrow_function_empty_params() {
    let source = r#"
        ثابت احصل_قيمة = () => 42;
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { init, .. } => {
            let init = init.as_ref().expect("Expected initializer");
            match &init.kind {
                ExprKind::Lambda { params, body } => {
                    assert_eq!(params.len(), 0);
                    match body {
                        LambdaBody::Expr(expr) => match &expr.kind {
                            ExprKind::Literal(Literal::Int(42)) => {}
                            _ => panic!("Expected literal 42"),
                        },
                        _ => panic!("Expected expression body"),
                    }
                }
                _ => panic!("Expected Lambda expression"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_arrow_function_block_body() {
    let source = r#"
        ثابت معقدة = (س) => {
            متغير نتيجة = س * 2;
            أرجع نتيجة;
        };
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { init, .. } => {
            let init = init.as_ref().expect("Expected initializer");
            match &init.kind {
                ExprKind::Lambda { params, body } => {
                    assert_eq!(params.len(), 1);
                    assert!(matches!(body, LambdaBody::Block(_)));
                }
                _ => panic!("Expected Lambda expression"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_arrow_function_typed_params() {
    let source = r#"
        ثابت جمع = (أ: عدد، ب: عدد) => أ + ب;
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { init, .. } => {
            let init = init.as_ref().expect("Expected initializer");
            match &init.kind {
                ExprKind::Lambda { params, .. } => {
                    assert_eq!(params.len(), 2);
                    assert!(params[0].ty.is_some());
                    assert!(params[1].ty.is_some());
                }
                _ => panic!("Expected Lambda expression"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_arrow_function_arabic_alt() {
    let source = r#"
        ثابت جمع_م = (أ_م، ب_م) => أ_م + ب_م;
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { name, init, .. } => {
            assert_eq!(name, "جمع_م");
            let init = init.as_ref().expect("Expected initializer");
            match &init.kind {
                ExprKind::Lambda { params, .. } => {
                    assert_eq!(params.len(), 2);
                    assert_eq!(params[0].name, "أ_م");
                    assert_eq!(params[1].name, "ب_م");
                }
                _ => panic!("Expected Lambda expression"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_arrow_function_nested() {
    let source = r#"
        ثابت مكرر = (س) => (ص) => س + ص;
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { init, .. } => {
            let init = init.as_ref().expect("Expected initializer");
            match &init.kind {
                ExprKind::Lambda { params, body } => {
                    assert_eq!(params.len(), 1);
                    assert_eq!(params[0].name, "س");
                    match body {
                        LambdaBody::Expr(expr) => match &expr.kind {
                            ExprKind::Lambda {
                                params: inner_params,
                                ..
                            } => {
                                assert_eq!(inner_params.len(), 1);
                                assert_eq!(inner_params[0].name, "ص");
                            }
                            _ => panic!("Expected inner Lambda expression"),
                        },
                        _ => panic!("Expected expression body"),
                    }
                }
                _ => panic!("Expected Lambda expression"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_grouping_not_arrow_function() {
    let source = r#"
        (5 + 3) * 2;
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Expr(expr) => match &expr.kind {
            ExprKind::Binary { op, left, .. } => {
                assert_eq!(*op, BinaryOp::Mul);
                assert!(matches!(&left.kind, ExprKind::Grouping(_)));
            }
            _ => panic!("Expected binary expression"),
        },
        _ => panic!("Expected expression statement"),
    }
}

#[test]
fn test_error_recovery_multiple_errors_in_block() {
    let source = r#"
        دالة اختبار() {
            متغير = 5;
            متغير ص = 10;
            ثابت = 20;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let result = parser.parse();

    assert!(result.is_err());
    let errors = parser.get_errors();
    assert!(
        errors.len() >= 2,
        "Expected at least 2 errors, got {}",
        errors.len()
    );
}

#[test]
fn test_error_recovery_valid_code_after_error() {
    let source = r#"
        متغير = 5;
        متغير س = 10;
    "#;
    let mut parser = parser_with_markers(source);
    let result = parser.parse();

    assert!(result.is_err());
    let errors = parser.get_errors();
    assert!(!errors.is_empty());
}

#[test]
fn test_error_recovery_class_member_errors() {
    let source = r#"
        صنف اختبار {
            خاص = 5;
            خاص س: عدد;
            عام = 10;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let result = parser.parse();

    assert!(result.is_err());
    let errors = parser.get_errors();
    assert!(
        errors.len() >= 2,
        "Expected at least 2 errors for invalid class members, got {}",
        errors.len()
    );
}

#[test]
fn test_error_recovery_get_errors_returns_all() {
    let source = r#"
        متغير = 1;
        ثابت = 2;
        متغير = 3;
    "#;
    let mut parser = parser_with_markers(source);
    let _ = parser.parse();

    let errors = parser.get_errors();
    assert!(
        errors.len() >= 3,
        "Expected at least 3 errors, got {}",
        errors.len()
    );
    for err in errors {
        assert!(!err.message.is_empty());
        assert!(!err.message_ar.is_empty());
    }
}
