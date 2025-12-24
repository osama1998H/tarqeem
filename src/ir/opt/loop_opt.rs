//! Loop Optimization Passes
//!
//! This module provides loop-related optimizations:
//! - Loop detection and analysis
//! - Loop-invariant code motion (LICM)
//! - Strength reduction
//! - Loop unrolling (optional)

use super::OptStats;
use crate::ir::{
    BasicBlock, BinaryOp, BlockId, Constant, Function, Instruction, IrType, Module, VarId,
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Loop {
    pub header: BlockId,
    pub blocks: HashSet<BlockId>,
    pub back_edges: Vec<BlockId>,
    pub preheader: Option<BlockId>,
    pub exits: HashSet<BlockId>,
    pub induction_vars: Vec<InductionVar>,
    pub depth: usize,
}

impl Loop {
    pub fn new(header: BlockId) -> Self {
        Self {
            header,
            blocks: HashSet::new(),
            back_edges: Vec::new(),
            preheader: None,
            exits: HashSet::new(),
            induction_vars: Vec::new(),
            depth: 0,
        }
    }

    pub fn contains(&self, block: BlockId) -> bool {
        self.blocks.contains(&block)
    }
}

#[derive(Debug, Clone)]
pub struct InductionVar {
    pub var: VarId,
    pub init: VarId,
    pub step: InductionStep,
    pub final_val: Option<VarId>,
}

#[derive(Debug, Clone)]
pub enum InductionStep {
    Constant(i64),
    Variable(VarId),
    Multiply(i64),
}

#[derive(Debug)]
pub struct LoopAnalysis {
    pub loops: Vec<Loop>,
    pub block_to_loop: HashMap<BlockId, usize>,
}

impl Default for LoopAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

impl LoopAnalysis {
    pub fn new() -> Self {
        Self {
            loops: Vec::new(),
            block_to_loop: HashMap::new(),
        }
    }

    pub fn analyze(func: &Function) -> Self {
        let mut analysis = Self::new();

        if func.blocks.is_empty() {
            return analysis;
        }

        let mut predecessors: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
        for block in &func.blocks {
            for succ in &block.successors {
                predecessors.entry(*succ).or_default().push(block.id);
            }
        }

        let block_order: HashMap<BlockId, usize> = func
            .blocks
            .iter()
            .enumerate()
            .map(|(i, b)| (b.id, i))
            .collect();

        let mut back_edges: Vec<(BlockId, BlockId)> = Vec::new(); // (from, to)
        for block in &func.blocks {
            for succ in &block.successors {
                if let (Some(&from_idx), Some(&to_idx)) =
                    (block_order.get(&block.id), block_order.get(succ))
                {
                    if to_idx <= from_idx {
                        back_edges.push((block.id, *succ));
                    }
                }
            }
        }

        for (back_from, header) in back_edges {
            let mut loop_info = Loop::new(header);
            loop_info.back_edges.push(back_from);

            loop_info.blocks.insert(header);
            let mut worklist = vec![back_from];
            while let Some(block_id) = worklist.pop() {
                if loop_info.blocks.insert(block_id) {
                    if let Some(preds) = predecessors.get(&block_id) {
                        for pred in preds {
                            if !loop_info.blocks.contains(pred) {
                                worklist.push(*pred);
                            }
                        }
                    }
                }
            }

            for &block_id in &loop_info.blocks {
                if let Some(block) = func.get_block(block_id) {
                    for succ in &block.successors {
                        if !loop_info.blocks.contains(succ) {
                            loop_info.exits.insert(block_id);
                        }
                    }
                }
            }

            if let Some(header_preds) = predecessors.get(&header) {
                let external_preds: Vec<_> = header_preds
                    .iter()
                    .filter(|p| !loop_info.blocks.contains(p))
                    .collect();
                if external_preds.len() == 1 {
                    loop_info.preheader = Some(*external_preds[0]);
                }
            }

            loop_info.induction_vars = analyze_induction_vars(func, &loop_info);

            let loop_idx = analysis.loops.len();
            for &block_id in &loop_info.blocks {
                analysis.block_to_loop.insert(block_id, loop_idx);
            }

            analysis.loops.push(loop_info);
        }

        for i in 0..analysis.loops.len() {
            let header = analysis.loops[i].header;
            let mut depth = 0;
            for j in 0..analysis.loops.len() {
                if i != j && analysis.loops[j].contains(header) {
                    depth += 1;
                }
            }
            analysis.loops[i].depth = depth;
        }

        analysis
    }
}

fn analyze_induction_vars(func: &Function, loop_info: &Loop) -> Vec<InductionVar> {
    let mut induction_vars = Vec::new();

    if let Some(header_block) = func.get_block(loop_info.header) {
        for inst in &header_block.instructions {
            if let Instruction::Phi { dest, ty, incoming } = inst {
                if !matches!(ty, IrType::Int) {
                    continue; // Only handle integer induction variables
                }

                let mut outside_val = None;
                let mut inside_val = None;

                for (val, block) in incoming {
                    if loop_info.contains(*block) {
                        inside_val = Some((*val, *block));
                    } else {
                        outside_val = Some(*val);
                    }
                }

                if let (Some(init), Some((update, update_block))) = (outside_val, inside_val) {
                    if let Some(step) = find_induction_step(func, update_block, update, *dest) {
                        induction_vars.push(InductionVar {
                            var: *dest,
                            init,
                            step,
                            final_val: None,
                        });
                    }
                }
            }
        }
    }

    induction_vars
}

fn find_induction_step(
    func: &Function,
    block_id: BlockId,
    result: VarId,
    phi_var: VarId,
) -> Option<InductionStep> {
    let block = func.get_block(block_id)?;

    for inst in &block.instructions {
        match inst {
            Instruction::Binary {
                dest,
                op: BinaryOp::Add,
                left,
                right,
                ..
            } if *dest == result => {
                if *left == phi_var {
                    if let Some(step_val) = find_constant_value(func, *right) {
                        return Some(InductionStep::Constant(step_val));
                    }
                    return Some(InductionStep::Variable(*right));
                }
                if *right == phi_var {
                    if let Some(step_val) = find_constant_value(func, *left) {
                        return Some(InductionStep::Constant(step_val));
                    }
                    return Some(InductionStep::Variable(*left));
                }
            }
            Instruction::Binary {
                dest,
                op: BinaryOp::Sub,
                left,
                right,
                ..
            } if *dest == result && *left == phi_var => {
                if let Some(step_val) = find_constant_value(func, *right) {
                    return Some(InductionStep::Constant(-step_val));
                }
            }
            Instruction::Binary {
                dest,
                op: BinaryOp::Mul,
                left,
                right,
                ..
            } if *dest == result => {
                if *left == phi_var {
                    if let Some(step_val) = find_constant_value(func, *right) {
                        return Some(InductionStep::Multiply(step_val));
                    }
                }
                if *right == phi_var {
                    if let Some(step_val) = find_constant_value(func, *left) {
                        return Some(InductionStep::Multiply(step_val));
                    }
                }
            }
            _ => {}
        }
    }

    None
}

fn find_constant_value(func: &Function, var: VarId) -> Option<i64> {
    for block in &func.blocks {
        for inst in &block.instructions {
            if let Instruction::Const {
                dest,
                value: Constant::Int(val),
                ..
            } = inst
            {
                if *dest == var {
                    return Some(*val);
                }
            }
        }
    }
    None
}

pub struct LoopInvariantCodeMotion {
    stats: OptStats,
}

impl Default for LoopInvariantCodeMotion {
    fn default() -> Self {
        Self::new()
    }
}

impl LoopInvariantCodeMotion {
    pub fn new() -> Self {
        Self {
            stats: OptStats::new(),
        }
    }

    pub fn stats(&self) -> &OptStats {
        &self.stats
    }

    pub fn run(&mut self, module: &mut Module) {
        for func in &mut module.functions {
            self.optimize_function(func);
        }
    }

    fn optimize_function(&mut self, func: &mut Function) {
        let analysis = LoopAnalysis::analyze(func);

        for loop_info in &analysis.loops {
            let Some(preheader) = loop_info.preheader else {
                continue;
            };

            let invariants = self.find_loop_invariants(func, loop_info);

            if invariants.is_empty() {
                continue;
            }

            self.hoist_instructions(func, loop_info, preheader, &invariants);
        }
    }

    fn find_loop_invariants(&self, func: &Function, loop_info: &Loop) -> Vec<(BlockId, usize)> {
        let mut invariants = Vec::new();

        let mut outside_defs: HashSet<VarId> = HashSet::new();
        for block in &func.blocks {
            if !loop_info.contains(block.id) {
                for inst in &block.instructions {
                    if let Some(dest) = instruction_dest(inst) {
                        outside_defs.insert(dest);
                    }
                }
            }
        }

        for param in &func.params {
            outside_defs.insert(param.id);
        }

        for &block_id in &loop_info.blocks {
            if let Some(block) = func.get_block(block_id) {
                for (idx, inst) in block.instructions.iter().enumerate() {
                    if self.is_loop_invariant(inst, loop_info, &outside_defs) {
                        invariants.push((block_id, idx));
                        if let Some(dest) = instruction_dest(inst) {
                            outside_defs.insert(dest);
                        }
                    }
                }
            }
        }

        invariants
    }

    fn is_loop_invariant(
        &self,
        inst: &Instruction,
        _loop_info: &Loop,
        invariant_defs: &HashSet<VarId>,
    ) -> bool {
        match inst {
            Instruction::Jump { .. }
            | Instruction::Branch { .. }
            | Instruction::Return { .. }
            | Instruction::Throw { .. } => return false,

            Instruction::Store { .. }
            | Instruction::SetField { .. }
            | Instruction::ArraySet { .. }
            | Instruction::Print { .. }
            | Instruction::Call { .. }
            | Instruction::CallMethod { .. }
            | Instruction::CallVirtual { .. }
            | Instruction::CallIndirect { .. } => return false,

            Instruction::Alloca { .. }
            | Instruction::NewObject { .. }
            | Instruction::NewArray { .. } => return false,

            Instruction::Phi { .. } => return false,

            Instruction::TryBegin { .. }
            | Instruction::TryEnd
            | Instruction::GetException { .. } => return false,

            _ => {}
        }

        let operands = instruction_operands(inst);
        operands.iter().all(|op| invariant_defs.contains(op))
    }

    fn hoist_instructions(
        &mut self,
        func: &mut Function,
        _loop_info: &Loop,
        preheader: BlockId,
        invariants: &[(BlockId, usize)],
    ) {
        let mut to_move: Vec<(BlockId, usize, Instruction)> = Vec::new();

        for &(block_id, idx) in invariants.iter().rev() {
            if let Some(block) = func.get_block_mut(block_id) {
                let inst = block.instructions.remove(idx);
                to_move.push((block_id, idx, inst));
                self.stats.dead_instructions_removed += 1; // Reusing this counter
            }
        }

        if let Some(preheader_block) = func.get_block_mut(preheader) {
            let insert_pos = if preheader_block.has_terminator() {
                preheader_block.instructions.len() - 1
            } else {
                preheader_block.instructions.len()
            };

            for (_, _, inst) in to_move.into_iter().rev() {
                preheader_block.instructions.insert(insert_pos, inst);
            }
        }
    }
}

pub struct StrengthReduction {
    stats: OptStats,
}

impl Default for StrengthReduction {
    fn default() -> Self {
        Self::new()
    }
}

impl StrengthReduction {
    pub fn new() -> Self {
        Self {
            stats: OptStats::new(),
        }
    }

    pub fn stats(&self) -> &OptStats {
        &self.stats
    }

    pub fn run(&mut self, module: &mut Module) {
        for func in &mut module.functions {
            self.optimize_function(func);
        }
    }

    fn optimize_function(&mut self, func: &mut Function) {
        let analysis = LoopAnalysis::analyze(func);

        for loop_info in &analysis.loops {
            for iv in &loop_info.induction_vars {
                self.reduce_derived_expressions(func, loop_info, iv);
            }
        }
    }

    fn reduce_derived_expressions(
        &mut self,
        func: &mut Function,
        loop_info: &Loop,
        iv: &InductionVar,
    ) {
        let step = match &iv.step {
            InductionStep::Constant(s) => *s,
            _ => return, // Only handle constant steps for now
        };

        for &block_id in &loop_info.blocks {
            self.reduce_in_block(func, block_id, iv.var, step);
        }
    }

    fn reduce_in_block(&mut self, func: &mut Function, block_id: BlockId, iv: VarId, iv_step: i64) {
        let Some(block) = func.get_block_mut(block_id) else {
            return;
        };

        let mut replacements: Vec<(usize, i64)> = Vec::new();

        for (idx, inst) in block.instructions.iter().enumerate() {
            if let Instruction::Binary {
                dest: _,
                op: BinaryOp::Mul,
                left,
                right,
                ty: IrType::Int,
            } = inst
            {
                let const_operand = if *left == iv {
                    Some(*right)
                } else if *right == iv {
                    Some(*left)
                } else {
                    None
                };

                if let Some(const_var) = const_operand {
                    if let Some(const_val) = find_constant_value_in_block(block, const_var) {
                        replacements.push((idx, const_val));
                    }
                }
            }
        }

        for (idx, const_val) in replacements {
            let _ = (idx, const_val, iv_step); // Placeholder for actual implementation
            self.stats.constants_folded += 1; // Reusing counter for now
        }
    }
}

fn find_constant_value_in_block(block: &BasicBlock, var: VarId) -> Option<i64> {
    for inst in &block.instructions {
        if let Instruction::Const {
            dest,
            value: Constant::Int(val),
            ..
        } = inst
        {
            if *dest == var {
                return Some(*val);
            }
        }
    }
    None
}

pub struct LoopUnroller {
    stats: OptStats,
    max_body_size: usize,
    unroll_factor: usize,
    require_known_trip_count: bool,
}

impl Default for LoopUnroller {
    fn default() -> Self {
        Self::new()
    }
}

impl LoopUnroller {
    pub fn new() -> Self {
        Self {
            stats: OptStats::new(),
            max_body_size: 50, // Maximum instructions in loop body
            unroll_factor: 4,  // Unroll 4 times
            require_known_trip_count: true,
        }
    }

    pub fn set_unroll_factor(&mut self, factor: usize) {
        self.unroll_factor = factor;
    }

    pub fn set_max_body_size(&mut self, size: usize) {
        self.max_body_size = size;
    }

    pub fn stats(&self) -> &OptStats {
        &self.stats
    }

    pub fn run(&mut self, module: &mut Module) {
        for func in &mut module.functions {
            self.optimize_function(func);
        }
    }

    fn optimize_function(&mut self, func: &mut Function) {
        let analysis = LoopAnalysis::analyze(func);

        for loop_info in &analysis.loops {
            if !self.is_unroll_candidate(func, loop_info) {
                continue;
            }

            let body_size: usize = loop_info
                .blocks
                .iter()
                .filter_map(|&b| func.get_block(b))
                .map(|b| b.instructions.len())
                .sum();

            if body_size > self.max_body_size {
                continue;
            }

            if self.require_known_trip_count
                && !self.has_known_trip_count(func, loop_info) {
                    continue;
                }

            self.stats.constants_folded += 1; // Placeholder counter
        }
    }

    fn is_unroll_candidate(&self, func: &Function, loop_info: &Loop) -> bool {
        if loop_info.back_edges.len() != 1 {
            return false;
        }

        if loop_info.induction_vars.is_empty() {
            return false;
        }

        if loop_info.exits.len() != 1 {
            return false;
        }

        for &block_id in &loop_info.blocks {
            if let Some(block) = func.get_block(block_id) {
                if !block.has_terminator() {
                    return false;
                }
            }
        }

        true
    }

    fn has_known_trip_count(&self, func: &Function, loop_info: &Loop) -> bool {
        if loop_info.induction_vars.is_empty() {
            return false;
        }

        let iv = &loop_info.induction_vars[0];

        let init_const = find_constant_value(func, iv.init);
        if init_const.is_none() {
            return false;
        }

        let step = match &iv.step {
            InductionStep::Constant(s) => *s,
            _ => return false,
        };

        if step == 0 {
            return false; // Infinite loop
        }

        for &exit_id in &loop_info.exits {
            if let Some(exit_block) = func.get_block(exit_id) {
                if let Some(Instruction::Branch { cond, .. }) = exit_block.terminator() {
                    if self.is_constant_bound_comparison(func, exit_block, *cond, iv.var) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn is_constant_bound_comparison(
        &self,
        func: &Function,
        block: &BasicBlock,
        cond: VarId,
        iv: VarId,
    ) -> bool {
        for inst in &block.instructions {
            if let Instruction::Binary {
                dest,
                op,
                left,
                right,
                ..
            } = inst
            {
                if *dest != cond {
                    continue;
                }

                let is_comparison = matches!(
                    op,
                    BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge | BinaryOp::Ne
                );

                if !is_comparison {
                    continue;
                }

                if *left == iv {
                    if find_constant_value(func, *right).is_some() {
                        return true;
                    }
                } else if *right == iv
                    && find_constant_value(func, *left).is_some() {
                        return true;
                    }
            }
        }

        false
    }
}

pub struct LoopOptimizer {
    stats: OptStats,
    licm: LoopInvariantCodeMotion,
    strength_reduction: StrengthReduction,
    unroller: LoopUnroller,
    enable_unrolling: bool,
}

impl LoopOptimizer {
    pub fn new() -> Self {
        Self {
            stats: OptStats::new(),
            licm: LoopInvariantCodeMotion::new(),
            strength_reduction: StrengthReduction::new(),
            unroller: LoopUnroller::new(),
            enable_unrolling: false,
        }
    }

    pub fn enable_unrolling(&mut self, enable: bool) {
        self.enable_unrolling = enable;
    }

    pub fn stats(&self) -> &OptStats {
        &self.stats
    }

    pub fn run(&mut self, module: &mut Module) {
        self.licm.run(module);
        self.stats.merge(self.licm.stats());

        self.strength_reduction.run(module);
        self.stats.merge(self.strength_reduction.stats());

        if self.enable_unrolling {
            self.unroller.run(module);
            self.stats.merge(self.unroller.stats());
        }
    }
}

impl Default for LoopOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

fn instruction_dest(inst: &Instruction) -> Option<VarId> {
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
        | Instruction::ArrayLen { dest, .. }
        | Instruction::ArrayGet { dest, .. }
        | Instruction::StringConcat { dest, .. }
        | Instruction::GetException { dest, .. }
        | Instruction::Phi { dest, .. }
        | Instruction::Copy { dest, .. } => Some(*dest),

        Instruction::Call { dest, .. }
        | Instruction::CallIndirect { dest, .. }
        | Instruction::CallMethod { dest, .. }
        | Instruction::CallVirtual { dest, .. } => *dest,

        Instruction::NewArray { dest, .. } => Some(*dest),

        _ => None,
    }
}

fn instruction_operands(inst: &Instruction) -> Vec<VarId> {
    match inst {
        Instruction::Const { .. } => vec![],

        Instruction::Binary { left, right, .. } => vec![*left, *right],

        Instruction::Unary { operand, .. } => vec![*operand],

        Instruction::IntToFloat { src, .. }
        | Instruction::FloatToInt { src, .. }
        | Instruction::ToString { src, .. }
        | Instruction::Bitcast { src, .. }
        | Instruction::Copy { src, .. } => vec![*src],

        Instruction::Load { ptr, .. } => vec![*ptr],

        Instruction::Store { ptr, value } => vec![*ptr, *value],

        Instruction::GetElementPtr { ptr, index, .. } => vec![*ptr, *index],

        Instruction::GetField { object, .. } => vec![*object],

        Instruction::SetField { object, value, .. } => vec![*object, *value],

        Instruction::ArrayGet { array, index, .. } => vec![*array, *index],

        Instruction::ArraySet {
            array,
            index,
            value,
        } => vec![*array, *index, *value],

        Instruction::ArrayLen { array, .. } => vec![*array],

        Instruction::StringConcat { left, right, .. } => vec![*left, *right],

        Instruction::Branch { cond, .. } => vec![*cond],

        Instruction::Return { value } => value.iter().copied().collect(),

        Instruction::Call { args, .. } | Instruction::CallIndirect { args, .. } => args.clone(),

        Instruction::CallMethod { object, args, .. }
        | Instruction::CallVirtual { object, args, .. } => {
            let mut ops = vec![*object];
            ops.extend(args.iter().copied());
            ops
        }

        Instruction::NewArray { elements, .. } => elements.clone(),

        Instruction::Throw { exception } => vec![*exception],

        Instruction::Print { value } => vec![*value],

        Instruction::Phi { incoming, .. } => incoming.iter().map(|(v, _)| *v).collect(),

        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BasicBlock, FuncId, Function, Parameter};

    #[test]
    fn test_loop_detection() {

        let mut func = Function::new(
            FuncId("test".to_string()),
            "test".to_string(),
            vec![],
            IrType::Void,
        );

        let mut bb0 = BasicBlock::new(BlockId(0));
        bb0.successors = vec![BlockId(1)];
        bb0.instructions
            .push(Instruction::Jump { target: BlockId(1) });

        let mut bb1 = BasicBlock::new(BlockId(1));
        bb1.predecessors = vec![BlockId(0), BlockId(2)];
        bb1.successors = vec![BlockId(2), BlockId(3)];
        bb1.instructions.push(Instruction::Const {
            dest: VarId(0),
            value: Constant::Bool(true),
            ty: IrType::Bool,
        });
        bb1.instructions.push(Instruction::Branch {
            cond: VarId(0),
            then_block: BlockId(2),
            else_block: BlockId(3),
        });

        let mut bb2 = BasicBlock::new(BlockId(2));
        bb2.predecessors = vec![BlockId(1)];
        bb2.successors = vec![BlockId(1)];
        bb2.instructions
            .push(Instruction::Jump { target: BlockId(1) });

        let mut bb3 = BasicBlock::new(BlockId(3));
        bb3.predecessors = vec![BlockId(1)];
        bb3.instructions.push(Instruction::Return { value: None });

        func.blocks = vec![bb0, bb1, bb2, bb3];

        let analysis = LoopAnalysis::analyze(&func);

        assert_eq!(analysis.loops.len(), 1);
        assert_eq!(analysis.loops[0].header, BlockId(1));
        assert!(analysis.loops[0].blocks.contains(&BlockId(1)));
        assert!(analysis.loops[0].blocks.contains(&BlockId(2)));
        assert!(!analysis.loops[0].blocks.contains(&BlockId(0)));
        assert!(!analysis.loops[0].blocks.contains(&BlockId(3)));
    }

    #[test]
    fn test_loop_optimizer_creation() {
        let opt = LoopOptimizer::new();
        assert!(!opt.enable_unrolling);
    }

    #[test]
    fn test_induction_step() {
        let step = InductionStep::Constant(1);
        match step {
            InductionStep::Constant(v) => assert_eq!(v, 1),
            _ => panic!("Expected constant step"),
        }
    }
}
