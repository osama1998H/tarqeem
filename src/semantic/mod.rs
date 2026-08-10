//! Semantic analysis for Tarqeem
//!
//! This module provides semantic analysis including:
//! - Name resolution
//! - Scope management
//! - Type checking
//! - Symbol table management
//! - Class hierarchy resolution
//! - Interface implementation validation
//! - Method resolution
//! - Generic type support
//! - Module loading and imports

mod analyzer;
mod class_resolver;
mod generics;
mod linker;
mod method_resolver;
mod modules;
mod scope;
mod types;

#[cfg(test)]
mod scope_tests;
#[cfg(test)]
mod types_tests;

pub use analyzer::Analyzer;
pub use class_resolver::{ClassInfo, ClassResolver, FieldInfo, InterfaceInfo, MethodInfo};
pub use generics::{GenericContext, GenericResolver};
pub use linker::link_program;
pub use method_resolver::{MemberResolution, MethodCallResolution, MethodResolver};
pub use modules::{ExportKind, ExportedSymbol, LoadedModule, ModuleId, ModuleLoader};
pub use scope::{Scope, Symbol, SymbolKind};
pub use types::{parse_type_name, Type};
