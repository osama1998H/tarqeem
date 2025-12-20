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

mod const_fold;
mod cse;
mod dce;
mod inline;

pub use const_fold::ConstantFolder;
pub use cse::CommonSubexprElim;
pub use dce::DeadCodeEliminator;
pub use inline::FunctionInliner;

use super::Module;

/// Optimization levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptLevel {
    /// No optimization (default for development)
    O0,
    /// Basic optimizations (constant folding, DCE)
    O1,
    /// Standard optimizations (+ CSE)
    O2,
    /// Aggressive optimizations (+ inlining)
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

/// Statistics collected during optimization
#[derive(Debug, Clone, Default)]
pub struct OptStats {
    /// Number of constants folded
    pub constants_folded: usize,
    /// Number of dead instructions removed
    pub dead_instructions_removed: usize,
    /// Number of dead blocks removed
    pub dead_blocks_removed: usize,
    /// Number of common subexpressions eliminated
    pub cse_hits: usize,
    /// Number of functions inlined
    pub functions_inlined: usize,
}

impl OptStats {
    /// Create new empty statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge another stats into this one
    pub fn merge(&mut self, other: &OptStats) {
        self.constants_folded += other.constants_folded;
        self.dead_instructions_removed += other.dead_instructions_removed;
        self.dead_blocks_removed += other.dead_blocks_removed;
        self.cse_hits += other.cse_hits;
        self.functions_inlined += other.functions_inlined;
    }

    /// Check if any optimizations were performed
    pub fn any_changes(&self) -> bool {
        self.constants_folded > 0
            || self.dead_instructions_removed > 0
            || self.dead_blocks_removed > 0
            || self.cse_hits > 0
            || self.functions_inlined > 0
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
        Ok(())
    }
}

/// The optimization pipeline
pub struct Optimizer {
    level: OptLevel,
    stats: OptStats,
    /// Maximum iterations for fixed-point optimization
    max_iterations: usize,
}

impl Optimizer {
    /// Create a new optimizer with the given level
    pub fn new(level: OptLevel) -> Self {
        Self {
            level,
            stats: OptStats::new(),
            max_iterations: 10,
        }
    }

    /// Get the current optimization level
    pub fn level(&self) -> OptLevel {
        self.level
    }

    /// Get optimization statistics
    pub fn stats(&self) -> &OptStats {
        &self.stats
    }

    /// Set maximum iterations for fixed-point optimization
    pub fn set_max_iterations(&mut self, max: usize) {
        self.max_iterations = max;
    }

    /// Run the optimization pipeline on a module
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

            // O3: Function inlining
            if self.level >= OptLevel::O3 {
                let mut inliner = FunctionInliner::new();
                inliner.run(module);
                if inliner.stats().functions_inlined > 0 {
                    changed = true;
                    self.stats.merge(inliner.stats());
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
