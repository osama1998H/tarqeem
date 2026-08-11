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
        StmtKind::Export(export_items) => match export_items {
            ExportItems::Declaration(inner) => match &inner.kind {
                StmtKind::FuncDecl { name, .. } => {
                    assert_eq!(name, "مساعدة");
                }
                _ => panic!("Expected FuncDecl inside export"),
            },
            _ => panic!("Expected Declaration export"),
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
        StmtKind::Export(export_items) => match export_items {
            ExportItems::Declaration(inner) => match &inner.kind {
                StmtKind::ClassDecl { name, .. } => {
                    assert_eq!(name, "مساعد");
                }
                _ => panic!("Expected ClassDecl inside export"),
            },
            _ => panic!("Expected Declaration export"),
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
fn test_parse_bare_return_before_newline() {
    // A bare أرجع on its own line must not swallow the next line as
    // its return expression
    let source = r#"
        دالة فحص(س: عدد) {
            إذا (س < 0) {
                أرجع
            }
            اطبع(س)
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser
        .parse()
        .expect("bare return before newline must parse");

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, .. } => {
            assert_eq!(body.statements.len(), 2, "if-statement and print call");
        }
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

// ─── Function-type annotations `(T, U) -> R` (issue #180) ───
//
// `parse_type_annotation` branches on a leading `(` into
// `parse_function_type_annotation` (src/parser/parser/decl_parser.rs);
// these tests pin the accepted grammar — comma variants, bare `()`,
// right-associative `->`, nesting — and the ب٠٠٠٢ rejection of a
// non-empty param list with no `->`.

#[test]
fn test_parse_function_type_annotation_arabic_comma() {
    let source = r#"
        ثابت جمع: (عدد، عدد) -> عدد = (أ، ب) => أ + ب؛
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { ty, .. } => {
            let type_ann = ty.as_ref().expect("Expected type annotation");
            match &type_ann.kind {
                TypeKind::Function {
                    params,
                    return_type,
                } => {
                    assert_eq!(params.len(), 2);
                    assert!(matches!(&params[0].kind, TypeKind::Simple(n) if n == "عدد"));
                    assert!(matches!(&params[1].kind, TypeKind::Simple(n) if n == "عدد"));
                    let ret = return_type.as_ref().expect("Expected a return type");
                    assert!(matches!(&ret.kind, TypeKind::Simple(n) if n == "عدد"));
                }
                _ => panic!("Expected function type annotation"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_function_type_annotation_ascii_comma() {
    let source = r#"
        ثابت جمع: (عدد, عدد) -> عدد = (أ, ب) => أ + ب;
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { ty, .. } => {
            let type_ann = ty.as_ref().expect("Expected type annotation");
            match &type_ann.kind {
                TypeKind::Function { params, .. } => {
                    assert_eq!(params.len(), 2);
                }
                _ => panic!("Expected function type annotation"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_bare_unit_function_type() {
    let source = r#"
        متغير ف: ()؛
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { ty, .. } => {
            let type_ann = ty.as_ref().expect("Expected type annotation");
            match &type_ann.kind {
                TypeKind::Function {
                    params,
                    return_type,
                } => {
                    // Bare `()` = a function returning nothing. Absence is
                    // modelled structurally (`None`), not with a sentinel
                    // type name — Tarqeem has no `فراغ` keyword.
                    assert!(params.is_empty());
                    assert!(return_type.is_none());
                }
                _ => panic!("Expected function type annotation"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_zero_param_function_type_with_return() {
    let source = r#"
        متغير ف: () -> عدد؛
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { ty, .. } => {
            let type_ann = ty.as_ref().expect("Expected type annotation");
            match &type_ann.kind {
                TypeKind::Function {
                    params,
                    return_type,
                } => {
                    assert!(params.is_empty());
                    let ret = return_type.as_ref().expect("Expected a return type");
                    assert!(matches!(&ret.kind, TypeKind::Simple(n) if n == "عدد"));
                }
                _ => panic!("Expected function type annotation"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_curried_function_type_is_right_associative() {
    let source = r#"
        متغير ف: (عدد) -> (عدد) -> عدد؛
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { ty, .. } => {
            let type_ann = ty.as_ref().expect("Expected type annotation");
            match &type_ann.kind {
                TypeKind::Function {
                    params,
                    return_type,
                } => {
                    assert_eq!(params.len(), 1);
                    let ret = return_type.as_ref().expect("Expected a return type");
                    match &ret.kind {
                        TypeKind::Function {
                            params: inner_params,
                            return_type: inner_return,
                        } => {
                            assert_eq!(inner_params.len(), 1);
                            let inner = inner_return.as_ref().expect("Expected a return type");
                            assert!(matches!(&inner.kind, TypeKind::Simple(n) if n == "عدد"));
                        }
                        _ => panic!("Expected curried (right-associative) function type"),
                    }
                }
                _ => panic!("Expected function type annotation"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_function_type_parameter_does_not_steal_declaration_arrow() {
    let source = r#"
        دالة طبق(ج: (عدد) -> عدد، ق: عدد) -> عدد {
            أرجع ج(ق)؛
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl {
            params,
            return_type,
            ..
        } => {
            assert_eq!(params.len(), 2);
            let param_ty = params[0].ty.as_ref().expect("Expected param type");
            assert!(matches!(&param_ty.kind, TypeKind::Function { .. }));

            let return_type = return_type.as_ref().expect("Expected return type");
            assert!(matches!(&return_type.kind, TypeKind::Simple(n) if n == "عدد"));
        }
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_function_type_as_return_type() {
    let source = r#"
        دالة اصنع() -> (عدد) -> عدد { }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { return_type, .. } => {
            let return_type = return_type.as_ref().expect("Expected return type");
            assert!(matches!(&return_type.kind, TypeKind::Function { .. }));
        }
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_function_type_nested_in_generic_argument() {
    let source = r#"
        متغير ق: مصفوفة<(عدد) -> عدد>؛
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { ty, .. } => {
            let type_ann = ty.as_ref().expect("Expected type annotation");
            match &type_ann.kind {
                TypeKind::Generic { base, args } => {
                    assert_eq!(base, "مصفوفة");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0].kind, TypeKind::Function { .. }));
                }
                _ => panic!("Expected generic type"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_function_type_inside_arrow_lambda_param_annotation() {
    let source = r#"
        ثابت ط = (ج: (عدد) -> عدد) => ج(٢)؛
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { init, .. } => {
            let init = init.as_ref().expect("Expected initializer");
            match &init.kind {
                ExprKind::Lambda { params, .. } => {
                    assert_eq!(params.len(), 1);
                    let param_ty = params[0].ty.as_ref().expect("Expected param type");
                    assert!(matches!(&param_ty.kind, TypeKind::Function { .. }));
                }
                _ => panic!("Expected Lambda expression"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_function_type_missing_arrow_is_error() {
    use crate::error::codes::ERR_UNEXPECTED_TOKEN;

    let source = r#"
        متغير ف: (عدد)؛
    "#;
    let mut parser = parser_with_markers(source);
    let result = parser.parse();

    let err = result.expect_err("Expected a parse error for missing '->'");
    assert_eq!(
        err.code.as_deref(),
        Some(ERR_UNEXPECTED_TOKEN.to_string().as_str())
    );
}

#[test]
fn test_parse_grouping_with_arabic_digits_still_works() {
    // Control test: proves plain parenthesized-expression grouping is
    // unaffected by the new leading-'(' branch in parse_type_annotation.
    let source = r#"
        متغير س = (١ + ٢) * ٣؛
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { init, .. } => {
            let init = init.as_ref().expect("Expected initializer");
            match &init.kind {
                ExprKind::Binary { op, left, .. } => {
                    assert_eq!(*op, BinaryOp::Mul);
                    assert!(matches!(&left.kind, ExprKind::Grouping(_)));
                }
                _ => panic!("Expected binary expression"),
            }
        }
        _ => panic!("Expected VarDecl"),
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
        assert!(!err.message.is_empty());
    }
}

// ─── Contextual keywords: احصل/عيّن/حالة as identifiers (issue #183) ───
// These keywords are reserved only inside خاصية accessor blocks and تطابق
// arms; everywhere else they must parse as ordinary identifiers.

#[test]
fn test_parse_method_named_get_and_set() {
    // Mirrors stdlib_trq/مجموعات/قائمة.ترقيم methods
    let source = r#"
        صنف قائمة {
            عام دالة احصل(فهرس: عدد) -> عدد {
                أرجع فهرس;
            }

            عام دالة عيّن(فهرس: عدد، قيمة: عدد) {
                اطبع(قيمة);
            }
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl { members, .. } => {
            assert_eq!(members.len(), 2);
            match &members[0] {
                ClassMember::Method { name, .. } => assert_eq!(name, "احصل"),
                _ => panic!("Expected method named احصل"),
            }
            match &members[1] {
                ClassMember::Method { name, .. } => assert_eq!(name, "عيّن"),
                _ => panic!("Expected method named عيّن"),
            }
        }
        _ => panic!("Expected ClassDecl"),
    }
}

#[test]
fn test_parse_field_named_case() {
    // Mirrors stdlib_trq/اختبار/نتائج.ترقيم field
    let source = r#"
        صنف نتيجة {
            عام حالة: عدد;
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl { members, .. } => match &members[0] {
            ClassMember::Field { name, .. } => assert_eq!(name, "حالة"),
            _ => panic!("Expected field named حالة"),
        },
        _ => panic!("Expected ClassDecl"),
    }
}

#[test]
fn test_parse_variable_named_case() {
    let source = r#"
        متغير حالة = 5;
        حالة = حالة + 1;
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { name, .. } => assert_eq!(name, "حالة"),
        _ => panic!("Expected VarDecl"),
    }
    match &ast.statements[1].kind {
        StmtKind::Expr(expr) => match &expr.kind {
            ExprKind::Assignment { target, .. } => match &target.kind {
                ExprKind::Identifier(name) => assert_eq!(name, "حالة"),
                _ => panic!("Expected identifier target"),
            },
            _ => panic!("Expected assignment"),
        },
        _ => panic!("Expected expression statement"),
    }
}

#[test]
fn test_parse_member_call_get_set_spelling_preserved() {
    // عين (no shadda) and عيّن lex to the same keyword token, but as
    // identifiers they are distinct — the AST keeps the spelling the user wrote
    let source = r#"
        قائمتي.احصل(0);
        قائمتي.عين(0، 5);
        قائمتي.عيّن(1، 2);
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    let property_of = |stmt: &Stmt| -> String {
        match &stmt.kind {
            StmtKind::Expr(expr) => match &expr.kind {
                ExprKind::Call { callee, .. } => match &callee.kind {
                    ExprKind::Member { property, .. } => property.clone(),
                    _ => panic!("Expected member access callee"),
                },
                _ => panic!("Expected call"),
            },
            _ => panic!("Expected expression statement"),
        }
    };
    assert_eq!(property_of(&ast.statements[0]), "احصل");
    assert_eq!(property_of(&ast.statements[1]), "عين");
    assert_eq!(property_of(&ast.statements[2]), "عيّن");
}

#[test]
fn test_parse_enum_variant_with_contextual_keyword_type_arg() {
    // A type named حالة must be accepted as a generic type argument in the
    // speculative Enum<T>::Variant path, like any other identifier
    let source = r#"
        متغير س = اختياري<حالة>::عدم;
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { init, .. } => {
            let init = init.as_ref().expect("Expected initializer");
            match &init.kind {
                ExprKind::EnumVariant {
                    enum_name,
                    type_args,
                    variant_name,
                    ..
                } => {
                    assert_eq!(enum_name, "اختياري");
                    assert_eq!(variant_name, "عدم");
                    assert_eq!(type_args.len(), 1);
                }
                _ => panic!("Expected EnumVariant expression"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_synchronize_to_member_stops_at_contextual_keyword_names() {
    use crate::lexer::TokenKind;

    // Error recovery inside a class body must resume at a member named with a
    // contextual keyword instead of consuming it as garbage
    let mut parser = Parser::new("= 5 حالة: نص");
    parser.synchronize_to_member();
    assert!(matches!(parser.peek().kind, TokenKind::Case));

    let mut parser = Parser::new("= 5 خاصية اسم: نص");
    parser.synchronize_to_member();
    assert!(matches!(parser.peek().kind, TokenKind::Property));
}

#[test]
fn test_synchronize_to_arm_skips_midline_case_identifier() {
    use crate::lexer::TokenKind;

    // A mid-line حالة is an identifier use inside the broken arm; recovery
    // must resume at the next line-start حالة (the real arm head)
    let mut parser = Parser::new("=> حالة + 1\nحالة 2 => 3");
    parser.synchronize_to_arm();
    assert!(matches!(parser.peek().kind, TokenKind::Case));
    assert!(matches!(parser.previous().kind, TokenKind::Newline));
}

#[test]
fn test_parse_top_level_function_named_get() {
    // Mirrors stdlib_trq/شبكة/http.ترقيم: صدّر دالة احصل
    let source = r#"
        دالة احصل(رابط: نص) -> نص {
            أرجع رابط;
        }

        صدّر دالة عيّن(قيمة: عدد) {
            اطبع(قيمة);
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { name, .. } => assert_eq!(name, "احصل"),
        _ => panic!("Expected FuncDecl named احصل"),
    }
    match &ast.statements[1].kind {
        StmtKind::Export(ExportItems::Declaration(stmt)) => match &stmt.kind {
            StmtKind::FuncDecl { name, .. } => assert_eq!(name, "عيّن"),
            _ => panic!("Expected exported FuncDecl named عيّن"),
        },
        _ => panic!("Expected Export declaration"),
    }
}

#[test]
fn test_parse_import_item_named_get() {
    let source = r#"
        استورد { احصل، حالة } من "شبكة";
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::Import { items, .. } => match items {
            ImportItems::Named(imports) => {
                assert_eq!(imports.len(), 2);
                assert_eq!(imports[0].name, "احصل");
                assert_eq!(imports[1].name, "حالة");
            }
            _ => panic!("Expected named imports"),
        },
        _ => panic!("Expected Import statement"),
    }
}

#[test]
fn test_parse_property_accessors() {
    // Guards the contexts where احصل/عيّن remain keywords
    let source = r#"
        صنف شخص {
            خاص _اسم: نص;

            خاصية اسم: نص {
                احصل {
                    أرجع هذا._اسم;
                }
                عيّن(قيمة) {
                    هذا._اسم = قيمة;
                }
            }
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl { members, .. } => match &members[1] {
            ClassMember::Property {
                name, accessors, ..
            } => {
                assert_eq!(name, "اسم");
                assert_eq!(accessors.len(), 2);
                assert!(matches!(accessors[0], PropertyAccessor::Get { .. }));
                match &accessors[1] {
                    PropertyAccessor::Set { param_name, .. } => assert_eq!(param_name, "قيمة"),
                    _ => panic!("Expected Set accessor"),
                }
            }
            _ => panic!("Expected Property member"),
        },
        _ => panic!("Expected ClassDecl"),
    }
}

#[test]
fn test_parse_match_with_case_named_scrutinee() {
    // حالة stays the arm keyword inside تطابق while acting as a variable outside
    let source = r#"
        متغير حالة = 2;
        تطابق (حالة) {
            حالة 1 => اطبع("واحد")
            حالة 2 => اطبع("اثنان")
            غير_ذلك => اطبع("غير معروف")
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[1].kind {
        StmtKind::Match { expr, arms } => {
            assert_eq!(arms.len(), 3);
            match &expr.kind {
                ExprKind::Identifier(name) => assert_eq!(name, "حالة"),
                _ => panic!("Expected identifier scrutinee"),
            }
        }
        _ => panic!("Expected Match statement"),
    }
}

#[test]
fn test_parse_arrow_param_named_case() {
    let source = r#"
        متغير د = (حالة: عدد) => حالة + 1;
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { init, .. } => {
            let init = init.as_ref().expect("Expected initializer");
            match &init.kind {
                ExprKind::Lambda { params, .. } => {
                    assert_eq!(params.len(), 1);
                    assert_eq!(params[0].name, "حالة");
                }
                _ => panic!("Expected Lambda expression"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

#[test]
fn test_parse_object_literal_key_named_case() {
    let source = r#"
        متغير م = {حالة: 1};
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::VarDecl { init, .. } => {
            let init = init.as_ref().expect("Expected initializer");
            match &init.kind {
                ExprKind::Object(pairs) => {
                    assert_eq!(pairs.len(), 1);
                    assert_eq!(pairs[0].0, "حالة");
                }
                _ => panic!("Expected object literal"),
            }
        }
        _ => panic!("Expected VarDecl"),
    }
}

// Regression tests for #193/#194/#198 (comment handling & error-masking bundle)
//
// Comment tokens (`LineComment` "//", `DocComment` "///", `BlockDocComment`
// "/** */") must be handled consistently at every "wait for a terminator
// token" loop in the parser, and a real mid-file error must never be masked
// by the generic end-of-file/end-marker diagnostic. The lexer/parser fixes for
// #193/#194/#198 have landed; these tests are what keep them fixed.

// ─── Group 1: a comment-only body must parse to an empty container ───

#[test]
fn test_parse_function_body_only_line_comment() {
    let source = r#"
        دالة س() {
            // تعليق
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, .. } => assert!(body.statements.is_empty()),
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_function_body_only_doc_comment() {
    let source = r#"
        دالة س() {
            /// تعليق
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, .. } => assert!(body.statements.is_empty()),
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_function_body_only_block_doc_comment() {
    let source = r#"
        دالة س() {
            /** تعليق */
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, .. } => assert!(body.statements.is_empty()),
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_class_body_only_line_comment() {
    let source = r#"
        صنف س {
            // تعليق
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl { members, .. } => assert!(members.is_empty()),
        _ => panic!("Expected ClassDecl"),
    }
}

#[test]
fn test_parse_class_body_only_doc_comment() {
    let source = r#"
        صنف س {
            /// تعليق
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl { members, .. } => assert!(members.is_empty()),
        _ => panic!("Expected ClassDecl"),
    }
}

#[test]
fn test_parse_class_body_only_block_doc_comment() {
    let source = r#"
        صنف س {
            /** تعليق */
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl { members, .. } => assert!(members.is_empty()),
        _ => panic!("Expected ClassDecl"),
    }
}

#[test]
fn test_parse_interface_body_only_line_comment() {
    let source = r#"
        ميثاق م {
            // تعليق
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::InterfaceDecl { methods, .. } => assert!(methods.is_empty()),
        _ => panic!("Expected InterfaceDecl"),
    }
}

#[test]
fn test_parse_interface_body_only_doc_comment() {
    // Unlike the "//" case, this doc-comment variant is currently broken
    // even with the partial line-comment guard already in place — a new
    // case exposed by this bundle, not just a duplicate of the "//" test.
    let source = r#"
        ميثاق م {
            /// تعليق
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::InterfaceDecl { methods, .. } => assert!(methods.is_empty()),
        _ => panic!("Expected InterfaceDecl"),
    }
}

#[test]
fn test_parse_interface_body_only_block_doc_comment() {
    let source = r#"
        ميثاق م {
            /** تعليق */
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::InterfaceDecl { methods, .. } => assert!(methods.is_empty()),
        _ => panic!("Expected InterfaceDecl"),
    }
}

#[test]
fn test_parse_enum_body_only_line_comment() {
    let source = r#"
        تعداد ت {
            // تعليق
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::EnumDecl { variants, .. } => assert!(variants.is_empty()),
        _ => panic!("Expected EnumDecl"),
    }
}

#[test]
fn test_parse_enum_body_only_doc_comment() {
    let source = r#"
        تعداد ت {
            /// تعليق
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::EnumDecl { variants, .. } => assert!(variants.is_empty()),
        _ => panic!("Expected EnumDecl"),
    }
}

#[test]
fn test_parse_enum_body_only_block_doc_comment() {
    let source = r#"
        تعداد ت {
            /** تعليق */
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::EnumDecl { variants, .. } => assert!(variants.is_empty()),
        _ => panic!("Expected EnumDecl"),
    }
}

#[test]
fn test_parse_match_block_only_line_comment() {
    let source = r#"
        دالة س(ص: عدد) {
            تطابق (ص) {
                // تعليق
            }
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, .. } => match &body.statements[0].kind {
            StmtKind::Match { arms, .. } => assert!(arms.is_empty()),
            _ => panic!("Expected Match statement"),
        },
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_match_block_only_doc_comment() {
    let source = r#"
        دالة س(ص: عدد) {
            تطابق (ص) {
                /// تعليق
            }
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, .. } => match &body.statements[0].kind {
            StmtKind::Match { arms, .. } => assert!(arms.is_empty()),
            _ => panic!("Expected Match statement"),
        },
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_match_block_only_block_doc_comment() {
    let source = r#"
        دالة س(ص: عدد) {
            تطابق (ص) {
                /** تعليق */
            }
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, .. } => match &body.statements[0].kind {
            StmtKind::Match { arms, .. } => assert!(arms.is_empty()),
            _ => panic!("Expected Match statement"),
        },
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_top_level_only_line_comment() {
    let mut parser = parser_with_markers("// تعليق");
    let ast = parser.parse().unwrap();
    assert!(ast.statements.is_empty());
}

#[test]
fn test_parse_top_level_only_doc_comment() {
    let mut parser = parser_with_markers("/// تعليق");
    let ast = parser.parse().unwrap();
    assert!(ast.statements.is_empty());
}

#[test]
fn test_parse_top_level_only_block_doc_comment() {
    let mut parser = parser_with_markers("/** تعليق */");
    let ast = parser.parse().unwrap();
    assert!(ast.statements.is_empty());
}

#[test]
fn test_parse_property_accessor_trailing_comment_before_close() {
    // A comment between the last accessor and the closing '}' of a خاصية
    // block must not be mistaken for the start of another accessor.
    let source = r#"
        صنف ن {
            خاصية س: عدد {
                احصل => 1
                // تعليق
            }
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl { members, .. } => match &members[0] {
            ClassMember::Property { accessors, .. } => assert_eq!(accessors.len(), 1),
            _ => panic!("Expected Property member"),
        },
        _ => panic!("Expected ClassDecl"),
    }
}

// ─── Group 2: trailing comment before a terminator, after a real statement (#194) ───

#[test]
fn test_parse_trailing_line_comment_after_statement() {
    let source = r#"
        دالة س() {
            اطبع(1) // تعليق
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, .. } => assert_eq!(body.statements.len(), 1),
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_trailing_doc_comment_after_statement() {
    let source = r#"
        دالة س() {
            اطبع(1) /// وثيقة
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, .. } => {
            assert_eq!(body.statements.len(), 1);
            let trailing = body.statements[0]
                .trailing_comment
                .as_deref()
                .expect("Expected trailing comment");
            assert!(trailing.contains("وثيقة"));
        }
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_trailing_block_doc_comment_after_statement() {
    let source = r#"
        دالة س() {
            اطبع(1) /** وثيقة */
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, .. } => {
            assert_eq!(body.statements.len(), 1);
            let trailing = body.statements[0]
                .trailing_comment
                .as_deref()
                .expect("Expected trailing comment");
            assert!(trailing.contains("وثيقة"));
        }
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_trailing_comment_does_not_swallow_next_statement() {
    let source = r#"
        دالة س() {
            متغير أ = 1
            // تعليق
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, .. } => assert_eq!(body.statements.len(), 1),
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_class_member_then_trailing_comment() {
    let source = r#"
        صنف س {
            عام دالة م() {}
            // تعليق
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl { members, .. } => assert_eq!(members.len(), 1),
        _ => panic!("Expected ClassDecl"),
    }
}

#[test]
fn test_parse_top_level_trailing_comment_before_end_marker() {
    // Built manually (not via parser_with_markers) so the trailing comment
    // sits immediately before الحمد_لله, after a real top-level statement.
    let source = format!("بسم_الله\nمتغير س = 5\n{}\nالحمد_لله", "// تعليق");
    let mut parser = Parser::new(&source);
    let ast = parser.parse().unwrap();

    assert_eq!(ast.statements.len(), 1);
}

// ─── Group 3: أرجع must not swallow a trailing comment as its expression (#194) ───

#[test]
fn test_parse_return_expr_with_trailing_doc_comment() {
    let source = r#"
        دالة س() -> عدد {
            أرجع 1 /// وثيقة
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, .. } => match &body.statements[0].kind {
            StmtKind::Return(Some(expr)) => match &expr.kind {
                ExprKind::Literal(Literal::Int(1)) => {}
                other => panic!("Expected literal 1, got {:?}", other),
            },
            other => panic!("Expected Return(Some(_)), got {:?}", other),
        },
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_bare_return_with_trailing_doc_comment() {
    let source = r#"
        دالة س() {
            أرجع /// وثيقة
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, .. } => match &body.statements[0].kind {
            StmtKind::Return(None) => {}
            other => panic!("Expected Return(None), got {:?}", other),
        },
        _ => panic!("Expected FuncDecl"),
    }
}

// ─── Group 4: a trailing /// must not bleed into the next declaration's doc comment (#193 lexer fix) ───

#[test]
fn test_trailing_doc_comment_does_not_attach_to_next_declaration() {
    let source = r#"
        متغير س = 5 /// ملاحظة عابرة
        /// وثيقة الدالة ب
        دالة ب() {}
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    let var_stmt = ast
        .statements
        .iter()
        .find(|s| matches!(s.kind, StmtKind::VarDecl { .. }))
        .expect("Expected a VarDecl statement");
    let var_trailing = var_stmt
        .trailing_comment
        .as_deref()
        .expect("Expected trailing comment on VarDecl");
    assert!(var_trailing.contains("ملاحظة عابرة"));

    let func_stmt = ast
        .statements
        .iter()
        .find_map(|s| match &s.kind {
            StmtKind::FuncDecl {
                name, doc_comment, ..
            } if name == "ب" => Some(doc_comment.clone()),
            _ => None,
        })
        .expect("Expected a FuncDecl named ب");
    assert_eq!(func_stmt, Some("وثيقة الدالة ب".to_string()));
}

// ─── Group 5: a real mid-file error must not be masked by the end-marker diagnostic (#193) ───

#[test]
fn test_real_error_is_not_masked_by_end_marker() {
    use crate::error::codes::ERR_EXPECTED_VARIABLE_NAME;

    let source = r#"
        دالة س() {
            متغير = 5
        }
    "#;
    let mut parser = parser_with_markers(source);
    let result = parser.parse();

    let err = result.expect_err("Expected a parse error");
    assert!(
        !err.message.contains("الحمد_لله"),
        "Real error must not be masked by the end-marker diagnostic, got: {}",
        err.message
    );
    assert_eq!(
        err.code.as_deref(),
        Some(ERR_EXPECTED_VARIABLE_NAME.to_string().as_str())
    );
}

// test_missing_file_end_marker_still_reports_end_marker is covered by
// src/parser/parser/mod.rs's own test module (owned by another agent's file).

#[test]
fn test_stray_right_brace_at_top_level_terminates() {
    let mut parser = Parser::new(&wrap_with_markers("}"));
    let result = parser.parse();

    assert!(result.is_err());
    assert!(
        parser.get_errors().len() < 20,
        "Expected a bounded number of errors, got {}",
        parser.get_errors().len()
    );
}

#[test]
fn test_nested_block_error_does_not_cascade() {
    let source = r#"
        دالة أ() {
            إذا (س) {
                متغير = 1
            }
        }
    "#;
    let mut parser = parser_with_markers(source);
    let result = parser.parse();

    assert!(result.is_err());
    assert!(
        parser.get_errors().len() <= 2,
        "Expected error recovery to stay local to the nested block, got {} errors",
        parser.get_errors().len()
    );
}

// Regression tests for the code-review fixes to #193/#194/#198's comment
// handling: `pending_comments` is a single buffer shared across the parser,
// but four statement-list loops (parse_class_member, and the property
// accessor/interface-method/enum-variant loops) never drained it, so a
// comment collected there survived until the next unrelated
// parse_declaration() call and got misattached to whatever statement
// followed the enclosing construct. Group A below pins the fix (the leak
// no longer happens); Group B pins Block::dangling_comments, the new field
// that lets parse_block preserve a comment that precedes '}' with no
// following statement to attach to.

// ─── Group A: a comment inside a class/interface/enum/property-accessor
// body must not leak to the statement that follows the whole construct ───

#[test]
fn test_interface_method_comment_does_not_leak_to_next_statement() {
    let source = r#"
        ميثاق م {
            // تعليق
            دالة أ()
        }
        متغير س = 5
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    let var_stmt = ast
        .statements
        .iter()
        .find(|s| matches!(s.kind, StmtKind::VarDecl { .. }))
        .expect("Expected a VarDecl statement");
    assert!(
        var_stmt.leading_comments.is_empty(),
        "Comment inside the interface body must not leak to the following statement"
    );
}

#[test]
fn test_enum_variant_comment_does_not_leak_to_next_statement() {
    let source = r#"
        تعداد م {
            // تعليق
            أ
        }
        متغير س = 5
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    let var_stmt = ast
        .statements
        .iter()
        .find(|s| matches!(s.kind, StmtKind::VarDecl { .. }))
        .expect("Expected a VarDecl statement");
    assert!(
        var_stmt.leading_comments.is_empty(),
        "Comment inside the enum body must not leak to the following statement"
    );
}

#[test]
fn test_property_accessor_comment_does_not_leak_to_next_statement() {
    let source = r#"
        صنف م {
            خاصية س: عدد {
                // تعليق
                احصل => 1
            }
        }
        متغير ص = 5
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    let var_stmt = ast
        .statements
        .iter()
        .find(|s| matches!(s.kind, StmtKind::VarDecl { .. }))
        .expect("Expected a VarDecl statement");
    assert!(
        var_stmt.leading_comments.is_empty(),
        "Comment inside the property accessor body must not leak to the following statement"
    );
}

#[test]
fn test_class_member_comment_stays_with_member() {
    let source = r#"
        صنف س {
            // تعليق
            عام دالة م() {}
        }
        متغير ص = 5
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::ClassDecl { members, .. } => match &members[0] {
            ClassMember::Method {
                leading_comments, ..
            } => {
                // scan_line_comment keeps the raw text after `//`, including
                // the leading space — it is not a DocComment, which strips it.
                assert_eq!(leading_comments, &vec![" تعليق".to_string()]);
            }
            other => panic!("Expected ClassMember::Method, got {:?}", other),
        },
        _ => panic!("Expected ClassDecl"),
    }

    match &ast.statements[1].kind {
        StmtKind::VarDecl { .. } => {
            assert!(ast.statements[1].leading_comments.is_empty());
        }
        _ => panic!("Expected VarDecl"),
    }
}

// ─── Group B: Block::dangling_comments preserves a comment with no
// statement to attach to (a comment-only body, or one trailing the last
// statement before '}') ───

#[test]
fn test_parse_block_dangling_line_comment() {
    let source = r#"
        دالة س() {
            // تعليق
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, .. } => {
            assert!(body.statements.is_empty());
            assert_eq!(body.dangling_comments, vec![" تعليق".to_string()]);
        }
        _ => panic!("Expected FuncDecl"),
    }
}

#[test]
fn test_parse_block_dangling_after_statement() {
    let source = r#"
        دالة س() {
            اطبع(1)
            // تعليق
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    match &ast.statements[0].kind {
        StmtKind::FuncDecl { body, .. } => {
            assert_eq!(body.statements.len(), 1);
            assert_eq!(body.dangling_comments, vec![" تعليق".to_string()]);
        }
        _ => panic!("Expected FuncDecl"),
    }
}

// ─── Group 8: leading /** */ attaches as a doc comment; صدّر keeps its doc (#201, #204) ───

/// `/** */` before a declaration used to fall through to the expression parser
/// as `رمز غير متوقع: BlockDocComment(..)`. It is only safe to attach now that
/// the formatter re-prefixes `///` on every doc line — attaching it while the
/// formatter still stripped markers would have converted a loud parse error into
/// silent corruption (#201).
#[test]
fn test_parse_leading_block_doc_comment_attaches_to_declaration() {
    let source = r#"
        /** وثيقة الدالة */
        دالة س() {}
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser
        .parse()
        .expect("/** */ before a declaration must parse");

    let doc = ast
        .statements
        .iter()
        .find_map(|s| match &s.kind {
            StmtKind::FuncDecl {
                name, doc_comment, ..
            } if name == "س" => Some(doc_comment.clone()),
            _ => None,
        })
        .expect("Expected a FuncDecl named س");
    assert_eq!(doc, Some("وثيقة الدالة".to_string()));
}

#[test]
fn test_parse_leading_block_doc_comment_on_class_and_enum() {
    let source = r#"
        /** وثيقة الصنف */
        صنف ش {
            /** وثيقة الحقل */
            خاص اسم: نص
        }
        /** وثيقة التعداد */
        تعداد ل {
            /** وثيقة الحالة */
            أحمر
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().expect("must parse");

    let class_doc = ast
        .statements
        .iter()
        .find_map(|s| match &s.kind {
            StmtKind::ClassDecl { doc_comment, .. } => Some(doc_comment.clone()),
            _ => None,
        })
        .expect("Expected a ClassDecl");
    assert_eq!(class_doc, Some("وثيقة الصنف".to_string()));

    let enum_doc = ast
        .statements
        .iter()
        .find_map(|s| match &s.kind {
            StmtKind::EnumDecl { doc_comment, .. } => Some(doc_comment.clone()),
            _ => None,
        })
        .expect("Expected an EnumDecl");
    assert_eq!(enum_doc, Some("وثيقة التعداد".to_string()));
}

/// Mirror of `test_trailing_doc_comment_does_not_attach_to_next_declaration`
/// for the block form: accepting `/** */` as a leading doc comment must not let
/// a *trailing* one bleed forward onto the next declaration.
#[test]
fn test_trailing_block_doc_comment_does_not_attach_to_next_declaration() {
    let source = r#"
        متغير س = 5 /** ملاحظة عابرة */
        /** وثيقة الدالة ب */
        دالة ب() {}
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().unwrap();

    let var_trailing = ast
        .statements
        .iter()
        .find(|s| matches!(s.kind, StmtKind::VarDecl { .. }))
        .expect("Expected a VarDecl statement")
        .trailing_comment
        .as_deref()
        .expect("Expected trailing comment on VarDecl");
    assert!(var_trailing.contains("ملاحظة عابرة"));

    let func_doc = ast
        .statements
        .iter()
        .find_map(|s| match &s.kind {
            StmtKind::FuncDecl {
                name, doc_comment, ..
            } if name == "ب" => Some(doc_comment.clone()),
            _ => None,
        })
        .expect("Expected a FuncDecl named ب");
    assert_eq!(func_doc, Some("وثيقة الدالة ب".to_string()));
}

/// #204: `parse_declaration` consumes the doc comment before it can tell that a
/// `صدّر` follows, then recurses — so the inner declaration used to receive
/// `doc_comment: None` and the doc was silently dropped for every exported
/// declaration.
#[test]
fn test_parse_exported_declaration_keeps_doc_comment() {
    let source = r#"
        /// وثيقة الدالة المصدرة
        صدّر دالة جمع(أ: عدد، ب: عدد) -> عدد {
            أرجع أ + ب
        }
        /// وثيقة الصنف المصدر
        صدّر صنف نقطة {
            عام س: عدد
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().expect("must parse");

    let mut func_doc = None;
    let mut class_doc = None;
    for stmt in &ast.statements {
        if let StmtKind::Export(ExportItems::Declaration(inner)) = &stmt.kind {
            match &inner.kind {
                StmtKind::FuncDecl { doc_comment, .. } => func_doc = doc_comment.clone(),
                StmtKind::ClassDecl { doc_comment, .. } => class_doc = doc_comment.clone(),
                other => panic!("Unexpected exported declaration: {:?}", other),
            }
        }
    }

    assert_eq!(func_doc, Some("وثيقة الدالة المصدرة".to_string()));
    assert_eq!(class_doc, Some("وثيقة الصنف المصدر".to_string()));
}

/// `صدّر\n/// وثيقة\nدالة س() {}` used to be a hard parse error of the #203
/// class: the one-shot `consume_doc_comment()` ran before the newline after
/// `صدّر` was skipped, so the doc token still sat where a declaration keyword
/// was expected. `collect_leading_trivia` skips blank lines *before* looking for
/// comments, so the form now parses and the doc attaches. The predecessor of
/// this test pinned the error and said "if it ever attaches, update this test";
/// this is that update. Its real intent is unchanged and still asserted: the
/// doc comment must never be lost *silently*.
#[test]
fn test_parse_doc_comment_between_export_and_declaration_now_attaches() {
    let source = r#"
        صدّر
        /// وثيقة الدالة
        دالة س() {}
    "#;
    let mut parser = parser_with_markers(source);
    let result = parser.parse();

    let ast = result.expect("صدّر followed by a doc comment on its own line must parse");
    assert!(
        parser.get_errors().is_empty(),
        "the form is supported now, so nothing should be reported"
    );

    let attached_doc = ast.statements.iter().find_map(|s| match &s.kind {
        StmtKind::Export(ExportItems::Declaration(inner)) => match &inner.kind {
            StmtKind::FuncDecl { doc_comment, .. } => doc_comment.clone(),
            _ => None,
        },
        _ => None,
    });

    assert_eq!(attached_doc, Some("وثيقة الدالة".to_string()));
}

// ─── Group 9: a trailing /** */ must not be stolen as the next member's doc ───

/// Documentation describes what follows it, so a `/** */` trailing code on the
/// same line annotates that line. Accepting `BlockDocComment` unconditionally
/// let `consume_doc_comment` re-attach it to the *next* class member, so
/// `tarqeem doc` published the note under the wrong name and `fmt -w` rewrote
/// the file that way. Verified against a merge-base build before fixing.
#[test]
fn test_trailing_block_doc_comment_is_not_stolen_by_next_class_member() {
    let source = r#"
        صنف ش {
            خاص اسم: نص /** ملاحظة على الاسم */
            خاص عمر: عدد
        }
    "#;
    let mut parser = parser_with_markers(source);
    let result = parser.parse();

    // Whatever the parser does with this form, the note must never end up as
    // the documentation of `عمر`.
    if let Ok(ast) = result.as_ref() {
        for stmt in &ast.statements {
            if let StmtKind::ClassDecl { members, .. } = &stmt.kind {
                for member in members {
                    if let ClassMember::Field {
                        name, doc_comment, ..
                    } = member
                    {
                        if name == "عمر" {
                            assert!(
                                doc_comment.is_none(),
                                "note about 'اسم' must not become the doc of 'عمر': {:?}",
                                doc_comment
                            );
                        }
                    }
                }
            }
        }
    }
}

/// A doc comment sitting on the same line as `صدّر` must still be consumed —
/// leaving it in the stream made it fall through to the expression parser, a new
/// hard error for source that compiled before.
#[test]
fn test_doc_comment_trailing_export_keyword_does_not_error() {
    let source = r#"
        /// خارجي
        صدّر /// داخلي
        دالة س() {}
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser
        .parse()
        .expect("a doc comment after صدّر must not be a parse error");

    // The doc written above `صدّر` documents the declaration; the one trailing
    // the keyword is kept as an ordinary comment rather than discarded.
    let doc = ast
        .statements
        .iter()
        .find_map(|s| match &s.kind {
            StmtKind::Export(ExportItems::Declaration(inner)) => match &inner.kind {
                StmtKind::FuncDecl { doc_comment, .. } => {
                    Some(inner.leading_comments.clone()).map(|lc| (doc_comment.clone(), lc))
                }
                _ => None,
            },
            _ => None,
        })
        .expect("Expected an exported FuncDecl");
    assert_eq!(doc.0, Some("خارجي".to_string()));
    assert!(
        doc.1.iter().any(|c| c.contains("داخلي")),
        "the note trailing صدّر must be preserved, got {:?}",
        doc.1
    );
}

/// A doc comment before a statement with no `doc_comment` field must be kept as
/// a leading comment. Consuming it and dropping it made `fmt -w` erase the text.
#[test]
fn test_orphaned_doc_comment_is_demoted_to_leading_comment() {
    let source = r#"
        /** ملاحظة مهمة */
        اطبع("س")
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().expect("must parse");

    let stmt = ast
        .statements
        .iter()
        .find(|s| matches!(s.kind, StmtKind::Expr(_)))
        .expect("Expected an expression statement");
    assert!(
        stmt.leading_comments
            .iter()
            .any(|c| c.contains("ملاحظة مهمة")),
        "orphaned doc comment must be preserved, got {:?}",
        stmt.leading_comments
    );
}

// ─── Group 11: a comment run before a declaration, in any order (#203) ───

/// The minimal #203 repro: `collect_line_comments()` ran before the doc block
/// was consumed, so a `//` written after it was never collected and fell through
/// the declaration dispatch as ب٠٠٠١.
#[test]
fn test_line_comment_after_doc_block_before_declaration_parses() {
    let source = r#"
        /// وثيقة
        // ملاحظة
        دالة س() {}
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().expect("a // after a /// must parse");

    let stmt = &ast.statements[0];
    match &stmt.kind {
        StmtKind::FuncDecl { doc_comment, .. } => {
            assert_eq!(doc_comment.as_deref(), Some("وثيقة"));
        }
        other => panic!("Expected FuncDecl, got {other:?}"),
    }
    assert_eq!(stmt.leading_comments, vec![" ملاحظة".to_string()]);
}

/// The exact shape of `stdlib_trq/رياضيات/اساسي.ترقيم:1-19`, which is how 20 of
/// the 33 unparseable stdlib files opened: file doc, `//` banner, real doc, code.
#[test]
fn test_banner_between_module_doc_and_declaration_parses() {
    let source = r#"
        /// وحدة الرياضيات

        // ═══════
        // القيمة المطلقة
        // ═══════

        /// القيمة المطلقة لعدد
        صدّر دالة مطلق(س: عدد) -> عدد {
            أرجع س
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().expect("the stdlib banner shape must parse");

    assert_eq!(ast.module_doc.as_deref(), Some("وحدة الرياضيات"));

    let stmt = &ast.statements[0];
    assert_eq!(
        stmt.leading_comments,
        vec![
            " ═══════".to_string(),
            " القيمة المطلقة".to_string(),
            " ═══════".to_string(),
        ],
        "banner lines must stay above the declaration, in source order"
    );
    match &stmt.kind {
        StmtKind::Export(ExportItems::Declaration(inner)) => match &inner.kind {
            StmtKind::FuncDecl { doc_comment, .. } => {
                assert_eq!(doc_comment.as_deref(), Some("القيمة المطلقة لعدد"));
            }
            other => panic!("Expected FuncDecl, got {other:?}"),
        },
        other => panic!("Expected Export, got {other:?}"),
    }
}

/// The `stdlib_trq/نص.ترقيم` shape: two doc blocks split by a blank line, which
/// the lexer refuses to merge, so the second one used to hit ب٠٠٠١.
#[test]
fn test_two_doc_blocks_before_declaration_hoist_first_to_module_doc() {
    let source = r#"
        /// وحدة النصوص

        /// تحقق إذا كان النص فارغاً
        صدّر دالة فارغ(س: نص) -> منطقي {
            أرجع صحيح
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().expect("two doc blocks must parse");

    assert_eq!(ast.module_doc.as_deref(), Some("وحدة النصوص"));
    match &ast.statements[0].kind {
        StmtKind::Export(ExportItems::Declaration(inner)) => match &inner.kind {
            StmtKind::FuncDecl { doc_comment, .. } => {
                assert_eq!(doc_comment.as_deref(), Some("تحقق إذا كان النص فارغاً"));
            }
            other => panic!("Expected FuncDecl, got {other:?}"),
        },
        other => panic!("Expected Export, got {other:?}"),
    }
}

/// Pins the 20 corpus files whose header is followed directly by a declaration:
/// there is no signal that such a doc describes the file rather than the
/// declaration, so it must keep attaching exactly as it did before #203.
#[test]
fn test_module_doc_not_taken_when_declaration_follows_directly() {
    let source = r#"
        /// صنف القائمة الديناميكية

        صدّر صنف قائمة {
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().expect("must parse");

    assert!(
        ast.module_doc.is_none(),
        "a doc with a declaration right after it documents that declaration"
    );
    match &ast.statements[0].kind {
        StmtKind::Export(ExportItems::Declaration(inner)) => match &inner.kind {
            StmtKind::ClassDecl { doc_comment, .. } => {
                assert_eq!(doc_comment.as_deref(), Some("صنف القائمة الديناميكية"));
            }
            other => panic!("Expected ClassDecl, got {other:?}"),
        },
        other => panic!("Expected Export, got {other:?}"),
    }
}

/// Without the `الحمد_لله`/`Eof` clause in `doc_comment_is_module_header`, a file
/// doc with nothing after it survives one `fmt` pass and is discarded by the
/// second — data loss that only shows up on the second run.
#[test]
fn test_module_doc_taken_when_no_declaration_follows() {
    let source = r#"
        /// وثيقة الملف
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser
        .parse()
        .expect("a file with only a doc comment must parse");

    assert_eq!(ast.module_doc.as_deref(), Some("وثيقة الملف"));
    assert!(ast.statements.is_empty());
}

/// Several doc blocks and line-comment runs interleaved: the block nearest the
/// declaration documents it and everything else keeps its source position.
#[test]
fn test_interleaved_comment_runs_keep_source_order() {
    let source = r#"
        // أ
        /// وثيقة١
        // ب
        /// وثيقة٢
        دالة س() {}
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().expect("must parse");

    let stmt = &ast.statements[0];
    match &stmt.kind {
        StmtKind::FuncDecl { doc_comment, .. } => {
            assert_eq!(doc_comment.as_deref(), Some("وثيقة٢"));
        }
        other => panic!("Expected FuncDecl, got {other:?}"),
    }
    assert_eq!(
        stmt.leading_comments,
        vec![" أ".to_string(), "وثيقة١".to_string(), " ب".to_string()],
        "the demoted doc must sit where it was written, between أ and ب"
    );
}

/// `استورد` has no `doc_comment` field, so a doc above the file's first import
/// used to be demoted and re-emitted as `//` — `fmt -w` silently downgraded the
/// module header of `stdlib_trq/ملفات/مجلد.ترقيم` and `مجموعات/فهرس.ترقيم` that
/// way. It is now recognised as the file's doc and keeps its marker.
#[test]
fn test_doc_block_before_leading_import_becomes_module_doc() {
    let source = r#"
        /// وحدة المجموعات
        استورد { س } من "وحدة"
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().expect("must parse");

    assert_eq!(ast.module_doc.as_deref(), Some("وحدة المجموعات"));
    let stmt = &ast.statements[0];
    assert!(matches!(stmt.kind, StmtKind::Import { .. }));
    assert!(stmt.leading_comments.is_empty());
}

/// The same shape for a re-export, which used to drop the doc outright rather
/// than demote it (`stdlib_trq/اختبار.ترقيم` lost all five of its `///` lines to
/// `fmt -w`).
#[test]
fn test_doc_block_before_leading_reexport_becomes_module_doc() {
    let source = r#"
        /// وحدة الاختبارات
        صدّر * من "اختبار/فهرس"
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().expect("must parse");

    assert_eq!(ast.module_doc.as_deref(), Some("وحدة الاختبارات"));
}

/// Only the *file's* doc is hoisted. An import further down the file has no
/// header role, so a doc above it keeps demoting into a leading comment — text
/// preserved, marker downgraded, exactly as before.
#[test]
fn test_doc_block_before_later_import_is_still_demoted() {
    let source = r#"
        دالة س() {}

        /// وثيقة الاستيراد
        استورد { ص } من "وحدة"
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser.parse().expect("must parse");

    assert!(ast.module_doc.is_none());
    let import = ast
        .statements
        .iter()
        .find(|s| matches!(s.kind, StmtKind::Import { .. }))
        .expect("Expected an import statement");
    assert_eq!(import.leading_comments, vec!["وثيقة الاستيراد".to_string()]);
}

/// The `parse_class_member` adoption: the same one-shot pair guarded class
/// members, so this shape was a hard error inside a `صنف` too.
#[test]
fn test_class_member_line_comment_after_doc_block_parses() {
    let source = r#"
        صنف ش {
            /// وثيقة الدالة
            // ملاحظة
            عام دالة م() {}
        }
    "#;
    let mut parser = parser_with_markers(source);
    let ast = parser
        .parse()
        .expect("a // after a /// in a class must parse");

    match &ast.statements[0].kind {
        StmtKind::ClassDecl { members, .. } => match &members[0] {
            ClassMember::Method {
                doc_comment,
                leading_comments,
                ..
            } => {
                assert_eq!(doc_comment.as_deref(), Some("وثيقة الدالة"));
                assert_eq!(leading_comments, &vec![" ملاحظة".to_string()]);
            }
            other => panic!("Expected Method, got {other:?}"),
        },
        other => panic!("Expected ClassDecl, got {other:?}"),
    }
}

/// `MethodSignature` and `EnumVariant` have no comment field, so the trivia loop
/// is deliberately NOT adopted in those two loops: demoting there would replace
/// today's loud error with a silent drop that `fmt -w` makes permanent. Pinned so
/// a later tidy-up cannot "finish the job" and lose user text instead.
#[test]
fn test_doc_comment_before_interface_method_still_errors_loudly() {
    let source = r#"
        ميثاق م {
            /// وثيقة
            // ملاحظة
            دالة س()
        }
    "#;
    let mut parser = parser_with_markers(source);
    let result = parser.parse();

    assert!(
        result.is_err() || !parser.get_errors().is_empty(),
        "an unattachable comment in an interface body must be reported, not swallowed"
    );
}

#[test]
fn test_doc_comment_before_property_accessor_still_errors_loudly() {
    let source = r#"
        صنف ش {
            خاصية س: عدد {
                /// وثيقة
                احصل {
                    أرجع 1
                }
            }
        }
    "#;
    let mut parser = parser_with_markers(source);
    let result = parser.parse();

    assert!(
        result.is_err() || !parser.get_errors().is_empty(),
        "an unattachable comment before an accessor must be reported, not swallowed"
    );
}
