//! Documentation extractor
//!
//! Extracts documentation from a parsed AST and produces a Documentation model.

use super::comment::DocCommentParser;
use super::model::{
    ClassDoc, ConstructorDoc, DocItem, Documentation, EnumDoc, EnumFieldDoc, EnumVariantDoc,
    FieldDoc, FunctionDoc, InterfaceDoc, MethodDoc, MethodSignatureDoc, ParamDoc, ReturnDoc,
    VariableDoc,
};
use crate::parser::{Ast, ClassMember, Stmt, StmtKind, TypeAnnotation, TypeKind, Visibility};

pub struct DocExtractor {
    module_name: String,
    source_path: String,
}

impl DocExtractor {
    pub fn new(module_name: String, source_path: String) -> Self {
        Self {
            module_name,
            source_path,
        }
    }

    pub fn extract(&self, ast: &Ast) -> Documentation {
        let mut doc = Documentation::new(self.module_name.clone(), self.source_path.clone());

        // `description` is rendered by every generator but had no source until
        // the parser started keeping a file's own doc comment (Ast::module_doc).
        doc.description = ast
            .module_doc
            .as_deref()
            .and_then(|comment| DocCommentParser::parse(comment).description);

        for stmt in &ast.statements {
            if let Some(item) = self.extract_stmt(stmt, false) {
                doc.items.push(item);
            }
        }

        doc
    }

    fn extract_stmt(&self, stmt: &Stmt, is_exported: bool) -> Option<DocItem> {
        match &stmt.kind {
            StmtKind::VarDecl {
                name,
                mutable,
                ty,
                doc_comment,
                ..
            } => {
                let mut var_doc = VariableDoc::new(name.clone());
                var_doc.is_mutable = *mutable;
                var_doc.is_exported = is_exported;
                var_doc.line = stmt.span.line;
                var_doc.ty = ty.as_ref().map(|t| self.type_to_string(t));

                if let Some(comment) = doc_comment {
                    let parsed = DocCommentParser::parse(comment);
                    var_doc.description = parsed.description;
                }

                Some(DocItem::Variable(var_doc))
            }

            StmtKind::FuncDecl {
                name,
                params,
                return_type,
                is_async,
                doc_comment,
                ..
            } => {
                let mut func_doc = FunctionDoc::new(name.clone());
                func_doc.is_async = *is_async;
                func_doc.is_exported = is_exported;
                func_doc.line = stmt.span.line;

                for param in params {
                    let param_doc = ParamDoc {
                        name: param.name.clone(),
                        ty: param.ty.as_ref().map(|t| self.type_to_string(t)),
                        description: None,
                        has_default: param.default.is_some(),
                    };
                    func_doc.params.push(param_doc);
                }

                if let Some(ret_ty) = return_type {
                    func_doc.returns = Some(ReturnDoc {
                        ty: Some(self.type_to_string(ret_ty)),
                        description: None,
                    });
                }

                if let Some(comment) = doc_comment {
                    let parsed = DocCommentParser::parse(comment);
                    func_doc.description = parsed.description;
                    func_doc.examples = parsed.examples;
                    func_doc.see_also = parsed.see_also;

                    for doc_param in parsed.params {
                        if let Some(ast_param) = func_doc
                            .params
                            .iter_mut()
                            .find(|p| p.name == doc_param.name)
                        {
                            ast_param.description = doc_param.description;
                            if ast_param.ty.is_none() {
                                ast_param.ty = doc_param.ty;
                            }
                        }
                    }

                    if let Some(ret) = parsed.returns {
                        if let Some(func_ret) = &mut func_doc.returns {
                            func_ret.description = ret.description;
                            if func_ret.ty.is_none() {
                                func_ret.ty = ret.ty;
                            }
                        } else {
                            func_doc.returns = Some(ret);
                        }
                    }
                }

                Some(DocItem::Function(func_doc))
            }

            StmtKind::ClassDecl {
                name,
                type_params,
                extends,
                implements,
                members,
                doc_comment,
            } => {
                let mut class_doc = ClassDoc::new(name.clone());
                class_doc.type_params = type_params.clone();
                class_doc.extends = extends.clone();
                class_doc.implements = implements.clone();
                class_doc.is_exported = is_exported;
                class_doc.line = stmt.span.line;

                if let Some(comment) = doc_comment {
                    let parsed = DocCommentParser::parse(comment);
                    class_doc.description = parsed.description;
                    class_doc.examples = parsed.examples;
                }

                for member in members {
                    match member {
                        ClassMember::Field {
                            visibility,
                            name,
                            ty,
                            is_static,
                            doc_comment,
                            ..
                        } => {
                            let mut field_doc = FieldDoc {
                                name: name.clone(),
                                ty: ty.as_ref().map(|t| self.type_to_string(t)),
                                description: None,
                                visibility: self.visibility_to_string(*visibility),
                                is_static: *is_static,
                            };

                            if let Some(comment) = doc_comment {
                                let parsed = DocCommentParser::parse(comment);
                                field_doc.description = parsed.description;
                            }

                            class_doc.fields.push(field_doc);
                        }

                        ClassMember::Method {
                            visibility,
                            name,
                            params,
                            return_type,
                            is_static,
                            is_async,
                            doc_comment,
                            ..
                        } => {
                            let mut method_doc = MethodDoc::new(name.clone());
                            method_doc.visibility = self.visibility_to_string(*visibility);
                            method_doc.is_static = *is_static;
                            method_doc.is_async = *is_async;

                            for param in params {
                                method_doc.params.push(ParamDoc {
                                    name: param.name.clone(),
                                    ty: param.ty.as_ref().map(|t| self.type_to_string(t)),
                                    description: None,
                                    has_default: param.default.is_some(),
                                });
                            }

                            if let Some(ret_ty) = return_type {
                                method_doc.returns = Some(ReturnDoc {
                                    ty: Some(self.type_to_string(ret_ty)),
                                    description: None,
                                });
                            }

                            if let Some(comment) = doc_comment {
                                let parsed = DocCommentParser::parse(comment);
                                method_doc.description = parsed.description;

                                for doc_param in parsed.params {
                                    if let Some(ast_param) = method_doc
                                        .params
                                        .iter_mut()
                                        .find(|p| p.name == doc_param.name)
                                    {
                                        ast_param.description = doc_param.description;
                                    }
                                }

                                if let Some(ret) = parsed.returns {
                                    if let Some(method_ret) = &mut method_doc.returns {
                                        method_ret.description = ret.description;
                                    } else {
                                        method_doc.returns = Some(ret);
                                    }
                                }
                            }

                            class_doc.methods.push(method_doc);
                        }

                        ClassMember::Constructor {
                            params,
                            doc_comment,
                            ..
                        } => {
                            let mut ctor_doc = ConstructorDoc {
                                description: None,
                                params: Vec::new(),
                            };

                            for param in params {
                                ctor_doc.params.push(ParamDoc {
                                    name: param.name.clone(),
                                    ty: param.ty.as_ref().map(|t| self.type_to_string(t)),
                                    description: None,
                                    has_default: param.default.is_some(),
                                });
                            }

                            if let Some(comment) = doc_comment {
                                let parsed = DocCommentParser::parse(comment);
                                ctor_doc.description = parsed.description;

                                for doc_param in parsed.params {
                                    if let Some(ast_param) = ctor_doc
                                        .params
                                        .iter_mut()
                                        .find(|p| p.name == doc_param.name)
                                    {
                                        ast_param.description = doc_param.description;
                                    }
                                }
                            }

                            class_doc.constructor = Some(ctor_doc);
                        }

                        ClassMember::Property {
                            visibility,
                            name,
                            ty,
                            is_static,
                            doc_comment,
                            ..
                        } => {
                            let mut field_doc = FieldDoc {
                                name: name.clone(),
                                ty: Some(self.type_to_string(ty)),
                                description: None,
                                visibility: self.visibility_to_string(*visibility),
                                is_static: *is_static,
                            };

                            if let Some(comment) = doc_comment {
                                let parsed = DocCommentParser::parse(comment);
                                field_doc.description = parsed.description;
                            }

                            class_doc.fields.push(field_doc);
                        }
                    }
                }

                Some(DocItem::Class(class_doc))
            }

            StmtKind::InterfaceDecl {
                name,
                type_params,
                methods,
                doc_comment,
            } => {
                let mut iface_doc = InterfaceDoc::new(name.clone());
                iface_doc.type_params = type_params.clone();
                iface_doc.is_exported = is_exported;
                iface_doc.line = stmt.span.line;

                if let Some(comment) = doc_comment {
                    let parsed = DocCommentParser::parse(comment);
                    iface_doc.description = parsed.description;
                }

                for method in methods {
                    let mut sig_doc = MethodSignatureDoc {
                        name: method.name.clone(),
                        description: None,
                        params: Vec::new(),
                        return_type: method.return_type.as_ref().map(|t| self.type_to_string(t)),
                    };

                    for param in &method.params {
                        sig_doc.params.push(ParamDoc {
                            name: param.name.clone(),
                            ty: param.ty.as_ref().map(|t| self.type_to_string(t)),
                            description: None,
                            has_default: param.default.is_some(),
                        });
                    }

                    if let Some(comment) = &method.doc_comment {
                        let parsed = DocCommentParser::parse(comment);
                        sig_doc.description = parsed.description;

                        for doc_param in parsed.params {
                            if let Some(ast_param) =
                                sig_doc.params.iter_mut().find(|p| p.name == doc_param.name)
                            {
                                ast_param.description = doc_param.description;
                            }
                        }
                    }

                    iface_doc.methods.push(sig_doc);
                }

                Some(DocItem::Interface(iface_doc))
            }

            StmtKind::EnumDecl {
                name,
                type_params,
                variants,
                doc_comment,
            } => {
                let mut enum_doc = EnumDoc::new(name.clone());
                enum_doc.type_params = type_params.clone();
                enum_doc.is_exported = is_exported;
                enum_doc.line = stmt.span.line;

                if let Some(comment) = doc_comment {
                    let parsed = DocCommentParser::parse(comment);
                    enum_doc.description = parsed.description;
                }

                for variant in variants {
                    let variant_doc = EnumVariantDoc {
                        name: variant.name.clone(),
                        description: None,
                        fields: variant
                            .fields
                            .iter()
                            .map(|f| EnumFieldDoc {
                                name: f.name.clone(),
                                ty: self.type_to_string(&f.ty),
                                description: None,
                            })
                            .collect(),
                        discriminant: variant.discriminant,
                    };
                    enum_doc.variants.push(variant_doc);
                }

                Some(DocItem::Enum(enum_doc))
            }

            StmtKind::Export(export_items) => {
                use crate::parser::ExportItems;
                match export_items {
                    ExportItems::Declaration(inner) => self.extract_stmt(inner, true),
                    // Named exports, wildcards, and re-exports don't have their own documentation
                    // They re-export items documented elsewhere
                    ExportItems::Named(_)
                    | ExportItems::Wildcard { .. }
                    | ExportItems::NamedReexport { .. } => None,
                }
            }

            _ => None,
        }
    }

    fn type_to_string(&self, ty: &TypeAnnotation) -> String {
        match &ty.kind {
            TypeKind::Simple(name) => name.clone(),
            TypeKind::Array(inner) => format!("مصفوفة<{}>", self.type_to_string(inner)),
            TypeKind::Map(key, val) => {
                format!(
                    "قاموس<{}، {}>",
                    self.type_to_string(key),
                    self.type_to_string(val)
                )
            }
            TypeKind::Function {
                params,
                return_type,
            } => {
                let params_str: Vec<_> = params.iter().map(|p| self.type_to_string(p)).collect();
                match return_type {
                    None => format!("({})", params_str.join("، ")),
                    Some(rt) => {
                        format!("({}) -> {}", params_str.join("، "), self.type_to_string(rt))
                    }
                }
            }
            TypeKind::Generic { base, args } => {
                let args_str: Vec<_> = args.iter().map(|a| self.type_to_string(a)).collect();
                format!("{}<{}>", base, args_str.join("، "))
            }
            TypeKind::Optional(inner) => format!("{}?", self.type_to_string(inner)),
        }
    }

    fn visibility_to_string(&self, vis: Visibility) -> String {
        match vis {
            Visibility::Public => "عام".to_string(),
            Visibility::Private => "خاص".to_string(),
            Visibility::Protected => "محمي".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn wrap_with_markers(source: &str) -> String {
        format!("بسم_الله\n{}\nالحمد_لله", source.trim())
    }

    #[test]
    fn test_extract_function() {
        let source = r#"
            دالة جمع(أ: عدد، ب: عدد) -> عدد {
                أرجع أ + ب;
            }
        "#;

        let wrapped = wrap_with_markers(source);
        let mut parser = Parser::new(&wrapped);
        let ast = parser.parse().unwrap();
        let extractor = DocExtractor::new("test".to_string(), "test.ترقيم".to_string());
        let doc = extractor.extract(&ast);

        assert_eq!(doc.items.len(), 1);
        if let DocItem::Function(func) = &doc.items[0] {
            assert_eq!(func.name, "جمع");
            // description is None when no doc comment is present
            assert_eq!(func.params.len(), 2);
            // param descriptions are None when no doc comment is present
            assert!(func.returns.is_some());
        } else {
            panic!("Expected Function");
        }
    }

    #[test]
    fn test_extract_class() {
        let source = r#"
            صنف شخص {
                خاص اسم: نص;

                منشئ(اسم: نص) {
                    هذا.اسم = اسم;
                }

                عام دالة احصل_اسم() -> نص {
                    أرجع هذا.اسم;
                }
            }
        "#;

        let wrapped = wrap_with_markers(source);
        let mut parser = Parser::new(&wrapped);
        let ast = parser.parse().unwrap();
        let extractor = DocExtractor::new("test".to_string(), "test.ترقيم".to_string());
        let doc = extractor.extract(&ast);

        assert_eq!(doc.items.len(), 1);
        if let DocItem::Class(class) = &doc.items[0] {
            assert_eq!(class.name, "شخص");
            // description is None when no doc comment is present
            assert_eq!(class.fields.len(), 1);
            assert!(class.constructor.is_some());
            assert_eq!(class.methods.len(), 1);
        } else {
            panic!("Expected Class");
        }
    }

    #[test]
    fn test_extract_exported_function() {
        let source = r#"
            صدّر دالة اختبار() {
            }
        "#;

        let wrapped = wrap_with_markers(source);
        let mut parser = Parser::new(&wrapped);
        let ast = parser.parse().unwrap();
        let extractor = DocExtractor::new("test".to_string(), "test.ترقيم".to_string());
        let doc = extractor.extract(&ast);

        assert_eq!(doc.items.len(), 1);
        if let DocItem::Function(func) = &doc.items[0] {
            assert!(func.is_exported);
        } else {
            panic!("Expected Function");
        }
    }

    /// `Documentation.description` is rendered by the markdown and HTML
    /// generators but had no source at all until the parser started keeping a
    /// file's own doc comment, so every generated page had an empty summary.
    #[test]
    fn test_module_doc_becomes_documentation_description() {
        let source = r#"
            /// وحدة الرياضيات
            ///
            /// @منذ ١.٠.٠

            // ═══

            /// القيمة المطلقة
            صدّر دالة مطلق(س: عدد) -> عدد {
                أرجع س
            }
        "#;

        let wrapped = wrap_with_markers(source);
        let mut parser = Parser::new(&wrapped);
        let ast = parser.parse().unwrap();
        let extractor = DocExtractor::new("رياضيات".to_string(), "رياضيات.ترقيم".to_string());
        let doc = extractor.extract(&ast);

        assert_eq!(doc.description.as_deref(), Some("وحدة الرياضيات"));
        // The file's doc must not have been stolen from the function.
        assert_eq!(doc.items.len(), 1);
        if let DocItem::Function(func) = &doc.items[0] {
            assert_eq!(func.description.as_deref(), Some("القيمة المطلقة"));
        } else {
            panic!("Expected Function");
        }
    }
}
