//! Documentation model for Tarqeem
//!
//! This module defines the data structures that represent extracted documentation.

use serde::{Deserialize, Serialize};

/// Complete documentation for a module/file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Documentation {
    /// Module/file name
    pub name: String,
    /// Module-level description
    pub description: Option<String>,
    /// Source file path
    pub source_path: String,
    /// All documented items
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

    /// Get all functions in the documentation
    pub fn functions(&self) -> impl Iterator<Item = &FunctionDoc> {
        self.items.iter().filter_map(|item| {
            if let DocItem::Function(f) = item {
                Some(f)
            } else {
                None
            }
        })
    }

    /// Get all classes in the documentation
    pub fn classes(&self) -> impl Iterator<Item = &ClassDoc> {
        self.items.iter().filter_map(|item| {
            if let DocItem::Class(c) = item {
                Some(c)
            } else {
                None
            }
        })
    }

    /// Get all interfaces in the documentation
    pub fn interfaces(&self) -> impl Iterator<Item = &InterfaceDoc> {
        self.items.iter().filter_map(|item| {
            if let DocItem::Interface(i) = item {
                Some(i)
            } else {
                None
            }
        })
    }

    /// Get all variables/constants in the documentation
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

/// A documented item (function, class, interface, or variable)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DocItem {
    Function(FunctionDoc),
    Class(ClassDoc),
    Interface(InterfaceDoc),
    Variable(VariableDoc),
}

/// Documentation for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDoc {
    /// Function name
    pub name: String,
    /// Arabic name (for bilingual support)
    pub name_ar: Option<String>,
    /// Description extracted from doc comment
    pub description: Option<String>,
    /// Parameter documentation
    pub params: Vec<ParamDoc>,
    /// Return type documentation
    pub returns: Option<ReturnDoc>,
    /// Example code
    pub examples: Vec<String>,
    /// Related items
    pub see_also: Vec<String>,
    /// Whether the function is async
    pub is_async: bool,
    /// Whether the function is exported
    pub is_exported: bool,
    /// Source location (line number)
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

/// Documentation for a parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDoc {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub ty: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Whether the parameter has a default value
    pub has_default: bool,
}

/// Documentation for a return value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnDoc {
    /// Return type
    pub ty: Option<String>,
    /// Description of what is returned
    pub description: Option<String>,
}

/// Documentation for a class
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDoc {
    /// Class name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Generic type parameters
    pub type_params: Vec<String>,
    /// Parent class (if extends)
    pub extends: Option<String>,
    /// Implemented interfaces
    pub implements: Vec<String>,
    /// Fields
    pub fields: Vec<FieldDoc>,
    /// Methods
    pub methods: Vec<MethodDoc>,
    /// Constructor documentation
    pub constructor: Option<ConstructorDoc>,
    /// Example code
    pub examples: Vec<String>,
    /// Whether the class is exported
    pub is_exported: bool,
    /// Source location (line number)
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

/// Documentation for a class field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDoc {
    /// Field name
    pub name: String,
    /// Field type
    pub ty: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Visibility (عام, خاص, محمي)
    pub visibility: String,
    /// Whether the field is static
    pub is_static: bool,
}

/// Documentation for a method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodDoc {
    /// Method name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Parameters
    pub params: Vec<ParamDoc>,
    /// Return documentation
    pub returns: Option<ReturnDoc>,
    /// Visibility
    pub visibility: String,
    /// Whether the method is static
    pub is_static: bool,
    /// Whether the method is async
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

/// Documentation for a constructor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructorDoc {
    /// Description
    pub description: Option<String>,
    /// Parameters
    pub params: Vec<ParamDoc>,
}

/// Documentation for an interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceDoc {
    /// Interface name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Generic type parameters
    pub type_params: Vec<String>,
    /// Method signatures
    pub methods: Vec<MethodSignatureDoc>,
    /// Whether the interface is exported
    pub is_exported: bool,
    /// Source location (line number)
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

/// Documentation for a method signature in an interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodSignatureDoc {
    /// Method name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Parameters
    pub params: Vec<ParamDoc>,
    /// Return type
    pub return_type: Option<String>,
}

/// Documentation for a variable or constant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableDoc {
    /// Variable name
    pub name: String,
    /// Type
    pub ty: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Whether it's mutable (متغير) or constant (ثابت)
    pub is_mutable: bool,
    /// Whether it's exported
    pub is_exported: bool,
    /// Source location (line number)
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
