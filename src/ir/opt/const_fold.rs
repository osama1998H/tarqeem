//! Constant Folding Optimization
//!
//! This pass evaluates constant expressions at compile time, replacing
//! them with their computed values.
//!
//! ## Examples
//!
//! ```text
//! // Before:
//! %1 = const 2
//! %2 = const 3
//! %3 = add %1, %2
//!
//! // After:
//! %3 = const 5
//! ```

use super::OptStats;
use crate::ir::{BasicBlock, BinaryOp, Constant, Function, Instruction, Module, VarId};
use std::collections::HashMap;

/// Constant folding optimization pass
pub struct ConstantFolder {
    stats: OptStats,
}

impl ConstantFolder {
    /// Create a new constant folder
    pub fn new() -> Self {
        Self {
            stats: OptStats::new(),
        }
    }

    /// Get optimization statistics
    pub fn stats(&self) -> &OptStats {
        &self.stats
    }

    /// Run constant folding on a module
    pub fn run(&mut self, module: &mut Module) {
        for function in &mut module.functions {
            self.fold_function(function);
        }
    }

    /// Fold constants in a function
    fn fold_function(&mut self, function: &mut Function) {
        // Map from VarId to known constant value
        let mut constants: HashMap<VarId, Constant> = HashMap::new();

        for block in &mut function.blocks {
            self.fold_block(block, &mut constants);
        }
    }

    /// Fold constants in a basic block
    fn fold_block(&mut self, block: &mut BasicBlock, constants: &mut HashMap<VarId, Constant>) {
        let mut new_instructions = Vec::new();

        for inst in &block.instructions {
            match inst {
                // Track constant definitions
                Instruction::Const { dest, value, .. } => {
                    constants.insert(*dest, value.clone());
                    new_instructions.push(inst.clone());
                }

                // Try to fold binary operations
                Instruction::Binary {
                    dest,
                    op,
                    left,
                    right,
                    ty,
                } => {
                    if let (Some(left_const), Some(right_const)) =
                        (constants.get(left), constants.get(right))
                    {
                        if let Some(result) = self.fold_binary(*op, left_const, right_const) {
                            // Record the result as a constant
                            constants.insert(*dest, result.clone());
                            // Replace with const instruction
                            new_instructions.push(Instruction::Const {
                                dest: *dest,
                                value: result,
                                ty: ty.clone(),
                            });
                            self.stats.constants_folded += 1;
                            continue;
                        }
                    }
                    new_instructions.push(inst.clone());
                }

                // Try to fold unary operations
                Instruction::Unary {
                    dest,
                    op,
                    operand,
                    ty,
                } => {
                    if let Some(operand_const) = constants.get(operand) {
                        if let Some(result) = self.fold_unary(*op, operand_const) {
                            constants.insert(*dest, result.clone());
                            new_instructions.push(Instruction::Const {
                                dest: *dest,
                                value: result,
                                ty: ty.clone(),
                            });
                            self.stats.constants_folded += 1;
                            continue;
                        }
                    }
                    new_instructions.push(inst.clone());
                }

                // Fold conditional branches with constant condition
                Instruction::Branch {
                    cond,
                    then_block,
                    else_block,
                } => {
                    if let Some(Constant::Bool(b)) = constants.get(cond) {
                        // Replace with unconditional jump
                        let target = if *b { *then_block } else { *else_block };
                        new_instructions.push(Instruction::Jump { target });
                        self.stats.constants_folded += 1;
                        continue;
                    }
                    new_instructions.push(inst.clone());
                }

                // Fold string concatenation of constant strings
                Instruction::StringConcat {
                    dest: _,
                    left,
                    right,
                } => {
                    if let (Some(Constant::String(left_idx)), Some(Constant::String(right_idx))) =
                        (constants.get(left), constants.get(right))
                    {
                        // For now, we can't easily concatenate strings since they're in a string table
                        // This would require access to the module's string table
                        // TODO: Implement string constant folding with string table access
                        let _ = (*left_idx, *right_idx); // Silence warnings
                    }
                    new_instructions.push(inst.clone());
                }

                // Keep other instructions as-is
                _ => {
                    new_instructions.push(inst.clone());
                }
            }
        }

        block.instructions = new_instructions;
    }

    /// Try to fold a binary operation on constants
    fn fold_binary(&self, op: BinaryOp, left: &Constant, right: &Constant) -> Option<Constant> {
        match (left, right) {
            // Integer operations
            (Constant::Int(l), Constant::Int(r)) => match op {
                BinaryOp::Add => Some(Constant::Int(l.wrapping_add(*r))),
                BinaryOp::Sub => Some(Constant::Int(l.wrapping_sub(*r))),
                BinaryOp::Mul => Some(Constant::Int(l.wrapping_mul(*r))),
                BinaryOp::Div if *r != 0 => Some(Constant::Int(l / r)),
                BinaryOp::Mod if *r != 0 => Some(Constant::Int(l % r)),
                BinaryOp::Pow if *r >= 0 => Some(Constant::Int(l.wrapping_pow(*r as u32))),
                BinaryOp::Eq => Some(Constant::Bool(l == r)),
                BinaryOp::Ne => Some(Constant::Bool(l != r)),
                BinaryOp::Lt => Some(Constant::Bool(l < r)),
                BinaryOp::Le => Some(Constant::Bool(l <= r)),
                BinaryOp::Gt => Some(Constant::Bool(l > r)),
                BinaryOp::Ge => Some(Constant::Bool(l >= r)),
                BinaryOp::BitAnd => Some(Constant::Int(l & r)),
                BinaryOp::BitOr => Some(Constant::Int(l | r)),
                BinaryOp::BitXor => Some(Constant::Int(l ^ r)),
                BinaryOp::Shl => Some(Constant::Int(l.wrapping_shl(*r as u32))),
                BinaryOp::Shr => Some(Constant::Int(l.wrapping_shr(*r as u32))),
                _ => None,
            },

            // Float operations
            (Constant::Float(l), Constant::Float(r)) => match op {
                BinaryOp::Add => Some(Constant::Float(l + r)),
                BinaryOp::Sub => Some(Constant::Float(l - r)),
                BinaryOp::Mul => Some(Constant::Float(l * r)),
                BinaryOp::Div if *r != 0.0 => Some(Constant::Float(l / r)),
                BinaryOp::Pow => Some(Constant::Float(l.powf(*r))),
                BinaryOp::Eq => Some(Constant::Bool((l - r).abs() < f64::EPSILON)),
                BinaryOp::Ne => Some(Constant::Bool((l - r).abs() >= f64::EPSILON)),
                BinaryOp::Lt => Some(Constant::Bool(l < r)),
                BinaryOp::Le => Some(Constant::Bool(l <= r)),
                BinaryOp::Gt => Some(Constant::Bool(l > r)),
                BinaryOp::Ge => Some(Constant::Bool(l >= r)),
                _ => None,
            },

            // Boolean operations
            (Constant::Bool(l), Constant::Bool(r)) => match op {
                BinaryOp::And => Some(Constant::Bool(*l && *r)),
                BinaryOp::Or => Some(Constant::Bool(*l || *r)),
                BinaryOp::Eq => Some(Constant::Bool(l == r)),
                BinaryOp::Ne => Some(Constant::Bool(l != r)),
                _ => None,
            },

            _ => None,
        }
    }

    /// Try to fold a unary operation on a constant
    fn fold_unary(&self, op: crate::ir::UnaryOp, operand: &Constant) -> Option<Constant> {
        match (op, operand) {
            (crate::ir::UnaryOp::Neg, Constant::Int(i)) => Some(Constant::Int(-i)),
            (crate::ir::UnaryOp::Neg, Constant::Float(f)) => Some(Constant::Float(-f)),
            (crate::ir::UnaryOp::Not, Constant::Bool(b)) => Some(Constant::Bool(!b)),
            (crate::ir::UnaryOp::BitNot, Constant::Int(i)) => Some(Constant::Int(!i)),
            _ => None,
        }
    }
}

impl Default for ConstantFolder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BasicBlock, BlockId, FuncId, Function, IrType, Module};

    fn create_test_function() -> Function {
        let mut func = Function::new(
            FuncId("test".to_string()),
            "test".to_string(),
            vec![],
            IrType::Void,
        );

        let mut block = BasicBlock::new(BlockId(0));

        // %0 = const 2
        block.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::Int(2),
            ty: IrType::Int,
        });

        // %1 = const 3
        block.instructions.push(Instruction::Const {
            dest: VarId(1),
            value: Constant::Int(3),
            ty: IrType::Int,
        });

        // %2 = add %0, %1  (should be folded to const 5)
        block.instructions.push(Instruction::Binary {
            dest: VarId(2),
            op: BinaryOp::Add,
            left: VarId(0),
            right: VarId(1),
            ty: IrType::Int,
        });

        // ret void
        block.instructions.push(Instruction::Return { value: None });

        func.blocks.push(block);
        func
    }

    #[test]
    fn test_constant_folding_add() {
        let mut module = Module::new("test".to_string());
        module.functions.push(create_test_function());

        let mut folder = ConstantFolder::new();
        folder.run(&mut module);

        // Check that the add was folded
        assert!(folder.stats().constants_folded > 0);

        // Check the resulting instruction
        let block = &module.functions[0].blocks[0];
        // Should now have: const 2, const 3, const 5, ret
        let folded = &block.instructions[2];
        match folded {
            Instruction::Const { value, .. } => {
                assert_eq!(*value, Constant::Int(5));
            }
            _ => panic!("Expected Const instruction after folding"),
        }
    }

    #[test]
    fn test_constant_folding_comparison() {
        let mut func = Function::new(
            FuncId("test".to_string()),
            "test".to_string(),
            vec![],
            IrType::Void,
        );

        let mut block = BasicBlock::new(BlockId(0));

        // %0 = const 5
        block.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::Int(5),
            ty: IrType::Int,
        });

        // %1 = const 3
        block.instructions.push(Instruction::Const {
            dest: VarId(1),
            value: Constant::Int(3),
            ty: IrType::Int,
        });

        // %2 = gt %0, %1  (should be folded to const true)
        block.instructions.push(Instruction::Binary {
            dest: VarId(2),
            op: BinaryOp::Gt,
            left: VarId(0),
            right: VarId(1),
            ty: IrType::Bool,
        });

        block.instructions.push(Instruction::Return { value: None });

        func.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut folder = ConstantFolder::new();
        folder.run(&mut module);

        let folded = &module.functions[0].blocks[0].instructions[2];
        match folded {
            Instruction::Const { value, .. } => {
                assert_eq!(*value, Constant::Bool(true));
            }
            _ => panic!("Expected Const instruction after folding"),
        }
    }

    #[test]
    fn test_branch_folding() {
        let mut func = Function::new(
            FuncId("test".to_string()),
            "test".to_string(),
            vec![],
            IrType::Void,
        );

        let mut entry = BasicBlock::new(BlockId(0));
        let then_block = BasicBlock::new(BlockId(1));
        let else_block = BasicBlock::new(BlockId(2));

        // %0 = const true
        entry.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::Bool(true),
            ty: IrType::Bool,
        });

        // branch %0, bb1, bb2 (should be folded to jump bb1)
        entry.instructions.push(Instruction::Branch {
            cond: VarId(0),
            then_block: BlockId(1),
            else_block: BlockId(2),
        });

        func.blocks.push(entry);
        func.blocks.push(then_block);
        func.blocks.push(else_block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut folder = ConstantFolder::new();
        folder.run(&mut module);

        // The branch should be replaced with a jump
        let terminator = module.functions[0].blocks[0].instructions.last().unwrap();
        match terminator {
            Instruction::Jump { target } => {
                assert_eq!(*target, BlockId(1));
            }
            _ => panic!("Expected Jump instruction after folding"),
        }
    }
}
