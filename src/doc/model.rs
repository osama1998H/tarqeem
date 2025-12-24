//! Documentation model for Tarqeem
//!
//! This module defines the data structures that represent extracted documentation.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Documentation {
    pub name: String,
    pub description: Option<String>,
    pub source_path: String,
    pub items: Vec<DocItem>,
}

impl Documentation {
    pub fn new(name: String, source_path: String) -> Self {
        Self {
            name,
            description: None,
            source_path,
            items: Vec::new(),
        }
    }

    pub fn functions(&self) -> impl Iterator<Item = &FunctionDoc> {
        self.items.iter().filter_map(|item| {
            if let DocItem::Function(f) = item {
                Some(f)
            } else {
                None
            }
        })
    }

    pub fn classes(&self) -> impl Iterator<Item = &ClassDoc> {
        self.items.iter().filter_map(|item| {
            if let DocItem::Class(c) = item {
                Some(c)
            } else {
                None
            }
        })
    }

    pub fn interfaces(&self) -> impl Iterator<Item = &InterfaceDoc> {
        self.items.iter().filter_map(|item| {
            if let DocItem::Interface(i) = item {
                Some(i)
            } else {
                None
            }
        })
    }

    pub fn variables(&self) -> impl Iterator<Item = &VariableDoc> {
        self.items.iter().filter_map(|item| {
            if let DocItem::Variable(v) = item {
                Some(v)
            } else {
                None
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DocItem {
    Function(FunctionDoc),
    Class(ClassDoc),
    Interface(InterfaceDoc),
    Variable(VariableDoc),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDoc {
    pub name: String,
    pub name_ar: Option<String>,
    pub description: Option<String>,
    pub params: Vec<ParamDoc>,
    pub returns: Option<ReturnDoc>,
    pub examples: Vec<String>,
    pub see_also: Vec<String>,
    pub is_async: bool,
    pub is_exported: bool,
    pub line: usize,
}

impl FunctionDoc {
    pub fn new(name: String) -> Self {
        Self {
            name,
            name_ar: None,
            description: None,
            params: Vec::new(),
            returns: None,
            examples: Vec::new(),
            see_also: Vec::new(),
            is_async: false,
            is_exported: false,
            line: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDoc {
    pub name: String,
    pub ty: Option<String>,
    pub description: Option<String>,
    pub has_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnDoc {
    pub ty: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDoc {
    pub name: String,
    pub description: Option<String>,
    pub type_params: Vec<String>,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub fields: Vec<FieldDoc>,
    pub methods: Vec<MethodDoc>,
    pub constructor: Option<ConstructorDoc>,
    pub examples: Vec<String>,
    pub is_exported: bool,
    pub line: usize,
}

impl ClassDoc {
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            type_params: Vec::new(),
            extends: None,
            implements: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            constructor: None,
            examples: Vec::new(),
            is_exported: false,
            line: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDoc {
    pub name: String,
    pub ty: Option<String>,
    pub description: Option<String>,
    pub visibility: String,
    pub is_static: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodDoc {
    pub name: String,
    pub description: Option<String>,
    pub params: Vec<ParamDoc>,
    pub returns: Option<ReturnDoc>,
    pub visibility: String,
    pub is_static: bool,
    pub is_async: bool,
}

impl MethodDoc {
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            params: Vec::new(),
            returns: None,
            visibility: "عام".to_string(),
            is_static: false,
            is_async: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructorDoc {
    pub description: Option<String>,
    pub params: Vec<ParamDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceDoc {
    pub name: String,
    pub description: Option<String>,
    pub type_params: Vec<String>,
    pub methods: Vec<MethodSignatureDoc>,
    pub is_exported: bool,
    pub line: usize,
}

impl InterfaceDoc {
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            type_params: Vec::new(),
            methods: Vec::new(),
            is_exported: false,
            line: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodSignatureDoc {
    pub name: String,
    pub description: Option<String>,
    pub params: Vec<ParamDoc>,
    pub return_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableDoc {
    pub name: String,
    pub ty: Option<String>,
    pub description: Option<String>,
    pub is_mutable: bool,
    pub is_exported: bool,
    pub line: usize,
}

impl VariableDoc {
    pub fn new(name: String) -> Self {
        Self {
            name,
            ty: None,
            description: None,
            is_mutable: true,
            is_exported: false,
            line: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_documentation_new() {
        let doc = Documentation::new("test".to_string(), "test.trq".to_string());
        assert_eq!(doc.name, "test");
        assert!(doc.items.is_empty());
    }

    #[test]
    fn test_function_doc() {
        let mut func = FunctionDoc::new("جمع".to_string());
        func.description = Some("دالة لجمع عددين".to_string());
        func.params.push(ParamDoc {
            name: "أ".to_string(),
            ty: Some("عدد".to_string()),
            description: Some("العدد الأول".to_string()),
            has_default: false,
        });
        func.params.push(ParamDoc {
            name: "ب".to_string(),
            ty: Some("عدد".to_string()),
            description: Some("العدد الثاني".to_string()),
            has_default: false,
        });
        func.returns = Some(ReturnDoc {
            ty: Some("عدد".to_string()),
            description: Some("مجموع العددين".to_string()),
        });

        assert_eq!(func.name, "جمع");
        assert_eq!(func.params.len(), 2);
        assert!(func.returns.is_some());
    }

    #[test]
    fn test_class_doc() {
        let mut class = ClassDoc::new("شخص".to_string());
        class.description = Some("صنف يمثل شخص".to_string());
        class.fields.push(FieldDoc {
            name: "اسم".to_string(),
            ty: Some("نص".to_string()),
            description: Some("اسم الشخص".to_string()),
            visibility: "خاص".to_string(),
            is_static: false,
        });

        assert_eq!(class.name, "شخص");
        assert_eq!(class.fields.len(), 1);
    }
}
