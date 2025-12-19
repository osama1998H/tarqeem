//! Scope and symbol table management

use super::types::Type;
use indexmap::IndexMap;

/// A symbol in the symbol table
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub ty: Type,
    pub mutable: bool,
    pub defined: bool,
}

impl Symbol {
    pub fn new(name: impl Into<String>, kind: SymbolKind, ty: Type) -> Self {
        Self {
            name: name.into(),
            kind,
            ty,
            mutable: true,
            defined: true,
        }
    }

    pub fn variable(name: impl Into<String>, ty: Type, mutable: bool) -> Self {
        Self {
            name: name.into(),
            kind: SymbolKind::Variable,
            ty,
            mutable,
            defined: true,
        }
    }

    pub fn function(name: impl Into<String>, params: Vec<Type>, return_type: Type) -> Self {
        Self {
            name: name.into(),
            kind: SymbolKind::Function,
            ty: Type::Function {
                params,
                return_type: Box::new(return_type),
            },
            mutable: false,
            defined: true,
        }
    }

    pub fn class(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            name: name.clone(),
            kind: SymbolKind::Class,
            ty: Type::Class(name),
            mutable: false,
            defined: true,
        }
    }
}

/// The kind of symbol
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,
    Function,
    Class,
    Interface,
    Parameter,
}

/// A scope in the program
#[derive(Debug)]
pub struct Scope {
    /// Symbols defined in this scope
    symbols: IndexMap<String, Symbol>,
    /// Parent scope (if any)
    parent: Option<Box<Scope>>,
    /// Scope kind
    kind: ScopeKind,
}

/// The kind of scope
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScopeKind {
    Global,
    Function,
    Block,
    Class,
    Loop,
}

impl Scope {
    /// Create a new global scope
    pub fn new_global() -> Self {
        let mut scope = Self {
            symbols: IndexMap::new(),
            parent: None,
            kind: ScopeKind::Global,
        };

        // Add built-in functions
        scope.define(Symbol::function("اطبع", vec![Type::Any], Type::Void));
        scope.define(Symbol::function("print", vec![Type::Any], Type::Void));
        scope.define(Symbol::function("طباعة", vec![Type::Any], Type::Void));

        scope.define(Symbol::function("طول", vec![Type::Any], Type::Int));
        scope.define(Symbol::function("len", vec![Type::Any], Type::Int));
        scope.define(Symbol::function("length", vec![Type::Any], Type::Int));

        scope.define(Symbol::function("نوع", vec![Type::Any], Type::String));
        scope.define(Symbol::function("type", vec![Type::Any], Type::String));
        scope.define(Symbol::function("typeof", vec![Type::Any], Type::String));

        scope
    }

    /// Create a new child scope
    pub fn new_child(parent: Scope, kind: ScopeKind) -> Self {
        Self {
            symbols: IndexMap::new(),
            parent: Some(Box::new(parent)),
            kind,
        }
    }

    /// Get the scope kind
    pub fn kind(&self) -> ScopeKind {
        self.kind
    }

    /// Define a new symbol in this scope
    pub fn define(&mut self, symbol: Symbol) -> bool {
        if self.symbols.contains_key(&symbol.name) {
            false
        } else {
            self.symbols.insert(symbol.name.clone(), symbol);
            true
        }
    }

    /// Look up a symbol in this scope or parent scopes
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        if let Some(symbol) = self.symbols.get(name) {
            Some(symbol)
        } else if let Some(parent) = &self.parent {
            parent.lookup(name)
        } else {
            None
        }
    }

    /// Look up a symbol only in this scope (not parents)
    pub fn lookup_local(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    /// Get a mutable reference to a symbol
    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        if self.symbols.contains_key(name) {
            self.symbols.get_mut(name)
        } else if let Some(parent) = &mut self.parent {
            parent.lookup_mut(name)
        } else {
            None
        }
    }

    /// Check if we're inside a loop
    pub fn is_in_loop(&self) -> bool {
        if self.kind == ScopeKind::Loop {
            true
        } else if let Some(parent) = &self.parent {
            parent.is_in_loop()
        } else {
            false
        }
    }

    /// Check if we're inside a function
    pub fn is_in_function(&self) -> bool {
        if self.kind == ScopeKind::Function {
            true
        } else if let Some(parent) = &self.parent {
            parent.is_in_function()
        } else {
            false
        }
    }

    /// Check if we're inside a class
    pub fn is_in_class(&self) -> bool {
        if self.kind == ScopeKind::Class {
            true
        } else if let Some(parent) = &self.parent {
            parent.is_in_class()
        } else {
            false
        }
    }

    /// Get the return type of the enclosing function
    pub fn get_function_return_type(&self) -> Option<Type> {
        if self.kind == ScopeKind::Function {
            // The function return type should be stored somewhere
            // For now, return Unknown
            Some(Type::Unknown)
        } else if let Some(parent) = &self.parent {
            parent.get_function_return_type()
        } else {
            None
        }
    }

    /// Pop this scope and return the parent
    pub fn pop(self) -> Option<Scope> {
        self.parent.map(|p| *p)
    }

    /// Get all symbols in this scope
    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_lookup() {
        let mut global = Scope::new_global();
        global.define(Symbol::variable("x", Type::Int, true));

        let child = Scope::new_child(global, ScopeKind::Block);

        assert!(child.lookup("x").is_some());
        assert!(child.lookup("اطبع").is_some());
        assert!(child.lookup("nonexistent").is_none());
    }

    #[test]
    fn test_scope_shadowing() {
        let mut global = Scope::new_global();
        global.define(Symbol::variable("x", Type::Int, true));

        let mut child = Scope::new_child(global, ScopeKind::Block);
        child.define(Symbol::variable("x", Type::String, true));

        let symbol = child.lookup("x").unwrap();
        assert_eq!(symbol.ty, Type::String);
    }

    #[test]
    fn test_loop_detection() {
        let global = Scope::new_global();
        let func = Scope::new_child(global, ScopeKind::Function);
        let loop_scope = Scope::new_child(func, ScopeKind::Loop);
        let block = Scope::new_child(loop_scope, ScopeKind::Block);

        assert!(block.is_in_loop());
        assert!(block.is_in_function());
    }
}
