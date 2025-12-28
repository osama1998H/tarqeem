//! Integration tests for the optimization pipeline.
//!
//! These tests verify that the optimization passes work correctly together
//! and produce the expected results when run as a pipeline.

#[cfg(test)]
mod tests {
    use crate::ir::opt::{OptLevel, OptStats, Optimizer};
    use crate::ir::{
        BasicBlock, BinaryOp, BlockId, Constant, FuncId, Function, Instruction, IrType, Module,
        Parameter, UnaryOp, VarId,
    };

    /// Helper function to create a simple function with a given name
    fn create_function(name: &str) -> Function {
        Function::new(
            FuncId(name.to_string()),
            name.to_string(),
            vec![],
            IrType::Void,
        )
    }

    /// Helper function to add a constant instruction
    fn add_const(block: &mut BasicBlock, dest: u32, value: i64) {
        block.instructions.push(Instruction::Const {
            dest: VarId(dest),
            value: Constant::Int(value),
            ty: IrType::Int,
        });
    }

    /// Helper function to add a binary instruction
    fn add_binary(block: &mut BasicBlock, dest: u32, op: BinaryOp, left: u32, right: u32) {
        block.instructions.push(Instruction::Binary {
            dest: VarId(dest),
            op,
            left: VarId(left),
            right: VarId(right),
            ty: IrType::Int,
        });
    }

    // =========================================================================
    // O0 Level Tests - No optimization
    // =========================================================================

    #[test]
    fn test_o0_no_optimization() {
        let mut func = create_function("test");
        let mut block = BasicBlock::new(BlockId(0));

        // Add: %0 = 2; %1 = 3; %2 = %0 + %1; (should not be folded at O0)
        add_const(&mut block, 0, 2);
        add_const(&mut block, 1, 3);
        add_binary(&mut block, 2, BinaryOp::Add, 0, 1);
        block.instructions.push(Instruction::Return { value: None });

        func.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let original_inst_count = module.functions[0].blocks[0].instructions.len();

        let mut optimizer = Optimizer::new(OptLevel::O0);
        optimizer.optimize(&mut module);

        // No changes at O0
        assert_eq!(
            module.functions[0].blocks[0].instructions.len(),
            original_inst_count
        );
        assert!(!optimizer.stats().any_changes());
    }

    // =========================================================================
    // O1 Level Tests - Basic optimizations
    // =========================================================================

    #[test]
    fn test_o1_constant_folding_basic() {
        let mut func = create_function("test");
        func.return_type = IrType::Int;
        let mut block = BasicBlock::new(BlockId(0));

        // Add: %0 = 2; %1 = 3; %2 = %0 + %1;
        add_const(&mut block, 0, 2);
        add_const(&mut block, 1, 3);
        add_binary(&mut block, 2, BinaryOp::Add, 0, 1);
        // Return the result so DCE doesn't remove it
        block.instructions.push(Instruction::Return {
            value: Some(VarId(2)),
        });

        func.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut optimizer = Optimizer::new(OptLevel::O1);
        optimizer.optimize(&mut module);

        // Constant folding should have happened
        assert!(optimizer.stats().constants_folded > 0);

        // The addition should be folded to constant 5
        // After DCE, intermediate constants may be removed, so search for the result
        let block = &module.functions[0].blocks[0];
        let has_folded_const = block.instructions.iter().any(|inst| {
            matches!(
                inst,
                Instruction::Const {
                    dest: VarId(2),
                    value: Constant::Int(5),
                    ..
                }
            )
        });
        assert!(
            has_folded_const,
            "Expected constant 5 at %2 after folding 2 + 3"
        );
    }

    #[test]
    fn test_o1_dead_code_elimination() {
        let mut func = create_function("test");
        let mut block = BasicBlock::new(BlockId(0));

        // Add: %0 = 42; %1 = 100; (unused); return;
        add_const(&mut block, 0, 42);
        add_const(&mut block, 1, 100); // This is dead code
        block.instructions.push(Instruction::Return { value: None });

        func.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut optimizer = Optimizer::new(OptLevel::O1);
        optimizer.optimize(&mut module);

        // Dead code should be eliminated
        assert!(optimizer.stats().dead_instructions_removed > 0);
    }

    #[test]
    fn test_o1_chain_folding() {
        let mut func = create_function("test");
        let mut block = BasicBlock::new(BlockId(0));

        // Chain of constant operations: 2 + 3 = 5, 5 * 4 = 20
        add_const(&mut block, 0, 2);
        add_const(&mut block, 1, 3);
        add_binary(&mut block, 2, BinaryOp::Add, 0, 1); // %2 = 5
        add_const(&mut block, 3, 4);
        add_binary(&mut block, 4, BinaryOp::Mul, 2, 3); // %4 = 20
        block.instructions.push(Instruction::Return {
            value: Some(VarId(4)),
        });

        func.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut optimizer = Optimizer::new(OptLevel::O1);
        optimizer.optimize(&mut module);

        // Multiple constants should be folded
        assert!(optimizer.stats().constants_folded >= 2);
    }

    #[test]
    fn test_o1_unreachable_block_removal() {
        let mut func = create_function("test");

        // Entry block with unconditional jump
        let mut entry = BasicBlock::new(BlockId(0));
        entry
            .instructions
            .push(Instruction::Jump { target: BlockId(2) });

        // Unreachable block
        let mut unreachable = BasicBlock::new(BlockId(1));
        add_const(&mut unreachable, 0, 42);
        unreachable
            .instructions
            .push(Instruction::Return { value: None });

        // Target block
        let mut target = BasicBlock::new(BlockId(2));
        target
            .instructions
            .push(Instruction::Return { value: None });

        func.blocks.push(entry);
        func.blocks.push(unreachable);
        func.blocks.push(target);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut optimizer = Optimizer::new(OptLevel::O1);
        optimizer.optimize(&mut module);

        assert!(optimizer.stats().dead_blocks_removed > 0);
    }

    // =========================================================================
    // O2 Level Tests - CSE and loop optimizations
    // =========================================================================

    #[test]
    fn test_o2_common_subexpression_elimination() {
        let mut func = create_function("test");
        func.params = vec![
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
        ];

        let mut block = BasicBlock::new(BlockId(0));

        // %2 = %0 + %1
        add_binary(&mut block, 2, BinaryOp::Add, 0, 1);
        // %3 = %0 + %1 (same expression, should be eliminated by CSE)
        add_binary(&mut block, 3, BinaryOp::Add, 0, 1);
        // Use both %2 and %3 so DCE doesn't remove them before CSE runs
        // %4 = %2 + %3 (this uses both values)
        add_binary(&mut block, 4, BinaryOp::Add, 2, 3);

        block.instructions.push(Instruction::Return {
            value: Some(VarId(4)),
        });

        func.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut optimizer = Optimizer::new(OptLevel::O2);
        optimizer.optimize(&mut module);

        // CSE should detect duplicate expressions
        assert!(
            optimizer.stats().cse_hits > 0,
            "CSE should detect that %2 and %3 are the same expression"
        );
    }

    #[test]
    fn test_o2_includes_o1_optimizations() {
        let mut func = create_function("test");
        let mut block = BasicBlock::new(BlockId(0));

        // O1 constant folding should still work at O2
        add_const(&mut block, 0, 10);
        add_const(&mut block, 1, 5);
        add_binary(&mut block, 2, BinaryOp::Sub, 0, 1);
        block.instructions.push(Instruction::Return { value: None });

        func.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut optimizer = Optimizer::new(OptLevel::O2);
        optimizer.optimize(&mut module);

        // Both constant folding and CSE should be available
        assert!(optimizer.stats().constants_folded > 0);
    }

    // =========================================================================
    // O3 Level Tests - Aggressive optimizations including inlining
    // =========================================================================

    #[test]
    fn test_o3_includes_all_lower_levels() {
        let mut func = create_function("test");
        let mut block = BasicBlock::new(BlockId(0));

        // O1/O2 optimizations should still work at O3
        add_const(&mut block, 0, 100);
        add_const(&mut block, 1, 50);
        add_binary(&mut block, 2, BinaryOp::Div, 0, 1);
        block.instructions.push(Instruction::Return { value: None });

        func.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut optimizer = Optimizer::new(OptLevel::O3);
        optimizer.optimize(&mut module);

        assert!(optimizer.stats().constants_folded > 0);
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn test_empty_function() {
        let func = create_function("empty");

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        // Should not crash on empty function
        let mut optimizer = Optimizer::new(OptLevel::O3);
        optimizer.optimize(&mut module);
    }

    #[test]
    fn test_single_instruction() {
        let mut func = create_function("single");
        let mut block = BasicBlock::new(BlockId(0));
        block.instructions.push(Instruction::Return { value: None });
        func.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut optimizer = Optimizer::new(OptLevel::O3);
        optimizer.optimize(&mut module);

        // Should have exactly one instruction (return)
        assert_eq!(module.functions[0].blocks[0].instructions.len(), 1);
    }

    #[test]
    fn test_boolean_constant_folding() {
        let mut func = create_function("test");
        func.return_type = IrType::Bool;
        let mut block = BasicBlock::new(BlockId(0));

        // true && false = false
        block.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::Bool(true),
            ty: IrType::Bool,
        });
        block.instructions.push(Instruction::Const {
            dest: VarId(1),
            value: Constant::Bool(false),
            ty: IrType::Bool,
        });
        block.instructions.push(Instruction::Binary {
            dest: VarId(2),
            op: BinaryOp::And,
            left: VarId(0),
            right: VarId(1),
            ty: IrType::Bool,
        });
        // Return the result so DCE doesn't remove it
        block.instructions.push(Instruction::Return {
            value: Some(VarId(2)),
        });

        func.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut optimizer = Optimizer::new(OptLevel::O1);
        optimizer.optimize(&mut module);

        assert!(optimizer.stats().constants_folded > 0);

        // Should fold to false - search for the result
        let block = &module.functions[0].blocks[0];
        let has_folded_const = block.instructions.iter().any(|inst| {
            matches!(
                inst,
                Instruction::Const {
                    dest: VarId(2),
                    value: Constant::Bool(false),
                    ..
                }
            )
        });
        assert!(
            has_folded_const,
            "Expected Const(false) at %2 after folding true && false"
        );
    }

    #[test]
    fn test_float_constant_folding() {
        let mut func = create_function("test");
        func.return_type = IrType::Float;
        let mut block = BasicBlock::new(BlockId(0));

        // 2.5 * 4.0 = 10.0
        block.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::Float(2.5),
            ty: IrType::Float,
        });
        block.instructions.push(Instruction::Const {
            dest: VarId(1),
            value: Constant::Float(4.0),
            ty: IrType::Float,
        });
        block.instructions.push(Instruction::Binary {
            dest: VarId(2),
            op: BinaryOp::Mul,
            left: VarId(0),
            right: VarId(1),
            ty: IrType::Float,
        });
        // Return the result so DCE doesn't remove it
        block.instructions.push(Instruction::Return {
            value: Some(VarId(2)),
        });

        func.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut optimizer = Optimizer::new(OptLevel::O1);
        optimizer.optimize(&mut module);

        assert!(optimizer.stats().constants_folded > 0);

        // Should fold to 10.0 - search for the result
        let block = &module.functions[0].blocks[0];
        let has_folded_const = block.instructions.iter().any(|inst| match inst {
            Instruction::Const {
                dest: VarId(2),
                value: Constant::Float(f),
                ..
            } => (f - 10.0).abs() < 0.0001,
            _ => false,
        });
        assert!(
            has_folded_const,
            "Expected Const(10.0) at %2 after folding 2.5 * 4.0"
        );
    }

    #[test]
    fn test_unary_constant_folding() {
        let mut func = create_function("test");
        func.return_type = IrType::Int;
        let mut block = BasicBlock::new(BlockId(0));

        // -42
        block.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::Int(42),
            ty: IrType::Int,
        });
        block.instructions.push(Instruction::Unary {
            dest: VarId(1),
            op: UnaryOp::Neg,
            operand: VarId(0),
            ty: IrType::Int,
        });
        // Return the result so DCE doesn't remove it
        block.instructions.push(Instruction::Return {
            value: Some(VarId(1)),
        });

        func.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut optimizer = Optimizer::new(OptLevel::O1);
        optimizer.optimize(&mut module);

        assert!(optimizer.stats().constants_folded > 0);

        // Should fold to -42 - search for the result
        let block = &module.functions[0].blocks[0];
        let has_folded_const = block.instructions.iter().any(|inst| {
            matches!(
                inst,
                Instruction::Const {
                    dest: VarId(1),
                    value: Constant::Int(-42),
                    ..
                }
            )
        });
        assert!(
            has_folded_const,
            "Expected Const(-42) at %1 after folding -42"
        );
    }

    // =========================================================================
    // Stats Tests
    // =========================================================================

    #[test]
    fn test_stats_accumulation() {
        let mut stats1 = OptStats::new();
        stats1.constants_folded = 5;
        stats1.dead_instructions_removed = 3;

        let mut stats2 = OptStats::new();
        stats2.constants_folded = 2;
        stats2.cse_hits = 1;

        stats1.merge(&stats2);

        assert_eq!(stats1.constants_folded, 7);
        assert_eq!(stats1.dead_instructions_removed, 3);
        assert_eq!(stats1.cse_hits, 1);
    }

    #[test]
    fn test_stats_any_changes() {
        let empty_stats = OptStats::new();
        assert!(!empty_stats.any_changes());

        let mut stats = OptStats::new();
        stats.constants_folded = 1;
        assert!(stats.any_changes());
    }

    #[test]
    fn test_multiple_functions() {
        let mut module = Module::new("test".to_string());

        // First function with constant folding opportunity
        let mut func1 = create_function("func1");
        let mut block1 = BasicBlock::new(BlockId(0));
        add_const(&mut block1, 0, 5);
        add_const(&mut block1, 1, 10);
        add_binary(&mut block1, 2, BinaryOp::Add, 0, 1);
        block1
            .instructions
            .push(Instruction::Return { value: None });
        func1.blocks.push(block1);

        // Second function with constant folding opportunity
        let mut func2 = create_function("func2");
        let mut block2 = BasicBlock::new(BlockId(0));
        add_const(&mut block2, 0, 20);
        add_const(&mut block2, 1, 4);
        add_binary(&mut block2, 2, BinaryOp::Mul, 0, 1);
        block2
            .instructions
            .push(Instruction::Return { value: None });
        func2.blocks.push(block2);

        module.functions.push(func1);
        module.functions.push(func2);

        let mut optimizer = Optimizer::new(OptLevel::O1);
        optimizer.optimize(&mut module);

        // Both functions should have been optimized
        assert!(optimizer.stats().constants_folded >= 2);
    }

    #[test]
    fn test_division_by_zero_not_folded() {
        let mut func = create_function("test");
        func.return_type = IrType::Int;
        let mut block = BasicBlock::new(BlockId(0));

        // 10 / 0 should NOT be folded (runtime error)
        add_const(&mut block, 0, 10);
        add_const(&mut block, 1, 0);
        add_binary(&mut block, 2, BinaryOp::Div, 0, 1);
        // Return the result so DCE doesn't remove it
        block.instructions.push(Instruction::Return {
            value: Some(VarId(2)),
        });

        func.blocks.push(block);

        let mut module = Module::new("test".to_string());
        module.functions.push(func);

        let mut optimizer = Optimizer::new(OptLevel::O1);
        optimizer.optimize(&mut module);

        // Division by zero should NOT be folded - search for Binary instruction
        let block = &module.functions[0].blocks[0];
        let has_div_instruction = block.instructions.iter().any(|inst| {
            matches!(
                inst,
                Instruction::Binary {
                    op: BinaryOp::Div,
                    ..
                }
            )
        });
        assert!(
            has_div_instruction,
            "Division by zero should not be folded, expected Binary Div instruction"
        );
    }
}
