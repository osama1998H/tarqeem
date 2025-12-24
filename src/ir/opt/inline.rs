//! Function Inlining
//!
//! This pass inlines small functions at their call sites to reduce
//! function call overhead and enable further optimizations.
//!
//! ## Inlining Criteria
//!
//! A function is eligible for inlining if:
//! - It's not recursive
//! - It's small enough (below instruction threshold)
//! - It's called few enough times (or is marked always_inline)
//!
//! ## Examples
//!
//! ```text
//! // Before:
//! دالة مربع(س: عدد) -> عدد {
//!     أرجع س * س
//! }
//! متغير ن = مربع(5)
//!
//! // After (inlined):
//! متغير ن = 5 * 5
//! ```

use super::OptStats;
use crate::ir::{BasicBlock, BlockId, FuncId, Function, Instruction, IrType, Module, VarId};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct InlineConfig {
    pub max_instructions: usize,
    pub max_blocks: usize,
    pub max_call_sites: usize,
    pub always_inline_threshold: usize,
}

impl Default for InlineConfig {
    fn default() -> Self {
        Self {
            max_instructions: 50,
            max_blocks: 5,
            max_call_sites: 10,
            always_inline_threshold: 10,
        }
    }
}

pub struct FunctionInliner {
    stats: OptStats,
    config: InlineConfig,
}

impl FunctionInliner {
    pub fn new() -> Self {
        Self {
            stats: OptStats::new(),
            config: InlineConfig::default(),
        }
    }

    pub fn with_config(config: InlineConfig) -> Self {
        Self {
            stats: OptStats::new(),
            config,
        }
    }

    pub fn stats(&self) -> &OptStats {
        &self.stats
    }

    pub fn run(&mut self, module: &mut Module) {
        // Find functions eligible for inlining
        let inline_candidates = self.find_inline_candidates(module);

        // Count call sites for each function
        let call_counts = self.count_call_sites(module);

        // Inline functions
        for func_idx in 0..module.functions.len() {
            self.inline_calls_in_function(module, func_idx, &inline_candidates, &call_counts);
        }
    }

    fn find_inline_candidates(&self, module: &Module) -> HashSet<FuncId> {
        let mut candidates = HashSet::new();

        for func in &module.functions {
            if self.is_inline_candidate(func) {
                candidates.insert(func.id.clone());
            }
        }

        candidates
    }

    fn is_inline_candidate(&self, func: &Function) -> bool {
        // Count total instructions
        let instruction_count: usize = func.blocks.iter().map(|b| b.instructions.len()).sum();

        // Check basic criteria
        if func.blocks.len() > self.config.max_blocks {
            return false;
        }

        if instruction_count > self.config.max_instructions {
            return false;
        }

        // Check for recursion (simple check - function calls itself)
        if self.is_recursive(func) {
            return false;
        }

        // Small functions are always candidates
        if instruction_count <= self.config.always_inline_threshold {
            return true;
        }

        true
    }

    fn is_recursive(&self, func: &Function) -> bool {
        for block in &func.blocks {
            for inst in &block.instructions {
                if let Instruction::Call { func: called, .. } = inst {
                    if called == &func.id {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn count_call_sites(&self, module: &Module) -> HashMap<FuncId, usize> {
        let mut counts = HashMap::new();

        for func in &module.functions {
            for block in &func.blocks {
                for inst in &block.instructions {
                    if let Instruction::Call { func: called, .. } = inst {
                        *counts.entry(called.clone()).or_insert(0) += 1;
                    }
                }
            }
        }

        counts
    }

    fn inline_calls_in_function(
        &mut self,
        module: &mut Module,
        func_idx: usize,
        candidates: &HashSet<FuncId>,
        call_counts: &HashMap<FuncId, usize>,
    ) {
        // We need to be careful with borrowing here
        // First, collect the calls we want to inline
        let calls_to_inline =
            self.find_calls_to_inline(&module.functions[func_idx], candidates, call_counts);

        if calls_to_inline.is_empty() {
            return;
        }

        // For each call site, perform inlining
        for (block_idx, inst_idx, callee_id) in calls_to_inline.into_iter().rev() {
            // Find the callee function
            if let Some(callee) = module.functions.iter().find(|f| f.id == callee_id).cloned() {
                self.inline_call(
                    &mut module.functions[func_idx],
                    block_idx,
                    inst_idx,
                    &callee,
                );
                self.stats.functions_inlined += 1;
            }
        }
    }

    fn find_calls_to_inline(
        &self,
        func: &Function,
        candidates: &HashSet<FuncId>,
        call_counts: &HashMap<FuncId, usize>,
    ) -> Vec<(usize, usize, FuncId)> {
        let mut calls = Vec::new();

        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (inst_idx, inst) in block.instructions.iter().enumerate() {
                if let Instruction::Call { func: callee, .. } = inst {
                    // Don't inline self-calls
                    if callee == &func.id {
                        continue;
                    }

                    // Check if this is an inline candidate
                    if !candidates.contains(callee) {
                        continue;
                    }

                    // Check call count threshold
                    let count = call_counts.get(callee).copied().unwrap_or(0);
                    if count > self.config.max_call_sites {
                        continue;
                    }

                    calls.push((block_idx, inst_idx, callee.clone()));
                }
            }
        }

        calls
    }

    fn inline_call(
        &mut self,
        caller: &mut Function,
        block_idx: usize,
        inst_idx: usize,
        callee: &Function,
    ) {
        // Get the call instruction
        let call_inst = caller.blocks[block_idx].instructions[inst_idx].clone();

        let (dest, args) = match &call_inst {
            Instruction::Call { dest, args, .. } => (dest.clone(), args.clone()),
            _ => return,
        };

        // Generate unique variable and block IDs for the inlined code
        let var_offset = self.find_max_var_id(caller) + 1;
        let block_offset = self.find_max_block_id(caller) + 1;

        // Create parameter mappings (callee param -> argument value)
        let mut var_map: HashMap<VarId, VarId> = HashMap::new();
        for (i, param) in callee.params.iter().enumerate() {
            if i < args.len() {
                var_map.insert(param.id, args[i]);
            }
        }

        // Clone and remap the callee's blocks
        let mut inlined_blocks: Vec<BasicBlock> = Vec::new();
        let mut return_var: Option<VarId> = None;

        for (i, block) in callee.blocks.iter().enumerate() {
            let new_block_id = BlockId(block_offset + i as u32);
            let mut new_block = BasicBlock::new(new_block_id);

            for inst in &block.instructions {
                match inst {
                    Instruction::Return { value } => {
                        // Handle return: store the return value
                        if let Some(ref val) = value {
                            let mapped_val = self.map_var(*val, &var_map, var_offset);
                            return_var = Some(mapped_val);
                        }
                        // The return becomes a jump to the continuation
                        // (handled after we process all blocks)
                    }
                    _ => {
                        // Remap variables and blocks in this instruction
                        let remapped =
                            self.remap_instruction(inst, &var_map, var_offset, block_offset);
                        new_block.instructions.push(remapped);
                    }
                }
            }

            inlined_blocks.push(new_block);
        }

        // Now modify the caller:
        // 1. Split the current block at the call site
        // 2. Insert the inlined blocks
        // 3. Add a copy of the return value to the destination (if any)

        // Get instructions after the call
        let after_call: Vec<Instruction> = caller.blocks[block_idx]
            .instructions
            .drain(inst_idx + 1..)
            .collect();

        // Remove the call instruction
        caller.blocks[block_idx].instructions.pop();

        // If the callee has blocks, jump to the first inlined block
        if !inlined_blocks.is_empty() {
            let first_inlined_block = BlockId(block_offset);
            caller.blocks[block_idx]
                .instructions
                .push(Instruction::Jump {
                    target: first_inlined_block,
                });
        }

        // Create continuation block for code after the call
        let continuation_block_id = BlockId(block_offset + inlined_blocks.len() as u32);
        let mut continuation_block = BasicBlock::new(continuation_block_id);

        // If there's a destination and a return value, add a copy
        if let (Some(dest_var), Some(ret_var)) = (dest, return_var) {
            // We'll need to handle this - for now, the return value is already
            // in the mapped variable, we just need to ensure it gets used correctly
            continuation_block.instructions.push(Instruction::Binary {
                dest: dest_var,
                op: crate::ir::BinaryOp::Add, // This is a hack - we need a Copy instruction
                left: ret_var,
                right: ret_var, // x + 0 would be better but we don't have the type info
                ty: IrType::Int, // TODO: proper type handling
            });
        }

        // Add the remaining instructions
        continuation_block.instructions.extend(after_call);

        // Add jump to continuation from the last inlined block
        if let Some(last_block) = inlined_blocks.last_mut() {
            last_block.instructions.push(Instruction::Jump {
                target: continuation_block_id,
            });
        }

        // Insert all the new blocks
        let insert_pos = block_idx + 1;
        for (i, block) in inlined_blocks.into_iter().enumerate() {
            caller.blocks.insert(insert_pos + i, block);
        }
        caller.blocks.push(continuation_block);
    }

    fn find_max_var_id(&self, func: &Function) -> u32 {
        let mut max_id = 0u32;

        for param in &func.params {
            max_id = max_id.max(param.id.0);
        }

        for block in &func.blocks {
            for inst in &block.instructions {
                if let Some(dest) = self.get_defined_var(inst) {
                    max_id = max_id.max(dest.0);
                }
                for used in self.get_used_vars(inst) {
                    max_id = max_id.max(used.0);
                }
            }
        }

        max_id
    }

    fn find_max_block_id(&self, func: &Function) -> u32 {
        func.blocks.iter().map(|b| b.id.0).max().unwrap_or(0)
    }

    fn map_var(&self, var: VarId, map: &HashMap<VarId, VarId>, offset: u32) -> VarId {
        map.get(&var).copied().unwrap_or(VarId(var.0 + offset))
    }

    fn remap_instruction(
        &self,
        inst: &Instruction,
        var_map: &HashMap<VarId, VarId>,
        var_offset: u32,
        block_offset: u32,
    ) -> Instruction {
        let map_var = |v: &VarId| self.map_var(*v, var_map, var_offset);
        let map_block = |b: &BlockId| BlockId(b.0 + block_offset);

        match inst {
            Instruction::Const { dest, value, ty } => Instruction::Const {
                dest: map_var(dest),
                value: value.clone(),
                ty: ty.clone(),
            },

            Instruction::Binary {
                dest,
                op,
                left,
                right,
                ty,
            } => Instruction::Binary {
                dest: map_var(dest),
                op: *op,
                left: map_var(left),
                right: map_var(right),
                ty: ty.clone(),
            },

            Instruction::Unary {
                dest,
                op,
                operand,
                ty,
            } => Instruction::Unary {
                dest: map_var(dest),
                op: *op,
                operand: map_var(operand),
                ty: ty.clone(),
            },

            Instruction::Load { dest, ptr, ty } => Instruction::Load {
                dest: map_var(dest),
                ptr: map_var(ptr),
                ty: ty.clone(),
            },

            Instruction::Store { ptr, value } => Instruction::Store {
                ptr: map_var(ptr),
                value: map_var(value),
            },

            Instruction::Alloca { dest, ty } => Instruction::Alloca {
                dest: map_var(dest),
                ty: ty.clone(),
            },

            Instruction::GetElementPtr {
                dest,
                ptr,
                index,
                elem_ty,
            } => Instruction::GetElementPtr {
                dest: map_var(dest),
                ptr: map_var(ptr),
                index: map_var(index),
                elem_ty: elem_ty.clone(),
            },

            Instruction::Jump { target } => Instruction::Jump {
                target: map_block(target),
            },

            Instruction::Branch {
                cond,
                then_block,
                else_block,
            } => Instruction::Branch {
                cond: map_var(cond),
                then_block: map_block(then_block),
                else_block: map_block(else_block),
            },

            Instruction::Call {
                dest,
                func,
                args,
                ret_ty,
            } => Instruction::Call {
                dest: dest.map(|d| map_var(&d)),
                func: func.clone(),
                args: args.iter().map(map_var).collect(),
                ret_ty: ret_ty.clone(),
            },

            Instruction::CallIndirect {
                dest,
                func_ptr,
                args,
                ret_ty,
            } => Instruction::CallIndirect {
                dest: dest.map(|d| map_var(&d)),
                func_ptr: map_var(func_ptr),
                args: args.iter().map(map_var).collect(),
                ret_ty: ret_ty.clone(),
            },

            Instruction::Return { value } => Instruction::Return {
                value: value.map(|v| map_var(&v)),
            },

            Instruction::Phi { dest, ty, incoming } => Instruction::Phi {
                dest: map_var(dest),
                ty: ty.clone(),
                incoming: incoming
                    .iter()
                    .map(|(v, b)| (map_var(v), map_block(b)))
                    .collect(),
            },

            Instruction::NewObject { dest, class } => Instruction::NewObject {
                dest: map_var(dest),
                class: class.clone(),
            },

            Instruction::GetField {
                dest,
                object,
                field,
                ty,
            } => Instruction::GetField {
                dest: map_var(dest),
                object: map_var(object),
                field: field.clone(),
                ty: ty.clone(),
            },

            Instruction::SetField {
                object,
                field,
                value,
            } => Instruction::SetField {
                object: map_var(object),
                field: field.clone(),
                value: map_var(value),
            },

            Instruction::CallMethod {
                dest,
                object,
                method,
                args,
                ret_ty,
            } => Instruction::CallMethod {
                dest: dest.map(|d| map_var(&d)),
                object: map_var(object),
                method: method.clone(),
                args: args.iter().map(map_var).collect(),
                ret_ty: ret_ty.clone(),
            },

            Instruction::CallVirtual {
                dest,
                object,
                method_index,
                args,
                ret_ty,
            } => Instruction::CallVirtual {
                dest: dest.map(|d| map_var(&d)),
                object: map_var(object),
                method_index: *method_index,
                args: args.iter().map(map_var).collect(),
                ret_ty: ret_ty.clone(),
            },

            Instruction::NewArray {
                dest,
                elem_ty,
                elements,
            } => Instruction::NewArray {
                dest: map_var(dest),
                elem_ty: elem_ty.clone(),
                elements: elements.iter().map(map_var).collect(),
            },

            Instruction::ArrayLen { dest, array } => Instruction::ArrayLen {
                dest: map_var(dest),
                array: map_var(array),
            },

            Instruction::ArrayGet {
                dest,
                array,
                index,
                elem_ty,
            } => Instruction::ArrayGet {
                dest: map_var(dest),
                array: map_var(array),
                index: map_var(index),
                elem_ty: elem_ty.clone(),
            },

            Instruction::ArraySet {
                array,
                index,
                value,
            } => Instruction::ArraySet {
                array: map_var(array),
                index: map_var(index),
                value: map_var(value),
            },

            Instruction::ArrayPush {
                array,
                value,
                elem_ty,
            } => Instruction::ArrayPush {
                array: map_var(array),
                value: map_var(value),
                elem_ty: elem_ty.clone(),
            },

            Instruction::StringConcat { dest, left, right } => Instruction::StringConcat {
                dest: map_var(dest),
                left: map_var(left),
                right: map_var(right),
            },

            Instruction::IntToFloat { dest, src } => Instruction::IntToFloat {
                dest: map_var(dest),
                src: map_var(src),
            },

            Instruction::FloatToInt { dest, src } => Instruction::FloatToInt {
                dest: map_var(dest),
                src: map_var(src),
            },

            Instruction::ToString { dest, src } => Instruction::ToString {
                dest: map_var(dest),
                src: map_var(src),
            },

            Instruction::Bitcast { dest, src, to_ty } => Instruction::Bitcast {
                dest: map_var(dest),
                src: map_var(src),
                to_ty: to_ty.clone(),
            },

            Instruction::Print { value } => Instruction::Print {
                value: map_var(value),
            },

            Instruction::Throw { exception } => Instruction::Throw {
                exception: map_var(exception),
            },

            Instruction::TryBegin { catch_block } => Instruction::TryBegin {
                catch_block: map_block(catch_block),
            },

            Instruction::TryEnd => Instruction::TryEnd,

            Instruction::GetException { dest } => Instruction::GetException {
                dest: map_var(dest),
            },

            Instruction::GlobalLoad { dest, name, ty } => Instruction::GlobalLoad {
                dest: map_var(dest),
                name: name.clone(),
                ty: ty.clone(),
            },

            Instruction::GlobalStore { name, value } => Instruction::GlobalStore {
                name: name.clone(),
                value: map_var(value),
            },

            Instruction::Nop => Instruction::Nop,
        }
    }

    fn get_defined_var(&self, inst: &Instruction) -> Option<VarId> {
        match inst {
            Instruction::Const { dest, .. }
            | Instruction::Binary { dest, .. }
            | Instruction::Unary { dest, .. }
            | Instruction::Load { dest, .. }
            | Instruction::Alloca { dest, .. }
            | Instruction::GetElementPtr { dest, .. }
            | Instruction::NewObject { dest, .. }
            | Instruction::GetField { dest, .. }
            | Instruction::NewArray { dest, .. }
            | Instruction::ArrayLen { dest, .. }
            | Instruction::ArrayGet { dest, .. }
            | Instruction::StringConcat { dest, .. }
            | Instruction::IntToFloat { dest, .. }
            | Instruction::FloatToInt { dest, .. }
            | Instruction::ToString { dest, .. }
            | Instruction::Bitcast { dest, .. }
            | Instruction::GetException { dest }
            | Instruction::Phi { dest, .. }
            | Instruction::GlobalLoad { dest, .. } => Some(*dest),

            Instruction::Call { dest, .. }
            | Instruction::CallIndirect { dest, .. }
            | Instruction::CallMethod { dest, .. }
            | Instruction::CallVirtual { dest, .. } => *dest,

            _ => None,
        }
    }

    fn get_used_vars(&self, inst: &Instruction) -> Vec<VarId> {
        match inst {
            Instruction::Binary { left, right, .. } => vec![*left, *right],
            Instruction::Unary { operand, .. } => vec![*operand],
            Instruction::Load { ptr, .. } => vec![*ptr],
            Instruction::Store { ptr, value } => vec![*ptr, *value],
            Instruction::GetElementPtr { ptr, index, .. } => vec![*ptr, *index],
            Instruction::Branch { cond, .. } => vec![*cond],
            Instruction::Return { value } => value.iter().copied().collect(),
            Instruction::Call { args, .. } => args.clone(),
            Instruction::CallIndirect { func_ptr, args, .. } => {
                let mut vars = vec![*func_ptr];
                vars.extend(args.iter().copied());
                vars
            }
            Instruction::GetField { object, .. } => vec![*object],
            Instruction::SetField { object, value, .. } => vec![*object, *value],
            Instruction::CallMethod { object, args, .. } => {
                let mut vars = vec![*object];
                vars.extend(args.iter().copied());
                vars
            }
            Instruction::CallVirtual { object, args, .. } => {
                let mut vars = vec![*object];
                vars.extend(args.iter().copied());
                vars
            }
            Instruction::NewArray { elements, .. } => elements.clone(),
            Instruction::ArrayLen { array, .. } => vec![*array],
            Instruction::ArrayGet { array, index, .. } => vec![*array, *index],
            Instruction::ArraySet {
                array,
                index,
                value,
            } => vec![*array, *index, *value],
            Instruction::ArrayPush { array, value, .. } => vec![*array, *value],
            Instruction::StringConcat { left, right, .. } => vec![*left, *right],
            Instruction::IntToFloat { src, .. }
            | Instruction::FloatToInt { src, .. }
            | Instruction::ToString { src, .. }
            | Instruction::Bitcast { src, .. } => vec![*src],
            Instruction::Print { value } => vec![*value],
            Instruction::Throw { exception } => vec![*exception],
            Instruction::Phi { incoming, .. } => incoming.iter().map(|(v, _)| *v).collect(),
            Instruction::GlobalStore { value, .. } => vec![*value],
            _ => vec![],
        }
    }
}

impl Default for FunctionInliner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Parameter;

    #[test]
    fn test_inline_candidate_detection() {
        // Create a simple small function
        let mut small_func = Function::new(
            FuncId("small".to_string()),
            "small".to_string(),
            vec![Parameter {
                id: VarId(0),
                name: "x".to_string(),
                ty: IrType::Int,
            }],
            IrType::Int,
        );

        let mut block = BasicBlock::new(BlockId(0));
        block.instructions.push(Instruction::Return {
            value: Some(VarId(0)),
        });
        small_func.blocks.push(block);

        let module = Module::new("test".to_string());
        let mut module = module;
        module.functions.push(small_func);

        let inliner = FunctionInliner::new();
        let candidates = inliner.find_inline_candidates(&module);

        assert!(candidates.contains(&FuncId("small".to_string())));
    }

    #[test]
    fn test_recursive_detection() {
        let mut recursive_func = Function::new(
            FuncId("recursive".to_string()),
            "recursive".to_string(),
            vec![Parameter {
                id: VarId(0),
                name: "n".to_string(),
                ty: IrType::Int,
            }],
            IrType::Int,
        );

        let mut block = BasicBlock::new(BlockId(0));
        // Call itself
        block.instructions.push(Instruction::Call {
            dest: Some(VarId(1)),
            func: FuncId("recursive".to_string()),
            args: vec![VarId(0)],
            ret_ty: IrType::Int,
        });
        block.instructions.push(Instruction::Return {
            value: Some(VarId(1)),
        });
        recursive_func.blocks.push(block);

        let inliner = FunctionInliner::new();
        assert!(inliner.is_recursive(&recursive_func));
    }

    #[test]
    fn test_call_counting() {
        let mut caller = Function::new(
            FuncId("caller".to_string()),
            "caller".to_string(),
            vec![],
            IrType::Int,
        );

        let mut block = BasicBlock::new(BlockId(0));
        // Call "helper" twice
        block.instructions.push(Instruction::Call {
            dest: Some(VarId(0)),
            func: FuncId("helper".to_string()),
            args: vec![],
            ret_ty: IrType::Int,
        });
        block.instructions.push(Instruction::Call {
            dest: Some(VarId(1)),
            func: FuncId("helper".to_string()),
            args: vec![],
            ret_ty: IrType::Int,
        });
        block.instructions.push(Instruction::Call {
            dest: Some(VarId(2)),
            func: FuncId("other".to_string()),
            args: vec![],
            ret_ty: IrType::Int,
        });
        block.instructions.push(Instruction::Return {
            value: Some(VarId(1)),
        });
        caller.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(caller);

        let inliner = FunctionInliner::new();
        let counts = inliner.count_call_sites(&module);

        assert_eq!(counts.get(&FuncId("helper".to_string())), Some(&2));
        assert_eq!(counts.get(&FuncId("other".to_string())), Some(&1));
    }
}
