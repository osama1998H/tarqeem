//! Source mapping for IR instructions
//!
//! This module provides a mapping between IR instructions and their original
//! source locations, enabling the debugger to correlate execution with source code.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::Span;
use crate::ir::{BlockId, FuncId, VarId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub span: Span,
}

impl SourceLocation {
    pub fn new(file: PathBuf, span: Span) -> Self {
        Self {
            file,
            line: span.line,
            column: span.column,
            span,
        }
    }

    pub fn from_position(file: PathBuf, line: usize, column: usize) -> Self {
        Self {
            file,
            line,
            column,
            span: Span::new(0, 0, line, column),
        }
    }
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file.display(), self.line, self.column)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InstructionKey {
    pub func_id: String,
    pub block_id: u32,
    pub inst_idx: usize,
}

impl InstructionKey {
    pub fn new(func_id: &FuncId, block_id: BlockId, inst_idx: usize) -> Self {
        Self {
            func_id: func_id.0.clone(),
            block_id: block_id.0,
            inst_idx,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariableKey {
    pub func_id: String,
    pub var_id: u32,
}

impl VariableKey {
    pub fn new(func_id: &FuncId, var_id: VarId) -> Self {
        Self {
            func_id: func_id.0.clone(),
            var_id: var_id.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VariableInfo {
    pub name: String,
    pub name_ar: Option<String>,
    pub type_name: String,
    pub is_mutable: bool,
    pub decl_location: Option<SourceLocation>,
}

impl VariableInfo {
    pub fn new(name: String, type_name: String, is_mutable: bool) -> Self {
        Self {
            name,
            name_ar: None,
            type_name,
            is_mutable,
            decl_location: None,
        }
    }

    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.decl_location = Some(location);
        self
    }
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub name_ar: Option<String>,
    pub decl_location: Option<SourceLocation>,
    pub param_names: Vec<String>,
    pub return_type: String,
}

impl FunctionInfo {
    pub fn new(name: String, param_names: Vec<String>, return_type: String) -> Self {
        Self {
            name,
            name_ar: None,
            decl_location: None,
            param_names,
            return_type,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    instruction_map: HashMap<InstructionKey, SourceLocation>,

    line_map: HashMap<(usize, PathBuf), Vec<InstructionKey>>,

    variable_map: HashMap<VariableKey, VariableInfo>,

    function_map: HashMap<String, FunctionInfo>,

    source_files: Vec<PathBuf>,

    source_content: HashMap<PathBuf, String>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_source(&mut self, file: PathBuf, content: String) {
        if !self.source_files.contains(&file) {
            self.source_files.push(file.clone());
        }
        self.source_content.insert(file, content);
    }

    pub fn add_instruction(
        &mut self,
        func_id: &FuncId,
        block_id: BlockId,
        inst_idx: usize,
        location: SourceLocation,
    ) {
        let key = InstructionKey::new(func_id, block_id, inst_idx);

        let line_key = (location.line, location.file.clone());
        self.line_map.entry(line_key).or_default().push(key.clone());

        self.instruction_map.insert(key, location);
    }

    pub fn add_variable(&mut self, func_id: &FuncId, var_id: VarId, info: VariableInfo) {
        let key = VariableKey::new(func_id, var_id);
        self.variable_map.insert(key, info);
    }

    pub fn add_function(&mut self, func_id: &FuncId, info: FunctionInfo) {
        self.function_map.insert(func_id.0.clone(), info);
    }

    pub fn get_instruction_location(
        &self,
        func_id: &FuncId,
        block_id: BlockId,
        inst_idx: usize,
    ) -> Option<&SourceLocation> {
        let key = InstructionKey::new(func_id, block_id, inst_idx);
        self.instruction_map.get(&key)
    }

    pub fn get_instructions_at_line(&self, file: &Path, line: usize) -> Vec<&InstructionKey> {
        let key = (line, file.to_path_buf());
        self.line_map
            .get(&key)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    pub fn get_variable_info(&self, func_id: &FuncId, var_id: VarId) -> Option<&VariableInfo> {
        let key = VariableKey::new(func_id, var_id);
        self.variable_map.get(&key)
    }

    pub fn find_variable_by_name(&self, func_id: &FuncId, name: &str) -> Option<VarId> {
        for (key, info) in &self.variable_map {
            if key.func_id == func_id.0 && info.name == name {
                return Some(VarId(key.var_id));
            }
            if key.func_id == func_id.0 {
                if let Some(ref name_ar) = info.name_ar {
                    if name_ar == name {
                        return Some(VarId(key.var_id));
                    }
                }
            }
        }
        None
    }

    pub fn get_function_info(&self, func_id: &FuncId) -> Option<&FunctionInfo> {
        self.function_map.get(&func_id.0)
    }

    pub fn get_source_line(&self, file: &PathBuf, line: usize) -> Option<&str> {
        self.source_content
            .get(file)
            .and_then(|content| content.lines().nth(line.saturating_sub(1)))
    }

    pub fn source_files(&self) -> &[PathBuf] {
        &self.source_files
    }

    pub fn is_empty(&self) -> bool {
        self.instruction_map.is_empty()
    }

    pub fn instruction_count(&self) -> usize {
        self.instruction_map.len()
    }

    pub fn get_valid_breakpoint_lines(&self, file: &PathBuf) -> Vec<usize> {
        let mut lines: Vec<usize> = self
            .line_map
            .keys()
            .filter(|(_, f)| f == file)
            .map(|(line, _)| *line)
            .collect();
        lines.sort();
        lines.dedup();
        lines
    }

    pub fn find_nearest_breakpoint_line(
        &self,
        file: &PathBuf,
        target_line: usize,
    ) -> Option<usize> {
        let valid_lines = self.get_valid_breakpoint_lines(file);

        if valid_lines.is_empty() {
            return None;
        }

        for &line in &valid_lines {
            if line >= target_line {
                return Some(line);
            }
        }

        valid_lines.last().copied()
    }

    pub fn merge(&mut self, other: SourceMap) {
        for (key, loc) in other.instruction_map {
            self.instruction_map.insert(key.clone(), loc.clone());

            let line_key = (loc.line, loc.file.clone());
            self.line_map.entry(line_key).or_default().push(key);
        }

        for (key, info) in other.variable_map {
            self.variable_map.insert(key, info);
        }

        for (id, info) in other.function_map {
            self.function_map.insert(id, info);
        }

        for file in other.source_files {
            if !self.source_files.contains(&file) {
                self.source_files.push(file);
            }
        }

        for (file, content) in other.source_content {
            self.source_content.entry(file).or_insert(content);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_location() {
        let span = Span::new(0, 10, 5, 1);
        let loc = SourceLocation::new(PathBuf::from("test.trq"), span);

        assert_eq!(loc.line, 5);
        assert_eq!(loc.column, 1);
        assert_eq!(format!("{}", loc), "test.trq:5:1");
    }

    #[test]
    fn test_source_map() {
        let mut map = SourceMap::new();
        let file = PathBuf::from("test.trq");
        let func_id = FuncId("main".to_string());

        map.add_source(
            file.clone(),
            "دالة رئيسية() {\n  اطبع(\"مرحبا\")\n}".to_string(),
        );

        let span = Span::new(0, 10, 2, 3);
        let loc = SourceLocation::new(file.clone(), span);
        map.add_instruction(&func_id, BlockId(0), 0, loc);

        assert!(!map.is_empty());
        assert_eq!(map.instruction_count(), 1);

        let retrieved = map.get_instruction_location(&func_id, BlockId(0), 0);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().line, 2);

        let at_line = map.get_instructions_at_line(&file, 2);
        assert_eq!(at_line.len(), 1);
    }

    #[test]
    fn test_variable_info() {
        let mut map = SourceMap::new();
        let func_id = FuncId("main".to_string());

        let info = VariableInfo::new("x".to_string(), "int".to_string(), true);
        map.add_variable(&func_id, VarId(0), info);

        let retrieved = map.get_variable_info(&func_id, VarId(0));
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "x");
    }

    #[test]
    fn test_valid_breakpoint_lines() {
        let mut map = SourceMap::new();
        let file = PathBuf::from("test.trq");
        let func_id = FuncId("main".to_string());

        for (line, idx) in [(2, 0), (5, 1), (5, 2), (10, 3)] {
            let span = Span::new(0, 10, line, 1);
            let loc = SourceLocation::new(file.clone(), span);
            map.add_instruction(&func_id, BlockId(0), idx, loc);
        }

        let lines = map.get_valid_breakpoint_lines(&file);
        assert_eq!(lines, vec![2, 5, 10]);
    }

    #[test]
    fn test_nearest_breakpoint_line() {
        let mut map = SourceMap::new();
        let file = PathBuf::from("test.trq");
        let func_id = FuncId("main".to_string());

        for (line, idx) in [(5, 0), (10, 1), (15, 2)] {
            let span = Span::new(0, 10, line, 1);
            let loc = SourceLocation::new(file.clone(), span);
            map.add_instruction(&func_id, BlockId(0), idx, loc);
        }

        assert_eq!(map.find_nearest_breakpoint_line(&file, 1), Some(5));
        assert_eq!(map.find_nearest_breakpoint_line(&file, 5), Some(5));
        assert_eq!(map.find_nearest_breakpoint_line(&file, 7), Some(10));
        assert_eq!(map.find_nearest_breakpoint_line(&file, 20), Some(15));
    }
}
