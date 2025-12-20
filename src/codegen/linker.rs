//! Linker Integration
//!
//! This module handles compiling LLVM IR to object files and linking
//! to create executables.

use super::Target;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Linker for creating executables from LLVM IR
pub struct Linker {
    /// Target configuration
    target: Target,
    /// Path to clang (if available)
    clang_path: Option<PathBuf>,
    /// Path to llc (if available)
    llc_path: Option<PathBuf>,
    /// Path to system linker
    ld_path: Option<PathBuf>,
    /// Optimization level (0-3)
    opt_level: u32,
    /// Enable debug info
    debug: bool,
    /// Verbose output
    verbose: bool,
}

impl Linker {
    /// Create a new linker for the given target
    pub fn new(target: Target) -> Self {
        let clang_path = find_program("clang");
        let llc_path = find_program("llc");
        let ld_path = find_linker(&target);

        Self {
            target,
            clang_path,
            llc_path,
            ld_path,
            opt_level: 0,
            debug: false,
            verbose: false,
        }
    }

    /// Set optimization level
    pub fn optimization_level(mut self, level: u32) -> Self {
        self.opt_level = level.min(3);
        self
    }

    /// Enable debug info
    pub fn debug(mut self, enable: bool) -> Self {
        self.debug = enable;
        self
    }

    /// Enable verbose output
    pub fn verbose(mut self, enable: bool) -> Self {
        self.verbose = enable;
        self
    }

    /// Compile LLVM IR to an object file
    pub fn compile_to_object(&self, llvm_ir: &str, output: &Path) -> Result<(), LinkerError> {
        // Write IR to temporary file
        let ir_path = output.with_extension("ll");
        fs::write(&ir_path, llvm_ir).map_err(|e| LinkerError {
            message: format!("Failed to write LLVM IR: {}", e),
            message_ar: format!("فشل في كتابة LLVM IR: {}", e),
        })?;

        // Prefer clang if available
        if let Some(ref clang) = self.clang_path {
            self.compile_with_clang(clang, &ir_path, output)?;
        } else if let Some(ref llc) = self.llc_path {
            self.compile_with_llc(llc, &ir_path, output)?;
        } else {
            return Err(LinkerError {
                message: "No compiler found. Install clang or llc.".to_string(),
                message_ar: "لم يتم العثور على مترجم. ثبّت clang أو llc.".to_string(),
            });
        }

        // Clean up IR file if not in verbose mode
        if !self.verbose {
            let _ = fs::remove_file(&ir_path);
        }

        Ok(())
    }

    /// Compile LLVM IR directly to an executable
    pub fn compile_to_executable(
        &self,
        llvm_ir: &str,
        output: &Path,
        runtime_path: Option<&Path>,
    ) -> Result<(), LinkerError> {
        // Write IR to temporary file
        let ir_path = output.with_extension("ll");
        fs::write(&ir_path, llvm_ir).map_err(|e| LinkerError {
            message: format!("Failed to write LLVM IR: {}", e),
            message_ar: format!("فشل في كتابة LLVM IR: {}", e),
        })?;

        // If we have clang, use it to compile and link in one step
        if let Some(ref clang) = self.clang_path {
            self.compile_and_link_with_clang(clang, &ir_path, output, runtime_path)?;
        } else {
            // Compile to object first, then link
            let obj_path = output.with_extension("o");
            self.compile_to_object(llvm_ir, &obj_path)?;
            self.link_object(&obj_path, output, runtime_path)?;

            // Clean up object file
            if !self.verbose {
                let _ = fs::remove_file(&obj_path);
            }
        }

        // Clean up IR file if not in verbose mode
        if !self.verbose {
            let _ = fs::remove_file(&ir_path);
        }

        Ok(())
    }

    /// Compile LLVM IR to assembly
    pub fn compile_to_assembly(&self, llvm_ir: &str, output: &Path) -> Result<(), LinkerError> {
        // Write IR to temporary file
        let ir_path = output.with_extension("ll");
        fs::write(&ir_path, llvm_ir).map_err(|e| LinkerError {
            message: format!("Failed to write LLVM IR: {}", e),
            message_ar: format!("فشل في كتابة LLVM IR: {}", e),
        })?;

        if let Some(ref clang) = self.clang_path {
            let mut cmd = Command::new(clang);
            cmd.arg("-S")
                .arg("-o")
                .arg(output)
                .arg(&ir_path)
                .arg(format!("-O{}", self.opt_level))
                .arg("-target")
                .arg(self.target.llvm_triple());

            if self.debug {
                cmd.arg("-g");
            }

            self.run_command(cmd, "clang")?;
        } else if let Some(ref llc) = self.llc_path {
            let mut cmd = Command::new(llc);
            cmd.arg("-o")
                .arg(output)
                .arg(&ir_path)
                .arg(format!("-O={}", self.opt_level))
                .arg("-mtriple")
                .arg(self.target.llvm_triple());

            self.run_command(cmd, "llc")?;
        } else {
            return Err(LinkerError {
                message: "No compiler found. Install clang or llc.".to_string(),
                message_ar: "لم يتم العثور على مترجم. ثبّت clang أو llc.".to_string(),
            });
        }

        // Clean up IR file
        if !self.verbose {
            let _ = fs::remove_file(&ir_path);
        }

        Ok(())
    }

    /// Compile with clang
    fn compile_with_clang(
        &self,
        clang: &Path,
        ir_path: &Path,
        output: &Path,
    ) -> Result<(), LinkerError> {
        let mut cmd = Command::new(clang);
        cmd.arg("-c")
            .arg("-o")
            .arg(output)
            .arg(ir_path)
            .arg(format!("-O{}", self.opt_level))
            .arg("-target")
            .arg(self.target.llvm_triple());

        if self.debug {
            cmd.arg("-g");
        }

        self.run_command(cmd, "clang")
    }

    /// Compile with llc
    fn compile_with_llc(
        &self,
        llc: &Path,
        ir_path: &Path,
        output: &Path,
    ) -> Result<(), LinkerError> {
        let mut cmd = Command::new(llc);
        cmd.arg("-filetype=obj")
            .arg("-o")
            .arg(output)
            .arg(ir_path)
            .arg(format!("-O={}", self.opt_level))
            .arg("-mtriple")
            .arg(self.target.llvm_triple());

        self.run_command(cmd, "llc")
    }

    /// Compile and link with clang in one step
    fn compile_and_link_with_clang(
        &self,
        clang: &Path,
        ir_path: &Path,
        output: &Path,
        runtime_path: Option<&Path>,
    ) -> Result<(), LinkerError> {
        let mut cmd = Command::new(clang);
        cmd.arg("-o")
            .arg(output)
            .arg(ir_path)
            .arg(format!("-O{}", self.opt_level))
            .arg("-target")
            .arg(self.target.llvm_triple());

        if self.debug {
            cmd.arg("-g");
        }

        // Link against runtime library if provided
        if let Some(runtime) = runtime_path {
            cmd.arg(runtime);
        }

        // Link against libc and libm
        cmd.arg("-lc").arg("-lm");

        self.run_command(cmd, "clang")
    }

    /// Link object file to executable
    fn link_object(
        &self,
        obj_path: &Path,
        output: &Path,
        runtime_path: Option<&Path>,
    ) -> Result<(), LinkerError> {
        let ld = self.ld_path.as_ref().ok_or_else(|| LinkerError {
            message: "No linker found.".to_string(),
            message_ar: "لم يتم العثور على رابط.".to_string(),
        })?;

        let mut cmd = Command::new(ld);
        cmd.arg("-o").arg(output).arg(obj_path);

        // Add runtime library
        if let Some(runtime) = runtime_path {
            cmd.arg(runtime);
        }

        // Platform-specific linker flags
        if self.target.triple.os == "linux" {
            // Link against system libraries
            cmd.arg("-dynamic-linker")
                .arg("/lib64/ld-linux-x86-64.so.2")
                .arg("-lc")
                .arg("-lm");
        } else if self.target.triple.os == "darwin" {
            cmd.arg("-lSystem");
        }

        self.run_command(cmd, "linker")
    }

    /// Run a command and check for errors
    fn run_command(&self, mut cmd: Command, name: &str) -> Result<(), LinkerError> {
        if self.verbose {
            eprintln!("Running: {:?}", cmd);
        }

        let output = cmd.output().map_err(|e| LinkerError {
            message: format!("Failed to run {}: {}", name, e),
            message_ar: format!("فشل في تشغيل {}: {}", name, e),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(LinkerError {
                message: format!("{} failed: {}", name, stderr),
                message_ar: format!("فشل {}: {}", name, stderr),
            });
        }

        Ok(())
    }

    /// Check if compilation is available
    pub fn is_available(&self) -> bool {
        self.clang_path.is_some() || self.llc_path.is_some()
    }

    /// Get the clang version if available
    pub fn clang_version(&self) -> Option<String> {
        let clang = self.clang_path.as_ref()?;
        let output = Command::new(clang).arg("--version").output().ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.lines().next().map(|s| s.to_string())
    }
}

/// Linker error
#[derive(Debug)]
pub struct LinkerError {
    pub message: String,
    pub message_ar: String,
}

impl std::fmt::Display for LinkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LinkerError {}

/// Find a program in PATH
fn find_program(name: &str) -> Option<PathBuf> {
    // Try the command itself
    if Command::new(name).arg("--version").output().is_ok() {
        return Some(PathBuf::from(name));
    }

    // Try common paths
    let common_paths = [
        "/usr/bin",
        "/usr/local/bin",
        "/opt/homebrew/bin",
        "/opt/local/bin",
    ];

    for path in common_paths {
        let full_path = Path::new(path).join(name);
        if full_path.exists() {
            return Some(full_path);
        }
    }

    None
}

/// Find the system linker
fn find_linker(target: &Target) -> Option<PathBuf> {
    if target.triple.os == "darwin" {
        find_program("ld")
    } else {
        // Prefer ld.lld (LLVM linker) then ld
        find_program("ld.lld").or_else(|| find_program("ld"))
    }
}

/// Emit options for output type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitType {
    /// Emit LLVM IR (.ll)
    LlvmIr,
    /// Emit assembly (.s)
    Assembly,
    /// Emit object file (.o)
    Object,
    /// Emit executable
    Executable,
}

impl EmitType {
    /// Get the file extension for this emit type
    pub fn extension(&self) -> &'static str {
        match self {
            EmitType::LlvmIr => "ll",
            EmitType::Assembly => "s",
            EmitType::Object => "o",
            EmitType::Executable => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_type_extension() {
        assert_eq!(EmitType::LlvmIr.extension(), "ll");
        assert_eq!(EmitType::Assembly.extension(), "s");
        assert_eq!(EmitType::Object.extension(), "o");
        assert_eq!(EmitType::Executable.extension(), "");
    }
}
