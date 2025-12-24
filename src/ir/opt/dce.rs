//! Dead Code Elimination (DCE)
//!
//! This pass removes:
//! - Unused variable assignments
//! - Unreachable basic blocks
//! - Unused functions (optional)
//!
//! ## Examples
//!
//! ```text
//! // Before:
//! %0 = const 5
//! %1 = const 10   // unused
//! ret %0
//!
//! // After:
//! %0 = const 5
//! ret %0
//! ```

use super::OptStats;
use crate::ir::{BasicBlock, BlockId, Function, Instruction, Module, VarId};
use std::collections::{HashSet, VecDeque};

pub struct DeadCodeEliminator {
    stats: OptStats,
}

impl DeadCodeEliminator {
    pub fn new() -> Self {
        Self {
            stats: OptStats::new(),
        }
    }

    pub fn stats(&self) -> &OptStats {
        &self.stats
    }

    pub fn run(&mut self, module: &mut Module) {
        for function in &mut module.functions {
            self.eliminate_dead_code(function);
        }
    }

    fn eliminate_dead_code(&mut self, function: &mut Function) {
        self.remove_unreachable_blocks(function);

        self.remove_dead_instructions(function);
    }

    fn remove_unreachable_blocks(&mut self, function: &mut Function) {
        if function.blocks.is_empty() {
            return;
        }

        let reachable = self.find_reachable_blocks(function);

        let original_count = function.blocks.len();

        function
            .blocks
            .retain(|block| reachable.contains(&block.id));

        let removed = original_count - function.blocks.len();
        self.stats.dead_blocks_removed += removed;
    }

    fn find_reachable_blocks(&self, function: &Function) -> HashSet<BlockId> {
        let mut reachable = HashSet::new();
        let mut worklist = VecDeque::new();

        if let Some(entry) = function.blocks.first() {
            worklist.push_back(entry.id);
        }

        while let Some(block_id) = worklist.pop_front() {
            if reachable.contains(&block_id) {
                continue;
            }
            reachable.insert(block_id);

            if let Some(block) = function.get_block(block_id) {
                for successor in self.get_successors(block) {
                    if !reachable.contains(&successor) {
                        worklist.push_back(successor);
                    }
                }
            }
        }

        reachable
    }

    fn get_successors(&self, block: &BasicBlock) -> Vec<BlockId> {
        let mut successors = Vec::new();

        if let Some(terminator) = block.instructions.last() {
            match terminator {
                Instruction::Jump { target } => {
                    successors.push(*target);
                }
                Instruction::Branch {
                    then_block,
                    else_block,
                    ..
                } => {
                    successors.push(*then_block);
                    successors.push(*else_block);
                }
                Instruction::TryBegin { catch_block } => {
                    successors.push(*catch_block);
                }
                _ => {}
            }
        }

        successors
    }

    fn remove_dead_instructions(&mut self, function: &mut Function) {
        let used_vars = self.find_used_variables(function);

        for block in &mut function.blocks {
            let original_len = block.instructions.len();

            block.instructions.retain(|inst| {
                if self.has_side_effects(inst) {
                    return true;
                }

                if let Some(dest) = self.get_defined_var(inst) {
                    used_vars.contains(&dest)
                } else {
                    true
                }
            });

            self.stats.dead_instructions_removed += original_len - block.instructions.len();
        }
    }

    fn find_used_variables(&self, function: &Function) -> HashSet<VarId> {
        let mut used = HashSet::new();

        for block in &function.blocks {
            for inst in &block.instructions {
                self.collect_uses(inst, &mut used);
            }
        }

        used
    }

    fn collect_uses(&self, inst: &Instruction, used: &mut HashSet<VarId>) {
        match inst {
            Instruction::Binary { left, right, .. } => {
                used.insert(*left);
                used.insert(*right);
            }
            Instruction::Unary { operand, .. } => {
                used.insert(*operand);
            }
            Instruction::IntToFloat { src, .. }
            | Instruction::FloatToInt { src, .. }
            | Instruction::ToString { src, .. } => {
                used.insert(*src);
            }
            Instruction::Bitcast { src, .. } => {
                used.insert(*src);
            }
            Instruction::Copy { src, .. } => {
                used.insert(*src);
            }
            Instruction::Load { ptr, .. } => {
                used.insert(*ptr);
            }
            Instruction::Store { ptr, value } => {
                used.insert(*ptr);
                used.insert(*value);
            }
            Instruction::GetElementPtr { ptr, index, .. } => {
                used.insert(*ptr);
                used.insert(*index);
            }
            Instruction::Branch { cond, .. } => {
                used.insert(*cond);
            }
            Instruction::Return { value } => {
                if let Some(v) = value {
                    used.insert(*v);
                }
            }
            Instruction::Call { args, .. } => {
                for arg in args {
                    used.insert(*arg);
                }
            }
            Instruction::CallIndirect { func_ptr, args, .. } => {
                used.insert(*func_ptr);
                for arg in args {
                    used.insert(*arg);
                }
            }
            Instruction::NewArray { elements, .. } => {
                for elem in elements {
                    used.insert(*elem);
                }
            }
            Instruction::ArrayLen { array, .. } => {
                used.insert(*array);
            }
            Instruction::ArrayGet { array, index, .. } => {
                used.insert(*array);
                used.insert(*index);
            }
            Instruction::ArraySet {
                array,
                index,
                value,
            } => {
                used.insert(*array);
                used.insert(*index);
                used.insert(*value);
            }
            Instruction::ArrayPush { array, value, .. } => {
                used.insert(*array);
                used.insert(*value);
            }
            Instruction::StringConcat { left, right, .. } => {
                used.insert(*left);
                used.insert(*right);
            }
            Instruction::GetField { object, .. } => {
                used.insert(*object);
            }
            Instruction::SetField { object, value, .. } => {
                used.insert(*object);
                used.insert(*value);
            }
            Instruction::CallMethod { object, args, .. } => {
                used.insert(*object);
                for arg in args {
                    used.insert(*arg);
                }
            }
            Instruction::CallVirtual { object, args, .. } => {
                used.insert(*object);
                for arg in args {
                    used.insert(*arg);
                }
            }
            Instruction::Throw { exception } => {
                used.insert(*exception);
            }
            Instruction::Print { value } => {
                used.insert(*value);
            }
            Instruction::Phi { incoming, .. } => {
                for (val, _) in incoming {
                    used.insert(*val);
                }
            }
            Instruction::GlobalStore { value, .. } => {
                used.insert(*value);
            }
            Instruction::GlobalLoad { .. }
            | Instruction::NewObject { .. }
            | Instruction::Const { .. }
            | Instruction::Alloca { .. }
            | Instruction::Jump { .. }
            | Instruction::TryBegin { .. }
            | Instruction::TryEnd
            | Instruction::GetException { .. }
            | Instruction::Nop => {}
        }
    }

    fn get_defined_var(&self, inst: &Instruction) -> Option<VarId> {
        match inst {
            Instruction::Const { dest, .. }
            | Instruction::Binary { dest, .. }
            | Instruction::Unary { dest, .. }
            | Instruction::IntToFloat { dest, .. }
            | Instruction::FloatToInt { dest, .. }
            | Instruction::ToString { dest, .. }
            | Instruction::Bitcast { dest, .. }
            | Instruction::Alloca { dest, .. }
            | Instruction::Load { dest, .. }
            | Instruction::GetElementPtr { dest, .. }
            | Instruction::NewObject { dest, .. }
            | Instruction::GetField { dest, .. }
            | Instruction::NewArray { dest, .. }
            | Instruction::ArrayLen { dest, .. }
            | Instruction::ArrayGet { dest, .. }
            | Instruction::StringConcat { dest, .. }
            | Instruction::GetException { dest }
            | Instruction::Phi { dest, .. }
            | Instruction::GlobalLoad { dest, .. }
            | Instruction::Copy { dest, .. } => Some(*dest),

            Instruction::Call { dest, .. }
            | Instruction::CallIndirect { dest, .. }
            | Instruction::CallMethod { dest, .. }
            | Instruction::CallVirtual { dest, .. } => *dest,

            _ => None,
        }
    }

    fn has_side_effects(&self, inst: &Instruction) -> bool {
        matches!(
            inst,
            Instruction::Store { .. }
                | Instruction::GlobalStore { .. }
                | Instruction::SetField { .. }
                | Instruction::ArraySet { .. }
                | Instruction::Call { .. }
                | Instruction::CallIndirect { .. }
                | Instruction::CallMethod { .. }
                | Instruction::CallVirtual { .. }
                | Instruction::Jump { .. }
                | Instruction::Branch { .. }
                | Instruction::Return { .. }
                | Instruction::Throw { .. }
                | Instruction::TryBegin { .. }
                | Instruction::TryEnd
                | Instruction::Print { .. }
        )
    }
}

impl Default for DeadCodeEliminator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Constant, FuncId, IrType};

    #[test]
    fn test_remove_unused_variable() {
        let mut func = Function::new(
            FuncId("test".to_string()),
            "test".to_string(),
            vec![],
            IrType::Void,
        );

        let mut block = BasicBlock::new(BlockId(0));

        block.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::Int(5),
            ty: IrType::Int,
        });

        block.instructions.push(Instruction::Const {
            dest: VarId(1),
            value: Constant::Int(10),
            ty: IrType::Int,
        });

        block.instructions.push(Instruction::Return {
            value: Some(VarId(0)),
        });

        func.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut dce = DeadCodeEliminator::new();
        dce.run(&mut module);

        assert_eq!(dce.stats().dead_instructions_removed, 1);

        assert_eq!(module.functions[0].blocks[0].instructions.len(), 2);
    }

    #[test]
    fn test_remove_unreachable_block() {
        let mut func = Function::new(
            FuncId("test".to_string()),
            "test".to_string(),
            vec![],
            IrType::Void,
        );

        let mut entry = BasicBlock::new(BlockId(0));
        entry
            .instructions
            .push(Instruction::Jump { target: BlockId(2) });

        let mut unreachable = BasicBlock::new(BlockId(1));
        unreachable
            .instructions
            .push(Instruction::Return { value: None });

        let mut reachable = BasicBlock::new(BlockId(2));
        reachable
            .instructions
            .push(Instruction::Return { value: None });

        func.blocks.push(entry);
        func.blocks.push(unreachable);
        func.blocks.push(reachable);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut dce = DeadCodeEliminator::new();
        dce.run(&mut module);

        assert_eq!(dce.stats().dead_blocks_removed, 1);
        assert_eq!(module.functions[0].blocks.len(), 2);
    }

    #[test]
    fn test_keep_side_effects() {
        let mut func = Function::new(
            FuncId("test".to_string()),
            "test".to_string(),
            vec![],
            IrType::Void,
        );

        let mut block = BasicBlock::new(BlockId(0));

        block.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::Int(5),
            ty: IrType::Int,
        });

        block
            .instructions
            .push(Instruction::Print { value: VarId(0) });

        block.instructions.push(Instruction::Return { value: None });

        func.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut dce = DeadCodeEliminator::new();
        dce.run(&mut module);

        assert_eq!(dce.stats().dead_instructions_removed, 0);
        assert_eq!(module.functions[0].blocks[0].instructions.len(), 3);
    }
}
