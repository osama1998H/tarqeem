//! JIT Runtime Support
//!
//! This module provides runtime support for JIT-compiled code including:
//! - On-Stack Replacement (OSR) for entering compiled code mid-execution
//! - Deoptimization for falling back to interpreter
//! - Background compilation for non-blocking tier-up
//!
//! دعم وقت التشغيل للترجمة الفورية
//! يشمل الاستبدال على المكدس، إلغاء التحسين، والترجمة في الخلفية

mod background;
mod deopt;
mod osr;

pub use background::{BackgroundCompiler, CompileRequest, CompileResult};
pub use deopt::{DeoptInfo, DeoptReason, Deoptimizer};
pub use osr::{OsrEntry, OsrState, OsrTrigger};
