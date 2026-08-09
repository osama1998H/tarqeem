//! Common Subexpression Elimination (CSE)
//!
//! This pass identifies repeated computations and replaces them with
//! references to previously computed values.
//!
//! ## Examples
//!
//! ```text
//! // Before:
//! %1 = add %a, %b
//! %2 = mul %1, %c
//! %3 = add %a, %b   // same as %1
//! %4 = mul %3, %d
//!
//! // After:
//! %1 = add %a, %b
//! %2 = mul %1, %c
//! %4 = mul %1, %d   // reuses %1 instead of computing again
//! ```

use super::OptStats;
use crate::ir::{BasicBlock, BinaryOp, Function, Instruction, Module, UnaryOp, VarId};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ExprKey {
    Binary {
        op: BinaryOp,
        left: VarId,
        right: VarId,
    },
    Unary {
        op: UnaryOp,
        operand: VarId,
    },
}

pub struct CommonSubexprElim {
    stats: OptStats,
}

impl CommonSubexprElim {
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
            self.eliminate_common_subexpressions(function);
        }
    }

    fn eliminate_common_subexpressions(&mut self, function: &mut Function) {
        for block in &mut function.blocks {
            self.cse_block(block);
        }
    }

    fn cse_block(&mut self, block: &mut BasicBlock) {
        let mut available: HashMap<ExprKey, VarId> = HashMap::new();

        let mut replacements: HashMap<VarId, VarId> = HashMap::new();

        let mut new_instructions = Vec::new();

        for inst in &block.instructions {
            let inst = self.apply_replacements(inst, &replacements);

            match &inst {
                Instruction::Binary {
                    dest,
                    op,
                    left,
                    right,
                    ty: _,
                } => {
                    let key = ExprKey::Binary {
                        op: *op,
                        left: *left,
                        right: *right,
                    };

                    if let Some(&existing) = available.get(&key) {
                        replacements.insert(*dest, existing);
                        self.stats.cse_hits += 1;
                        continue;
                    }

                    if self.is_commutative(*op) {
                        let commuted_key = ExprKey::Binary {
                            op: *op,
                            left: *right,
                            right: *left,
                        };
                        if let Some(&existing) = available.get(&commuted_key) {
                            replacements.insert(*dest, existing);
                            self.stats.cse_hits += 1;
                            continue;
                        }
                    }

                    available.insert(key, *dest);
                    new_instructions.push(inst);
                }

                Instruction::Unary {
                    dest, op, operand, ..
                } => {
                    let key = ExprKey::Unary {
                        op: *op,
                        operand: *operand,
                    };

                    if let Some(&existing) = available.get(&key) {
                        replacements.insert(*dest, existing);
                        self.stats.cse_hits += 1;
                        continue;
                    }

                    available.insert(key, *dest);
                    new_instructions.push(inst);
                }

                _ => {
                    new_instructions.push(inst);
                }
            }
        }

        block.instructions = new_instructions;
    }

    fn is_commutative(&self, op: BinaryOp) -> bool {
        matches!(
            op,
            BinaryOp::Add
                | BinaryOp::Mul
                | BinaryOp::Eq
                | BinaryOp::Ne
                | BinaryOp::And
                | BinaryOp::Or
                | BinaryOp::BitAnd
                | BinaryOp::BitOr
                | BinaryOp::BitXor
        )
    }

    fn apply_replacements(
        &self,
        inst: &Instruction,
        replacements: &HashMap<VarId, VarId>,
    ) -> Instruction {
        let replace = |var: &VarId| replacements.get(var).copied().unwrap_or(*var);

        match inst {
            Instruction::Binary {
                dest,
                op,
                left,
                right,
                ty,
            } => Instruction::Binary {
                dest: *dest,
                op: *op,
                left: replace(left),
                right: replace(right),
                ty: ty.clone(),
            },

            Instruction::Unary {
                dest,
                op,
                operand,
                ty,
            } => Instruction::Unary {
                dest: *dest,
                op: *op,
                operand: replace(operand),
                ty: ty.clone(),
            },

            Instruction::Load { dest, ptr, ty } => Instruction::Load {
                dest: *dest,
                ptr: replace(ptr),
                ty: ty.clone(),
            },

            Instruction::Store { ptr, value } => Instruction::Store {
                ptr: replace(ptr),
                value: replace(value),
            },

            Instruction::GetElementPtr {
                dest,
                ptr,
                index,
                elem_ty,
            } => Instruction::GetElementPtr {
                dest: *dest,
                ptr: replace(ptr),
                index: replace(index),
                elem_ty: elem_ty.clone(),
            },

            Instruction::Branch {
                cond,
                then_block,
                else_block,
            } => Instruction::Branch {
                cond: replace(cond),
                then_block: *then_block,
                else_block: *else_block,
            },

            Instruction::Return { value } => Instruction::Return {
                value: value.map(|v| replace(&v)),
            },

            Instruction::Call {
                dest,
                func,
                args,
                ret_ty,
            } => Instruction::Call {
                dest: *dest,
                func: func.clone(),
                args: args.iter().map(replace).collect(),
                ret_ty: ret_ty.clone(),
            },

            Instruction::CallIndirect {
                dest,
                func_ptr,
                args,
                ret_ty,
            } => Instruction::CallIndirect {
                dest: *dest,
                func_ptr: replace(func_ptr),
                args: args.iter().map(replace).collect(),
                ret_ty: ret_ty.clone(),
            },

            Instruction::GetField {
                dest,
                object,
                field,
                ty,
            } => Instruction::GetField {
                dest: *dest,
                object: replace(object),
                field: field.clone(),
                ty: ty.clone(),
            },

            Instruction::SetField {
                object,
                field,
                value,
            } => Instruction::SetField {
                object: replace(object),
                field: field.clone(),
                value: replace(value),
            },

            Instruction::CallMethod {
                dest,
                object,
                method,
                args,
                ret_ty,
                virtual_dispatch,
            } => Instruction::CallMethod {
                dest: *dest,
                object: replace(object),
                method: method.clone(),
                args: args.iter().map(replace).collect(),
                ret_ty: ret_ty.clone(),
                virtual_dispatch: *virtual_dispatch,
            },

            Instruction::CallVirtual {
                dest,
                object,
                method_index,
                args,
                ret_ty,
            } => Instruction::CallVirtual {
                dest: *dest,
                object: replace(object),
                method_index: *method_index,
                args: args.iter().map(replace).collect(),
                ret_ty: ret_ty.clone(),
            },

            Instruction::NewArray {
                dest,
                elem_ty,
                elements,
            } => Instruction::NewArray {
                dest: *dest,
                elem_ty: elem_ty.clone(),
                elements: elements.iter().map(replace).collect(),
            },

            Instruction::ArrayLen { dest, array } => Instruction::ArrayLen {
                dest: *dest,
                array: replace(array),
            },

            Instruction::ArrayGet {
                dest,
                array,
                index,
                elem_ty,
            } => Instruction::ArrayGet {
                dest: *dest,
                array: replace(array),
                index: replace(index),
                elem_ty: elem_ty.clone(),
            },

            Instruction::ArraySet {
                array,
                index,
                value,
            } => Instruction::ArraySet {
                array: replace(array),
                index: replace(index),
                value: replace(value),
            },

            Instruction::StringConcat { dest, left, right } => Instruction::StringConcat {
                dest: *dest,
                left: replace(left),
                right: replace(right),
            },

            Instruction::Print { value } => Instruction::Print {
                value: replace(value),
            },

            Instruction::Throw { exception } => Instruction::Throw {
                exception: replace(exception),
            },

            Instruction::Phi { dest, ty, incoming } => Instruction::Phi {
                dest: *dest,
                ty: ty.clone(),
                incoming: incoming
                    .iter()
                    .map(|(val, block)| (replace(val), *block))
                    .collect(),
            },

            Instruction::Copy { dest, src, ty } => Instruction::Copy {
                dest: *dest,
                src: replace(src),
                ty: ty.clone(),
            },

            _ => inst.clone(),
        }
    }
}

impl Default for CommonSubexprElim {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BasicBlock, BlockId, Constant, FuncId, IrType, Parameter};

    #[test]
    fn test_cse_binary() {
        let mut func = Function::new(
            FuncId("test".to_string()),
            "test".to_string(),
            vec![
                Parameter {
                    id: VarId(0),
                    name: "a".to_string(),
                    ty: IrType::Int,
                },
                Parameter {
                    id: VarId(1),
                    name: "b".to_string(),
                    ty: IrType::Int,
                },
            ],
            IrType::Int,
        );

        let mut block = BasicBlock::new(BlockId(0));

        block.instructions.push(Instruction::Binary {
            dest: VarId(2),
            op: BinaryOp::Add,
            left: VarId(0),
            right: VarId(1),
            ty: IrType::Int,
        });

        block.instructions.push(Instruction::Const {
            dest: VarId(10),
            value: Constant::Int(2),
            ty: IrType::Int,
        });
        block.instructions.push(Instruction::Binary {
            dest: VarId(3),
            op: BinaryOp::Mul,
            left: VarId(2),
            right: VarId(10),
            ty: IrType::Int,
        });

        block.instructions.push(Instruction::Binary {
            dest: VarId(4),
            op: BinaryOp::Add,
            left: VarId(0),
            right: VarId(1),
            ty: IrType::Int,
        });

        block.instructions.push(Instruction::Const {
            dest: VarId(11),
            value: Constant::Int(3),
            ty: IrType::Int,
        });
        block.instructions.push(Instruction::Binary {
            dest: VarId(5),
            op: BinaryOp::Mul,
            left: VarId(4),
            right: VarId(11),
            ty: IrType::Int,
        });

        block.instructions.push(Instruction::Return {
            value: Some(VarId(5)),
        });

        func.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut cse = CommonSubexprElim::new();
        cse.run(&mut module);

        assert_eq!(cse.stats().cse_hits, 1);

        let block = &module.functions[0].blocks[0];
        assert_eq!(block.instructions.len(), 6);
    }

    #[test]
    fn test_cse_commutative() {
        let mut func = Function::new(
            FuncId("test".to_string()),
            "test".to_string(),
            vec![
                Parameter {
                    id: VarId(0),
                    name: "a".to_string(),
                    ty: IrType::Int,
                },
                Parameter {
                    id: VarId(1),
                    name: "b".to_string(),
                    ty: IrType::Int,
                },
            ],
            IrType::Int,
        );

        let mut block = BasicBlock::new(BlockId(0));

        block.instructions.push(Instruction::Binary {
            dest: VarId(2),
            op: BinaryOp::Add,
            left: VarId(0),
            right: VarId(1),
            ty: IrType::Int,
        });

        block.instructions.push(Instruction::Binary {
            dest: VarId(3),
            op: BinaryOp::Add,
            left: VarId(1),
            right: VarId(0),
            ty: IrType::Int,
        });

        block.instructions.push(Instruction::Return {
            value: Some(VarId(3)),
        });

        func.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut cse = CommonSubexprElim::new();
        cse.run(&mut module);

        assert_eq!(cse.stats().cse_hits, 1);
    }
}
