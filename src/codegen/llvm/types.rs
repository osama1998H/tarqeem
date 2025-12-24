//! Type Mapping from Tarqeem IR to LLVM Types
//!
//! This module handles the conversion of Tarqeem IR types to LLVM IR type strings.

use crate::ir::{ClassId, IrType};
use std::collections::HashMap;

pub struct TypeMapper {
    pointer_bits: u32,
    struct_types: HashMap<String, String>,
    struct_fields: HashMap<String, Vec<IrType>>,
}

impl TypeMapper {
    pub fn new(pointer_bits: u32) -> Self {
        Self {
            pointer_bits,
            struct_types: HashMap::new(),
            struct_fields: HashMap::new(),
        }
    }

    pub fn map_type(&self, ty: &IrType) -> String {
        match ty {
            IrType::Void => "void".to_string(),
            IrType::Bool => "i1".to_string(),
            IrType::Int => "i64".to_string(),
            IrType::Float => "double".to_string(),
            IrType::String => "ptr".to_string(), // Opaque pointer to string struct
            IrType::Ptr(_) => {
                // LLVM 15+ uses opaque pointers
                "ptr".to_string()
            }
            IrType::Array(_, _) => {
                // Our runtime uses pointer-based dynamic arrays
                "ptr".to_string()
            }
            IrType::Function { params, ret } => {
                let param_types: Vec<String> = params.iter().map(|p| self.map_type(p)).collect();
                format!("{} ({})", self.map_type(ret), param_types.join(", "))
            }
            IrType::Struct(class_id) => {
                format!("%class.{}", mangle_name(&class_id.0))
            }
        }
    }

    pub fn map_param_type(&self, ty: &IrType) -> String {
        match ty {
            // Pass strings by pointer
            IrType::String => "ptr".to_string(),
            // Pass structs by pointer
            IrType::Struct(_) => "ptr".to_string(),
            // Pass arrays by pointer
            IrType::Array(_, _) => "ptr".to_string(),
            // Other types passed directly
            _ => self.map_type(ty),
        }
    }

    pub fn type_size(&self, ty: &IrType) -> u64 {
        match ty {
            IrType::Void => 0,
            IrType::Bool => 1,
            IrType::Int => 8,
            IrType::Float => 8,
            IrType::String => self.pointer_bits as u64 / 8,
            IrType::Ptr(_) => self.pointer_bits as u64 / 8,
            IrType::Array(elem, size) => self.type_size(elem) * (*size as u64),
            IrType::Function { .. } => self.pointer_bits as u64 / 8,
            IrType::Struct(class_id) => {
                // Calculate struct size from registered field types
                if let Some(field_types) = self.struct_fields.get(&class_id.0) {
                    let mut total_size = 0u64;
                    for field_ty in field_types {
                        // Add alignment padding if needed
                        let field_align = self.type_align(field_ty);
                        if field_align > 0 {
                            let padding = (field_align - (total_size % field_align)) % field_align;
                            total_size += padding;
                        }
                        total_size += self.type_size(field_ty);
                    }
                    // Ensure minimum size of pointer
                    if total_size == 0 {
                        self.pointer_bits as u64 / 8
                    } else {
                        total_size
                    }
                } else {
                    // Fallback to pointer size if struct not registered
                    self.pointer_bits as u64 / 8
                }
            }
        }
    }

    pub fn type_align(&self, ty: &IrType) -> u64 {
        match ty {
            IrType::Void => 1,
            IrType::Bool => 1,
            IrType::Int => 8,
            IrType::Float => 8,
            IrType::String => self.pointer_bits as u64 / 8,
            IrType::Ptr(_) => self.pointer_bits as u64 / 8,
            IrType::Array(elem, _) => self.type_align(elem),
            IrType::Function { .. } => self.pointer_bits as u64 / 8,
            IrType::Struct(_) => self.pointer_bits as u64 / 8,
        }
    }

    pub fn generate_struct_type(
        &mut self,
        class_id: &ClassId,
        fields: &[(String, IrType)],
    ) -> String {
        let mangled_name = mangle_name(&class_id.0);
        let field_types: Vec<String> = fields.iter().map(|(_, ty)| self.map_type(ty)).collect();
        let type_def = format!(
            "%class.{} = type {{ {} }}",
            mangled_name,
            field_types.join(", ")
        );
        self.struct_types
            .insert(class_id.0.clone(), type_def.clone());
        // Store field types for size calculation
        let ir_field_types: Vec<IrType> = fields.iter().map(|(_, ty)| ty.clone()).collect();
        self.struct_fields
            .insert(class_id.0.clone(), ir_field_types);
        type_def
    }

    pub fn string_struct_type() -> &'static str {
        // String structure: { i64 len, i64 cap, ptr data }
        "%struct.TrqString = type { i64, i64, ptr }"
    }

    pub fn array_struct_type() -> &'static str {
        // Array structure: { i64 len, i64 cap, ptr data }
        "%struct.TrqArray = type { i64, i64, ptr }"
    }

    pub fn zero_init(&self, ty: &IrType) -> String {
        match ty {
            IrType::Void => "void".to_string(),
            IrType::Bool => "false".to_string(),
            IrType::Int => "0".to_string(),
            IrType::Float => "0.0".to_string(),
            IrType::String => "null".to_string(),
            IrType::Ptr(_) => "null".to_string(),
            IrType::Array(_, _) => "zeroinitializer".to_string(),
            IrType::Function { .. } => "null".to_string(),
            IrType::Struct(_) => "zeroinitializer".to_string(),
        }
    }
}

pub fn mangle_name(name: &str) -> String {
    let mut result = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            result.push(ch);
        } else {
            // Encode as _U followed by hex codepoint
            result.push_str(&format!("_U{:04X}_", ch as u32));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_mapping() {
        let mapper = TypeMapper::new(64);

        assert_eq!(mapper.map_type(&IrType::Void), "void");
        assert_eq!(mapper.map_type(&IrType::Bool), "i1");
        assert_eq!(mapper.map_type(&IrType::Int), "i64");
        assert_eq!(mapper.map_type(&IrType::Float), "double");
        assert_eq!(mapper.map_type(&IrType::String), "ptr");

        let arr_ty = IrType::Array(Box::new(IrType::Int), 10);
        assert_eq!(mapper.map_type(&arr_ty), "ptr"); // Runtime uses pointer-based arrays

        let class_ty = IrType::Struct(ClassId("Person".to_string()));
        assert_eq!(mapper.map_type(&class_ty), "%class.Person");
    }

    #[test]
    fn test_type_sizes() {
        let mapper = TypeMapper::new(64);

        assert_eq!(mapper.type_size(&IrType::Bool), 1);
        assert_eq!(mapper.type_size(&IrType::Int), 8);
        assert_eq!(mapper.type_size(&IrType::Float), 8);
        assert_eq!(mapper.type_size(&IrType::Ptr(Box::new(IrType::Int))), 8);
    }
}
