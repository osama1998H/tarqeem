//! IR Optimization Passes
//!
//! This module provides optimization passes for the Tarqeem IR.
//! Optimizations are applied in a configurable pipeline based on
//! optimization level (-O0, -O1, -O2, -O3).
//!
//! ## Available Passes
//!
//! - **Constant Folding**: Evaluate constant expressions at compile time
//! - **Dead Code Elimination**: Remove unused variables and unreachable blocks
//! - **Common Subexpression Elimination**: Cache and reuse repeated computations
//! - **Function Inlining**: Replace function calls with function bodies
//! - **Loop Optimizations**: LICM, strength reduction, loop unrolling

mod const_fold;
mod cse;
mod dce;
mod inline;
pub mod loop_opt;

pub use const_fold::ConstantFolder;
pub use cse::CommonSubexprElim;
pub use dce::DeadCodeEliminator;
pub use inline::FunctionInliner;
pub use loop_opt::{LoopAnalysis, LoopOptimizer};

use super::Module;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptLevel {
    O0,
    O1,
    O2,
    O3,
}

impl Default for OptLevel {
    fn default() -> Self {
        OptLevel::O0
    }
}

impl std::fmt::Display for OptLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OptLevel::O0 => write!(f, "-O0"),
            OptLevel::O1 => write!(f, "-O1"),
            OptLevel::O2 => write!(f, "-O2"),
            OptLevel::O3 => write!(f, "-O3"),
        }
    }
}

impl std::str::FromStr for OptLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" | "O0" | "-O0" => Ok(OptLevel::O0),
            "1" | "O1" | "-O1" => Ok(OptLevel::O1),
            "2" | "O2" | "-O2" => Ok(OptLevel::O2),
            "3" | "O3" | "-O3" => Ok(OptLevel::O3),
            _ => Err(format!("Unknown optimization level: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OptStats {
    pub constants_folded: usize,
    pub dead_instructions_removed: usize,
    pub dead_blocks_removed: usize,
    pub cse_hits: usize,
    pub functions_inlined: usize,
    pub loop_invariants_hoisted: usize,
    pub loops_detected: usize,
}

impl OptStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge(&mut self, other: &OptStats) {
        self.constants_folded += other.constants_folded;
        self.dead_instructions_removed += other.dead_instructions_removed;
        self.dead_blocks_removed += other.dead_blocks_removed;
        self.cse_hits += other.cse_hits;
        self.functions_inlined += other.functions_inlined;
        self.loop_invariants_hoisted += other.loop_invariants_hoisted;
        self.loops_detected += other.loops_detected;
    }

    pub fn any_changes(&self) -> bool {
        self.constants_folded > 0
            || self.dead_instructions_removed > 0
            || self.dead_blocks_removed > 0
            || self.cse_hits > 0
            || self.functions_inlined > 0
            || self.loop_invariants_hoisted > 0
    }
}

impl std::fmt::Display for OptStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Optimization Statistics:")?;
        writeln!(f, "  Constants folded: {}", self.constants_folded)?;
        writeln!(
            f,
            "  Dead instructions removed: {}",
            self.dead_instructions_removed
        )?;
        writeln!(f, "  Dead blocks removed: {}", self.dead_blocks_removed)?;
        writeln!(f, "  CSE hits: {}", self.cse_hits)?;
        writeln!(f, "  Functions inlined: {}", self.functions_inlined)?;
        if self.loops_detected > 0 {
            writeln!(f, "  Loops detected: {}", self.loops_detected)?;
            writeln!(
                f,
                "  Loop invariants hoisted: {}",
                self.loop_invariants_hoisted
            )?;
        }
        Ok(())
    }
}

pub struct Optimizer {
    level: OptLevel,
    stats: OptStats,
    max_iterations: usize,
}

impl Optimizer {
    pub fn new(level: OptLevel) -> Self {
        Self {
            level,
            stats: OptStats::new(),
            max_iterations: 10,
        }
    }

    pub fn level(&self) -> OptLevel {
        self.level
    }

    pub fn stats(&self) -> &OptStats {
        &self.stats
    }

    pub fn set_max_iterations(&mut self, max: usize) {
        self.max_iterations = max;
    }

    pub fn optimize(&mut self, module: &mut Module) {
        if self.level == OptLevel::O0 {
            return; // No optimization
        }

        // Run optimization passes in a fixed-point loop
        for _ in 0..self.max_iterations {
            let mut changed = false;

            // O1+: Constant folding
            if self.level >= OptLevel::O1 {
                let mut folder = ConstantFolder::new();
                folder.run(module);
                if folder.stats().constants_folded > 0 {
                    changed = true;
                    self.stats.merge(folder.stats());
                }
            }

            // O1+: Dead code elimination
            if self.level >= OptLevel::O1 {
                let mut dce = DeadCodeEliminator::new();
                dce.run(module);
                if dce.stats().dead_instructions_removed > 0 || dce.stats().dead_blocks_removed > 0
                {
                    changed = true;
                    self.stats.merge(dce.stats());
                }
            }

            // O2+: Common subexpression elimination
            if self.level >= OptLevel::O2 {
                let mut cse = CommonSubexprElim::new();
                cse.run(module);
                if cse.stats().cse_hits > 0 {
                    changed = true;
                    self.stats.merge(cse.stats());
                }
            }

            // O2+: Loop optimizations (LICM, strength reduction)
            if self.level >= OptLevel::O2 {
                let mut loop_opt = LoopOptimizer::new();
                loop_opt.run(module);
                if loop_opt.stats().any_changes() {
                    changed = true;
                    self.stats.merge(loop_opt.stats());
                }
            }

            // O3: Function inlining
            if self.level >= OptLevel::O3 {
                let mut inliner = FunctionInliner::new();
                inliner.run(module);
                if inliner.stats().functions_inlined > 0 {
                    changed = true;
                    self.stats.merge(inliner.stats());
                }
            }

            // O3: Loop unrolling (more aggressive)
            if self.level >= OptLevel::O3 {
                let mut loop_opt = LoopOptimizer::new();
                loop_opt.enable_unrolling(true);
                loop_opt.run(module);
                if loop_opt.stats().any_changes() {
                    changed = true;
                    self.stats.merge(loop_opt.stats());
                }
            }

            // If no changes were made, we've reached a fixed point
            if !changed {
                break;
            }
        }
    }
}

impl Default for Optimizer {
    fn default() -> Self {
        Self::new(OptLevel::O0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opt_level_parsing() {
        assert_eq!("0".parse::<OptLevel>().unwrap(), OptLevel::O0);
        assert_eq!("1".parse::<OptLevel>().unwrap(), OptLevel::O1);
        assert_eq!("-O2".parse::<OptLevel>().unwrap(), OptLevel::O2);
        assert_eq!("O3".parse::<OptLevel>().unwrap(), OptLevel::O3);
    }

    #[test]
    fn test_opt_stats_merge() {
        let mut stats1 = OptStats {
            constants_folded: 5,
            dead_instructions_removed: 3,
            ..Default::default()
        };

        let stats2 = OptStats {
            constants_folded: 2,
            cse_hits: 4,
            ..Default::default()
        };

        stats1.merge(&stats2);
        assert_eq!(stats1.constants_folded, 7);
        assert_eq!(stats1.dead_instructions_removed, 3);
        assert_eq!(stats1.cse_hits, 4);
    }
}
