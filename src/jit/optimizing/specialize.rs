//! Type Specialization for JIT Compilation
//!
//! This module implements type specialization for generic functions
//! based on observed runtime types.
//!
//! التخصص النوعي للترجمة الفورية
//! يُنشئ نسخاً مُخصصة من الدوال المعممة بناءً على الأنماط الملاحظة

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::ir::{ClassId, Function, IrType};
use crate::jit::profile::ObservedType;

/// Key for looking up specialized function versions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecializationKey {
    /// Base function name
    pub func_name: String,

    /// Specialized type arguments
    pub type_args: Vec<IrType>,
}

impl Hash for SpecializationKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.func_name.hash(state);
        // Hash type arguments by their debug representation
        for ty in &self.type_args {
            format!("{:?}", ty).hash(state);
        }
    }
}

impl SpecializationKey {
    /// Create a new specialization key
    pub fn new(func_name: String, type_args: Vec<IrType>) -> Self {
        Self {
            func_name,
            type_args,
        }
    }

    /// Create a key from observed types
    pub fn from_observed(func_name: String, observed: &[ObservedType]) -> Option<Self> {
        let type_args: Vec<IrType> = observed
            .iter()
            .filter_map(|obs| obs.to_ir_type())
            .collect();

        // Only specialize if all types are monomorphic
        if type_args.len() == observed.len() {
            Some(Self::new(func_name, type_args))
        } else {
            None
        }
    }
}

/// Type specializer manages specialized function versions
pub struct TypeSpecializer {
    /// Cached specialized functions
    specializations: HashMap<SpecializationKey, Function>,

    /// Maximum number of specializations per function
    max_specializations: usize,

    /// Number of specializations per base function
    specialization_counts: HashMap<String, usize>,
}

impl TypeSpecializer {
    /// Create a new type specializer
    pub fn new() -> Self {
        Self {
            specializations: HashMap::new(),
            max_specializations: 4, // Limit to prevent code bloat
            specialization_counts: HashMap::new(),
        }
    }

    /// Create with custom max specializations
    pub fn with_max_specializations(max: usize) -> Self {
        Self {
            specializations: HashMap::new(),
            max_specializations: max,
            specialization_counts: HashMap::new(),
        }
    }

    /// Check if we should specialize a function
    pub fn should_specialize(&self, func_name: &str, observed: &[ObservedType]) -> bool {
        // Don't specialize if we've hit the limit
        let count = self.specialization_counts.get(func_name).unwrap_or(&0);
        if *count >= self.max_specializations {
            return false;
        }

        // Only specialize if all observed types are monomorphic
        observed.iter().all(|t| t.is_monomorphic())
    }

    /// Get or create a specialized version of a function
    pub fn get_or_specialize(
        &mut self,
        base_func: &Function,
        observed: &[ObservedType],
    ) -> Option<&Function> {
        let key = SpecializationKey::from_observed(base_func.name.clone(), observed)?;

        // Return existing specialization if available
        if self.specializations.contains_key(&key) {
            return self.specializations.get(&key);
        }

        // Check if we should create a new specialization
        if !self.should_specialize(&base_func.name, observed) {
            return None;
        }

        // Create specialized version
        let specialized = self.create_specialization(base_func, &key)?;

        // Update counts
        *self
            .specialization_counts
            .entry(base_func.name.clone())
            .or_insert(0) += 1;

        // Cache and return
        self.specializations.insert(key.clone(), specialized);
        self.specializations.get(&key)
    }

    /// Create a specialized version of a function
    fn create_specialization(
        &self,
        base_func: &Function,
        key: &SpecializationKey,
    ) -> Option<Function> {
        // Clone the base function
        let mut specialized = base_func.clone();

        // Create specialized name
        specialized.name = self.specialized_name(&key.func_name, &key.type_args);

        // Update parameter types if they match type arguments
        for (i, param) in specialized.params.iter_mut().enumerate() {
            if i < key.type_args.len() {
                param.ty = key.type_args[i].clone();
            }
        }

        Some(specialized)
    }

    /// Generate a unique name for a specialized function
    fn specialized_name(&self, base_name: &str, type_args: &[IrType]) -> String {
        let suffix: String = type_args
            .iter()
            .map(|ty| match ty {
                IrType::Int => "i",
                IrType::Float => "f",
                IrType::Bool => "b",
                IrType::String => "s",
                IrType::Void => "v",
                _ => "?",
            })
            .collect();

        format!("{}_{}", base_name, suffix)
    }

    /// Get statistics about specializations
    pub fn stats(&self) -> SpecializerStats {
        SpecializerStats {
            total_specializations: self.specializations.len(),
            functions_specialized: self.specialization_counts.len(),
            specializations_by_function: self.specialization_counts.clone(),
        }
    }

    /// Clear all cached specializations
    pub fn clear(&mut self) {
        self.specializations.clear();
        self.specialization_counts.clear();
    }
}

impl Default for TypeSpecializer {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about type specialization
#[derive(Debug, Clone)]
pub struct SpecializerStats {
    /// Total number of specialized functions
    pub total_specializations: usize,

    /// Number of base functions that have specializations
    pub functions_specialized: usize,

    /// Per-function specialization counts
    pub specializations_by_function: HashMap<String, usize>,
}

impl std::fmt::Display for SpecializerStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Type Specialization Statistics / إحصائيات التخصص النوعي"
        )?;
        writeln!(f, "=====================================================")?;
        writeln!(
            f,
            "Total specializations / إجمالي التخصصات: {}",
            self.total_specializations
        )?;
        writeln!(
            f,
            "Functions specialized / الدوال المُخصصة: {}",
            self.functions_specialized
        )?;
        Ok(())
    }
}

// Extension trait for ObservedType
impl ObservedType {
    /// Check if the observed type is monomorphic (single type)
    pub fn is_monomorphic(&self) -> bool {
        !matches!(self, ObservedType::Mixed | ObservedType::Unknown)
    }

    /// Convert observed type to IR type
    pub fn to_ir_type(&self) -> Option<IrType> {
        match self {
            ObservedType::AlwaysInt => Some(IrType::Int),
            ObservedType::AlwaysFloat => Some(IrType::Float),
            ObservedType::AlwaysBool => Some(IrType::Bool),
            ObservedType::AlwaysString => Some(IrType::String),
            ObservedType::AlwaysNull => Some(IrType::Void), // Approximate
            ObservedType::AlwaysObject(name) => Some(IrType::Struct(ClassId(name.clone()))),
            ObservedType::AlwaysArray => None, // Arrays need element type info
            ObservedType::Mixed => None,
            ObservedType::Unknown => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{FuncId, Parameter, VarId};

    fn create_test_function(name: &str) -> Function {
        Function::new(
            FuncId(name.to_string()),
            name.to_string(),
            vec![Parameter {
                id: VarId(0),
                name: "x".to_string(),
                ty: IrType::Int,
            }],
            IrType::Int,
        )
    }

    #[test]
    fn test_specialization_key() {
        let key1 = SpecializationKey::new("test".to_string(), vec![IrType::Int]);
        let key2 = SpecializationKey::new("test".to_string(), vec![IrType::Int]);
        let key3 = SpecializationKey::new("test".to_string(), vec![IrType::Float]);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_type_specializer_creation() {
        let specializer = TypeSpecializer::new();
        let stats = specializer.stats();

        assert_eq!(stats.total_specializations, 0);
        assert_eq!(stats.functions_specialized, 0);
    }

    #[test]
    fn test_should_specialize() {
        let specializer = TypeSpecializer::new();

        // Monomorphic types should be specialized
        let observed = vec![ObservedType::AlwaysInt];
        assert!(specializer.should_specialize("test", &observed));

        // Mixed types should not be specialized
        let mixed = vec![ObservedType::Mixed];
        assert!(!specializer.should_specialize("test", &mixed));
    }

    #[test]
    fn test_specialization_limit() {
        let mut specializer = TypeSpecializer::with_max_specializations(2);
        let func = create_test_function("test");

        // First two specializations should work
        let obs1 = vec![ObservedType::AlwaysInt];
        specializer.get_or_specialize(&func, &obs1);

        let obs2 = vec![ObservedType::AlwaysFloat];
        specializer.get_or_specialize(&func, &obs2);

        // Third should be blocked
        let obs3 = vec![ObservedType::AlwaysBool];
        assert!(!specializer.should_specialize("test", &obs3));
    }

    #[test]
    fn test_observed_type_conversion() {
        assert_eq!(ObservedType::AlwaysInt.to_ir_type(), Some(IrType::Int));
        assert_eq!(ObservedType::AlwaysFloat.to_ir_type(), Some(IrType::Float));
        assert_eq!(ObservedType::AlwaysBool.to_ir_type(), Some(IrType::Bool));
        assert_eq!(
            ObservedType::AlwaysString.to_ir_type(),
            Some(IrType::String)
        );
        assert_eq!(ObservedType::Mixed.to_ir_type(), None);
    }
}
