//! Linker Integration
//!
//! This module handles compiling LLVM IR to object files and linking
//! to create executables. Supports native targets and WebAssembly.

use super::Target;
use crate::error::codes::{ERR_LINKING_FAILED, ERR_LLVM_INTERNAL, ERR_RUNTIME_NOT_FOUND};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Configuration for WebAssembly output
#[derive(Debug, Clone, Default)]
pub struct WasmConfig {
    /// Generate JavaScript bindings for browser use
    pub js_bindings: bool,
    /// Export all public functions (not just __main__)
    pub export_all: bool,
    /// Initial memory pages (64KB each)
    pub memory_pages: u32,
    /// Maximum memory pages (0 = unlimited)
    pub max_memory_pages: u32,
    /// Stack size in bytes
    pub stack_size: u32,
    /// Generate source map for debugging
    pub source_map: bool,
    /// List of functions to export
    pub exports: Vec<String>,
}

impl WasmConfig {
    pub fn new() -> Self {
        Self {
            js_bindings: false,
            export_all: false,
            memory_pages: 16,      // 1MB initial
            max_memory_pages: 256, // 16MB max
            stack_size: 65536,     // 64KB stack
            source_map: false,
            exports: vec!["__main__".to_string(), "main".to_string()],
        }
    }

    pub fn with_js_bindings(mut self) -> Self {
        self.js_bindings = true;
        self
    }

    pub fn with_export_all(mut self) -> Self {
        self.export_all = true;
        self
    }

    pub fn with_memory_pages(mut self, pages: u32) -> Self {
        self.memory_pages = pages;
        self
    }
}

pub struct Linker {
    target: Target,
    clang_path: Option<PathBuf>,
    llc_path: Option<PathBuf>,
    ld_path: Option<PathBuf>,
    wasm_ld_path: Option<PathBuf>,
    opt_level: u32,
    debug: bool,
    verbose: bool,
    wasm_config: WasmConfig,
}

impl Linker {
    pub fn new(target: Target) -> Self {
        let clang_path = find_program("clang");
        let llc_path = find_program("llc");
        let ld_path = find_linker(&target);
        let wasm_ld_path = if target.is_wasm() {
            find_program("wasm-ld")
        } else {
            None
        };

        Self {
            target,
            clang_path,
            llc_path,
            ld_path,
            wasm_ld_path,
            opt_level: 0,
            debug: false,
            verbose: false,
            wasm_config: WasmConfig::new(),
        }
    }

    pub fn optimization_level(mut self, level: u32) -> Self {
        self.opt_level = level.min(3);
        self
    }

    pub fn debug(mut self, enable: bool) -> Self {
        self.debug = enable;
        self
    }

    pub fn verbose(mut self, enable: bool) -> Self {
        self.verbose = enable;
        self
    }

    /// Configure WebAssembly-specific options
    pub fn wasm_config(mut self, config: WasmConfig) -> Self {
        self.wasm_config = config;
        self
    }

    /// Check if WASM compilation is available
    pub fn is_wasm_available(&self) -> bool {
        self.target.is_wasm() && (self.clang_path.is_some() || self.wasm_ld_path.is_some())
    }

    pub fn compile_to_object(&self, llvm_ir: &str, output: &Path) -> Result<(), LinkerError> {
        let ir_path = output.with_extension("ll");
        fs::write(&ir_path, llvm_ir).map_err(|e| {
            LinkerError::with_code(
                format!(
                    "فشل في كتابة التمثيل الوسيط الخاص بالآلة الافتراضية منخفضة المستوى: {}",
                    e
                ),
                ERR_LLVM_INTERNAL.to_string(),
            )
        })?;

        if let Some(ref clang) = self.clang_path {
            self.compile_with_clang(clang, &ir_path, output)?;
        } else if let Some(ref llc) = self.llc_path {
            self.compile_with_llc(llc, &ir_path, output)?;
        } else {
            return Err(LinkerError::with_code(
                "لم يتم العثور على مترجم. ثبّت clang أو llc.".to_string(),
                ERR_LLVM_INTERNAL.to_string(),
            ));
        }

        if !self.verbose {
            let _ = fs::remove_file(&ir_path);
        }

        Ok(())
    }

    pub fn compile_to_executable(
        &self,
        llvm_ir: &str,
        output: &Path,
        runtime_path: Option<&Path>,
    ) -> Result<(), LinkerError> {
        let runtime = match runtime_path {
            Some(path) => path.to_path_buf(),
            None => locate_runtime().map_err(|searched| runtime_not_found(&searched))?,
        };
        if self.verbose {
            eprintln!("مكتبة وقت التشغيل: {}", runtime.display());
        }
        let runtime = Some(runtime);

        let ir_path = output.with_extension("ll");
        fs::write(&ir_path, llvm_ir).map_err(|e| {
            LinkerError::with_code(
                format!(
                    "فشل في كتابة التمثيل الوسيط الخاص بالآلة الافتراضية منخفضة المستوى: {}",
                    e
                ),
                ERR_LLVM_INTERNAL.to_string(),
            )
        })?;

        if let Some(ref clang) = self.clang_path {
            self.compile_and_link_with_clang(clang, &ir_path, output, runtime.as_deref())?;
        } else {
            let obj_path = output.with_extension("o");
            self.compile_to_object(llvm_ir, &obj_path)?;
            self.link_object(&obj_path, output, runtime.as_deref())?;

            if !self.verbose {
                let _ = fs::remove_file(&obj_path);
            }
        }

        if !self.verbose {
            let _ = fs::remove_file(&ir_path);
        }

        Ok(())
    }

    pub fn compile_to_assembly(&self, llvm_ir: &str, output: &Path) -> Result<(), LinkerError> {
        let ir_path = output.with_extension("ll");
        fs::write(&ir_path, llvm_ir).map_err(|e| {
            LinkerError::with_code(
                format!(
                    "فشل في كتابة التمثيل الوسيط الخاص بالآلة الافتراضية منخفضة المستوى: {}",
                    e
                ),
                ERR_LLVM_INTERNAL.to_string(),
            )
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
            return Err(LinkerError::with_code(
                "لم يتم العثور على مترجم. ثبّت clang أو llc.".to_string(),
                ERR_LLVM_INTERNAL.to_string(),
            ));
        }

        if !self.verbose {
            let _ = fs::remove_file(&ir_path);
        }

        Ok(())
    }

    /// Compile LLVM IR to WebAssembly binary
    ///
    /// This method compiles the LLVM IR to a .wasm file. If js_bindings is enabled
    /// in the WasmConfig, it will also generate JavaScript bindings.
    pub fn compile_to_wasm(
        &self,
        llvm_ir: &str,
        output: &Path,
        runtime_path: Option<&Path>,
    ) -> Result<(), LinkerError> {
        if !self.target.is_wasm() {
            return Err(LinkerError::with_code(
                "الهدف ليس WebAssembly. استخدم --target wasm32-unknown-unknown".to_string(),
                ERR_LLVM_INTERNAL.to_string(),
            ));
        }

        // The wasm archive, never the native one: handing an aarch64 `libtrq.a`
        // to a wasm link is worse than linking without it. Nothing builds
        // `libtrq_wasm.a` yet, so `None` is the normal outcome and, unlike the
        // native path, must not be fatal.
        let runtime = runtime_path
            .map(|p| p.to_path_buf())
            .or_else(find_wasm_runtime);
        if self.verbose {
            match &runtime {
                Some(path) => eprintln!("مكتبة وقت التشغيل: {}", path.display()),
                None => eprintln!("تحذير: لم يتم العثور على {WASM_RUNTIME_LIB}."),
            }
        }

        let ir_path = output.with_extension("ll");
        fs::write(&ir_path, llvm_ir).map_err(|e| {
            LinkerError::with_code(
                format!(
                    "فشل في كتابة التمثيل الوسيط الخاص بالآلة الافتراضية منخفضة المستوى: {}",
                    e
                ),
                ERR_LLVM_INTERNAL.to_string(),
            )
        })?;

        // Use clang for WASM compilation (it handles the full pipeline)
        if let Some(ref clang) = self.clang_path {
            self.compile_wasm_with_clang(clang, &ir_path, output, runtime.as_deref())?;
        } else {
            return Err(LinkerError::with_code(
                "لم يتم العثور على clang. ثبّت LLVM مع دعم WebAssembly.".to_string(),
                ERR_LLVM_INTERNAL.to_string(),
            ));
        }

        // Generate JavaScript bindings if requested
        if self.wasm_config.js_bindings {
            self.generate_js_bindings(output)?;
        }

        if !self.verbose {
            let _ = fs::remove_file(&ir_path);
        }

        Ok(())
    }

    /// Compile LLVM IR to WASM using clang
    fn compile_wasm_with_clang(
        &self,
        clang: &Path,
        ir_path: &Path,
        output: &Path,
        runtime_path: Option<&Path>,
    ) -> Result<(), LinkerError> {
        let mut cmd = Command::new(clang);

        // Basic compilation flags
        cmd.arg("-o")
            .arg(output)
            .arg(ir_path)
            .arg(format!("-O{}", self.opt_level))
            .arg("--target=wasm32-unknown-unknown");

        // WASM-specific flags
        cmd.arg("-nostdlib"); // No standard library (we provide our own runtime)
        cmd.arg("-Wl,--no-entry"); // No _start entry point
        cmd.arg("-Wl,--allow-undefined"); // Allow undefined symbols (JS imports)

        // Memory configuration
        cmd.arg(format!(
            "-Wl,--initial-memory={}",
            self.wasm_config.memory_pages * 65536
        ));
        if self.wasm_config.max_memory_pages > 0 {
            cmd.arg(format!(
                "-Wl,--max-memory={}",
                self.wasm_config.max_memory_pages * 65536
            ));
        }
        cmd.arg(format!("-Wl,-z,stack-size={}", self.wasm_config.stack_size));

        // Export functions
        if self.wasm_config.export_all {
            cmd.arg("-Wl,--export-all");
        } else {
            for export in &self.wasm_config.exports {
                cmd.arg(format!("-Wl,--export={}", export));
            }
        }

        // Export memory and table for JavaScript access
        cmd.arg("-Wl,--export=memory");

        // Include WASM runtime if provided
        if let Some(runtime) = runtime_path {
            cmd.arg(runtime);
        }

        if self.debug {
            cmd.arg("-g");
        }

        self.run_command(cmd, "clang (wasm)")
    }

    /// Generate JavaScript bindings for the WASM module
    fn generate_js_bindings(&self, wasm_path: &Path) -> Result<(), LinkerError> {
        let js_path = wasm_path.with_extension("js");
        let wasm_filename = wasm_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("module.wasm");

        let js_bindings = generate_tarqeem_js_bindings(wasm_filename, &self.wasm_config);

        fs::write(&js_path, js_bindings).map_err(|e| {
            LinkerError::with_code(
                format!("فشل في كتابة روابط JavaScript: {}", e),
                ERR_LLVM_INTERNAL.to_string(),
            )
        })?;

        if self.verbose {
            eprintln!(
                "Generated JS bindings: {} / تم توليد روابط JavaScript: {}",
                js_path.display(),
                js_path.display()
            );
        }

        Ok(())
    }

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

        if let Some(runtime) = runtime_path {
            cmd.arg(runtime);
        }

        cmd.arg("-lc").arg("-lm").arg("-lz"); // -lz for zlib (compression)

        self.run_command(cmd, "clang")
    }

    fn link_object(
        &self,
        obj_path: &Path,
        output: &Path,
        runtime_path: Option<&Path>,
    ) -> Result<(), LinkerError> {
        let ld = self.ld_path.as_ref().ok_or_else(|| {
            LinkerError::with_code(
                "لم يتم العثور على رابط.".to_string(),
                ERR_LINKING_FAILED.to_string(),
            )
        })?;

        let mut cmd = Command::new(ld);
        cmd.arg("-o").arg(output).arg(obj_path);

        if let Some(runtime) = runtime_path {
            cmd.arg(runtime);
        }

        if self.target.triple.os == "linux" {
            cmd.arg("-dynamic-linker")
                .arg("/lib64/ld-linux-x86-64.so.2")
                .arg("-lc")
                .arg("-lm")
                .arg("-lz"); // zlib for compression
        } else if self.target.triple.os == "darwin" {
            cmd.arg("-lSystem").arg("-lz"); // zlib for compression
        }

        self.run_command(cmd, "linker")
    }

    fn run_command(&self, mut cmd: Command, name: &str) -> Result<(), LinkerError> {
        if self.verbose {
            eprintln!("Running: {:?}", cmd);
        }

        let output = cmd.output().map_err(|e| {
            LinkerError::with_code(
                format!("فشل في تشغيل {}: {}", name, e),
                ERR_LINKING_FAILED.to_string(),
            )
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(LinkerError::with_code(
                format!("فشل {}: {}", name, stderr),
                ERR_LINKING_FAILED.to_string(),
            ));
        }

        Ok(())
    }

    pub fn is_available(&self) -> bool {
        self.clang_path.is_some() || self.llc_path.is_some()
    }

    pub fn clang_version(&self) -> Option<String> {
        let clang = self.clang_path.as_ref()?;
        let output = Command::new(clang).arg("--version").output().ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.lines().next().map(|s| s.to_string())
    }
}

#[derive(Debug)]
pub struct LinkerError {
    pub message: String,
    pub code: Option<String>,
}

impl LinkerError {
    pub fn new(message: String) -> Self {
        Self {
            message,
            code: None,
        }
    }

    pub fn with_code(message: String, code: String) -> Self {
        Self {
            message,
            code: Some(code),
        }
    }
}

impl std::fmt::Display for LinkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref code) = self.code {
            write!(f, "[{}] {}", code, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for LinkerError {}

/// The static runtime archive every native executable links against.
pub const RUNTIME_LIB: &str = if cfg!(windows) { "trq.lib" } else { "libtrq.a" };

/// The WebAssembly counterpart. Nothing in the workspace builds this yet, so
/// discovery is expected to come up empty and the wasm link proceeds without it.
pub const WASM_RUNTIME_LIB: &str = "libtrq_wasm.a";

/// Which archive to look for, and the directory installers nest it under.
struct RuntimeLayout {
    lib_name: &'static str,
    /// `runtime` natively, `runtime/wasm` for the wasm archive.
    runtime_dir: &'static str,
    /// Whether the two `TARQEEM_RUNTIME_PATH` reads apply — the one exported
    /// into the process and the one `build.rs` bakes in. Both name the *native*
    /// archive, so the wasm search must consult neither.
    use_native_override: bool,
}

const NATIVE_LAYOUT: RuntimeLayout = RuntimeLayout {
    lib_name: RUNTIME_LIB,
    runtime_dir: "runtime",
    use_native_override: true,
};

const WASM_LAYOUT: RuntimeLayout = RuntimeLayout {
    lib_name: WASM_RUNTIME_LIB,
    runtime_dir: "runtime/wasm",
    use_native_override: false,
};

/// Everything discovery reads from outside the process, gathered up front so
/// that the ordering itself stays a pure function of its inputs.
#[derive(Debug, Default, Clone)]
struct RuntimeEnv {
    /// `TARQEEM_RUNTIME_PATH` as exported into the running process.
    explicit: Option<PathBuf>,
    /// The same variable as `build.rs` baked in. It arrives through
    /// `cargo:rustc-env`, which feeds `option_env!` and *not* `std::env::var` —
    /// reading it at runtime is why that priority never fired before (#285).
    baked: Option<PathBuf>,
    exe_dir: Option<PathBuf>,
    manifest_dir: Option<PathBuf>,
    tarqeem_home: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    /// Windows only, but carried unconditionally so `runtime_candidates` stays
    /// pure on every platform rather than reading the environment under `cfg`.
    #[cfg_attr(not(windows), allow(dead_code))]
    local_app_data: Option<PathBuf>,
}

impl RuntimeEnv {
    fn current() -> Self {
        Self {
            explicit: std::env::var_os("TARQEEM_RUNTIME_PATH").map(PathBuf::from),
            local_app_data: std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
            baked: option_env!("TARQEEM_RUNTIME_PATH").map(PathBuf::from),
            exe_dir: std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf)),
            manifest_dir: std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from),
            tarqeem_home: std::env::var_os("TARQEEM_HOME").map(PathBuf::from),
            home_dir: dirs::home_dir(),
        }
    }
}

/// Every location that may hold the runtime archive, most specific first.
///
/// The rule the order encodes: **the archive built alongside this compiler wins
/// over any installed copy.** `install.sh` and the `Makefile` both tell users to
/// export `TARQEEM_HOME`, so ranking it first — as this once did — meant a
/// developer's `cargo build` output was never linked again, and a newly added
/// runtime symbol read as an undefined-symbol error forever (#285).
///
/// Resolving against the executable is what makes one order serve both cases:
/// `target/release/tarqeem` sits beside `target/release/libtrq.a`, and
/// `~/.tarqeem/bin/tarqeem` sits beside `../lib/libtrq.a`.
///
/// Pure by design — no environment reads and no filesystem probing — so the
/// precedence can be asserted directly instead of through `set_var`.
fn runtime_candidates(env: &RuntimeEnv, layout: &RuntimeLayout) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let lib = layout.lib_name;

    if layout.use_native_override {
        if let Some(explicit) = &env.explicit {
            paths.push(explicit.clone());
        }
    }

    if let Some(dir) = &env.exe_dir {
        paths.push(dir.join(lib));
        paths.push(dir.join(layout.runtime_dir).join(lib));
        if let Some(parent) = dir.parent() {
            paths.push(parent.join(layout.runtime_dir).join(lib));
            paths.push(parent.join("lib").join(lib));
        }
    }

    if layout.use_native_override {
        if let Some(baked) = &env.baked {
            paths.push(baked.clone());
        }
    }

    // The profile this compiler was built with is the only one whose runtime
    // matches it; probing `release` first regardless would hand a debug build
    // the release archive.
    if let Some(manifest) = &env.manifest_dir {
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        paths.push(manifest.join("target").join(profile).join(lib));
    }

    if let Some(home) = &env.tarqeem_home {
        paths.push(home.join("lib").join(lib));
    }

    paths.push(PathBuf::from(layout.runtime_dir).join(lib));

    if let Some(home) = &env.home_dir {
        paths.push(home.join(".tarqeem").join("lib").join(lib));
    }

    #[cfg(windows)]
    {
        if let Some(local_app_data) = &env.local_app_data {
            paths.push(local_app_data.join("Tarqeem/lib").join(lib));
        }
        paths.push(PathBuf::from("C:/Program Files/Tarqeem/lib").join(lib));
    }
    #[cfg(not(windows))]
    {
        paths.push(PathBuf::from("/usr/local/lib/tarqeem").join(lib));
        paths.push(PathBuf::from("/opt/homebrew/lib/tarqeem").join(lib));
        paths.push(PathBuf::from("/usr/lib/tarqeem").join(lib));
    }

    // Layouts overlap — an installed tree has `TARQEEM_HOME/lib` and the exe's
    // `../lib` naming one directory — and a repeat reads as a mistake in the
    // not-found listing. Keep the first occurrence, which is the higher rank.
    let mut seen = std::collections::HashSet::new();
    paths.retain(|path| seen.insert(path.clone()));

    paths
}

/// The runtime archive, or every path probed for it so a failure can say where
/// it looked.
pub fn locate_runtime() -> Result<PathBuf, Vec<PathBuf>> {
    locate(&NATIVE_LAYOUT)
}

fn locate(layout: &RuntimeLayout) -> Result<PathBuf, Vec<PathBuf>> {
    let candidates = runtime_candidates(&RuntimeEnv::current(), layout);
    match candidates.iter().find(|path| path.exists()) {
        Some(found) => Ok(found.clone()),
        None => Err(candidates),
    }
}

/// Finds the Tarqeem runtime library (`libtrq.a`, or `trq.lib` on Windows).
pub fn find_runtime() -> Option<PathBuf> {
    locate_runtime().ok()
}

/// Finds the WebAssembly runtime library.
pub fn find_wasm_runtime() -> Option<PathBuf> {
    locate(&WASM_LAYOUT).ok()
}

/// Renders the "no runtime anywhere" refusal, naming what was searched.
///
/// The archive comes from the separate `tarqeem-runtime` crate, which is not a
/// default workspace member, so "never built" is the overwhelmingly likely
/// cause — hence leading with the command that builds it.
fn runtime_not_found(searched: &[PathBuf]) -> LinkerError {
    let profile_flag = if cfg!(debug_assertions) {
        ""
    } else {
        " --release"
    };
    let listing = searched
        .iter()
        .map(|path| format!("  {}", path.display()))
        .collect::<Vec<_>>()
        .join("\n");

    LinkerError::with_code(
        format!(
            "لم يتم العثور على مكتبة وقت التشغيل {RUNTIME_LIB} اللازمة للربط الأصلي.\n\
             Could not find the runtime library required to link native binaries.\n\
             المواضع التي بُحث فيها / searched:\n{listing}\n\
             ابنِها بـ / build it with: cargo build -p tarqeem-runtime{profile_flag}"
        ),
        ERR_RUNTIME_NOT_FOUND.to_string(),
    )
}

fn find_program(name: &str) -> Option<PathBuf> {
    if Command::new(name).arg("--version").output().is_ok() {
        return Some(PathBuf::from(name));
    }

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

fn find_linker(target: &Target) -> Option<PathBuf> {
    if target.triple.os == "darwin" {
        find_program("ld")
    } else {
        find_program("ld.lld").or_else(|| find_program("ld"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitType {
    LlvmIr,
    Assembly,
    Object,
    Executable,
    /// WebAssembly binary (.wasm)
    Wasm,
    /// WebAssembly with JavaScript bindings (.wasm + .js)
    WasmModule,
}

impl EmitType {
    pub fn extension(&self) -> &'static str {
        match self {
            EmitType::LlvmIr => "ll",
            EmitType::Assembly => "s",
            EmitType::Object => "o",
            EmitType::Executable => "",
            EmitType::Wasm => "wasm",
            EmitType::WasmModule => "wasm",
        }
    }

    pub fn is_wasm(&self) -> bool {
        matches!(self, EmitType::Wasm | EmitType::WasmModule)
    }
}

/// Generate JavaScript bindings for Tarqeem WASM module
///
/// This generates a JavaScript file that provides:
/// - Memory management helpers
/// - String conversion (Arabic UTF-8 support)
/// - I/O bridging (print, input)
/// - Module loading
fn generate_tarqeem_js_bindings(wasm_filename: &str, config: &WasmConfig) -> String {
    format!(
        r#"/**
 * Tarqeem WebAssembly Runtime
 * روابط ترقيم لـ WebAssembly
 *
 * Generated automatically by the Tarqeem compiler.
 * تم توليدها تلقائياً بواسطة مترجم ترقيم.
 */

const TarqeemRuntime = {{
    memory: null,
    instance: null,
    textDecoder: new TextDecoder('utf-8'),
    textEncoder: new TextEncoder(),

    // I/O callbacks (can be overridden)
    onPrint: (text) => console.log(text),
    onInput: () => prompt('إدخال / Input:') || '',

    /**
     * Load and initialize a Tarqeem WASM module
     * @param {{string|URL}} [wasmPath] - Path to the .wasm file
     * @returns {{Promise<TarqeemRuntime>}}
     */
    async load(wasmPath = '{wasm_filename}') {{
        const imports = {{
            env: {{
                // Memory (provided to WASM)
                memory: new WebAssembly.Memory({{
                    initial: {initial_pages},
                    maximum: {max_pages}
                }}),

                // I/O imports
                __trq_js_print: (ptr, len) => {{
                    const view = new Uint8Array(this.memory.buffer, ptr, len);
                    const text = this.textDecoder.decode(view);
                    this.onPrint(text);
                }},

                __trq_js_print_int: (value) => {{
                    this.onPrint(String(value));
                }},

                __trq_js_print_float: (value) => {{
                    this.onPrint(String(value));
                }},

                __trq_js_print_bool: (value) => {{
                    this.onPrint(value ? 'صحيح' : 'خطأ');
                }},

                __trq_js_print_newline: () => {{
                    this.onPrint('\n');
                }},

                __trq_js_input: (bufPtr, maxLen) => {{
                    const input = this.onInput() || '';
                    const bytes = this.textEncoder.encode(input);
                    const view = new Uint8Array(this.memory.buffer, bufPtr, maxLen);
                    const len = Math.min(bytes.length, maxLen - 1);
                    view.set(bytes.slice(0, len));
                    view[len] = 0; // Null terminate
                    return len;
                }},

                // Math imports (fallback for missing builtins)
                __trq_js_sqrt: Math.sqrt,
                __trq_js_pow: Math.pow,
                __trq_js_sin: Math.sin,
                __trq_js_cos: Math.cos,
                __trq_js_tan: Math.tan,
                __trq_js_floor: Math.floor,
                __trq_js_ceil: Math.ceil,
                __trq_js_round: Math.round,
                __trq_js_abs: Math.abs,
                __trq_js_log: Math.log,
                __trq_js_exp: Math.exp,

                // Time
                __trq_js_now: () => Date.now(),
                __trq_js_performance_now: () => performance.now(),
            }}
        }};

        this.memory = imports.env.memory;

        try {{
            const response = await fetch(wasmPath);
            const bytes = await response.arrayBuffer();
            const module = await WebAssembly.compile(bytes);
            this.instance = await WebAssembly.instantiate(module, imports);

            // Update memory reference if WASM exports its own
            if (this.instance.exports.memory) {{
                this.memory = this.instance.exports.memory;
            }}

            return this;
        }} catch (error) {{
            throw new Error(`Failed to load Tarqeem module: ${{error.message}}`);
        }}
    }},

    /**
     * Call an exported Tarqeem function
     * @param {{string}} funcName - Function name (Arabic or mangled)
     * @param {{...any}} args - Arguments
     * @returns {{any}} Return value
     */
    call(funcName, ...args) {{
        const func = this.instance.exports[funcName];
        if (!func) {{
            throw new Error(`Function not found: ${{funcName}}`);
        }}
        return func(...args);
    }},

    /**
     * Run the main entry point
     * @returns {{any}} Return value from main
     */
    run() {{
        if (this.instance.exports.__main__) {{
            return this.instance.exports.__main__();
        }}
        if (this.instance.exports.main) {{
            return this.instance.exports.main();
        }}
        throw new Error('No main function found / لم يتم العثور على دالة رئيسية');
    }},

    /**
     * Read a string from WASM memory
     * @param {{number}} ptr - Pointer to TrqString structure
     * @returns {{string}} JavaScript string
     */
    readString(ptr) {{
        const view = new DataView(this.memory.buffer);
        const len = view.getInt32(ptr, true); // Little endian
        const dataPtr = view.getInt32(ptr + 8, true);
        const bytes = new Uint8Array(this.memory.buffer, dataPtr, len);
        return this.textDecoder.decode(bytes);
    }},

    /**
     * Write a string to WASM memory (requires allocation)
     * @param {{string}} str - JavaScript string
     * @returns {{number}} Pointer to allocated string
     */
    writeString(str) {{
        const bytes = this.textEncoder.encode(str);
        const allocFunc = this.instance.exports.trq_alloc ||
                         this.instance.exports.__trq_alloc;
        if (!allocFunc) {{
            throw new Error('String allocation not available');
        }}

        // Allocate TrqString structure + data
        const structSize = 24; // len(8) + cap(8) + data_ptr(8) for 64-bit, adjust for 32-bit
        const totalSize = structSize + bytes.length + 1;
        const ptr = allocFunc(totalSize);

        const view = new DataView(this.memory.buffer);
        const dataPtr = ptr + structSize;

        // Write TrqString structure
        view.setInt32(ptr, bytes.length, true);     // len
        view.setInt32(ptr + 4, 0, true);            // padding
        view.setInt32(ptr + 8, bytes.length, true); // cap
        view.setInt32(ptr + 12, 0, true);           // padding
        view.setInt32(ptr + 16, dataPtr, true);     // data pointer

        // Write string data
        const dataView = new Uint8Array(this.memory.buffer, dataPtr, bytes.length + 1);
        dataView.set(bytes);
        dataView[bytes.length] = 0; // Null terminate

        return ptr;
    }},

    /**
     * Get list of exported functions
     * @returns {{string[]}} Array of function names
     */
    getExports() {{
        return Object.keys(this.instance.exports).filter(
            name => typeof this.instance.exports[name] === 'function'
        );
    }}
}};

// CommonJS export
if (typeof module !== 'undefined' && module.exports) {{
    module.exports = TarqeemRuntime;
}}

// ES Module export
export default TarqeemRuntime;
export {{ TarqeemRuntime }};
"#,
        wasm_filename = wasm_filename,
        initial_pages = config.memory_pages,
        max_pages = if config.max_memory_pages > 0 {
            config.max_memory_pages.to_string()
        } else {
            "undefined".to_string()
        },
    )
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
        assert_eq!(EmitType::Wasm.extension(), "wasm");
        assert_eq!(EmitType::WasmModule.extension(), "wasm");
    }

    #[test]
    fn test_emit_type_is_wasm() {
        assert!(!EmitType::LlvmIr.is_wasm());
        assert!(!EmitType::Executable.is_wasm());
        assert!(EmitType::Wasm.is_wasm());
        assert!(EmitType::WasmModule.is_wasm());
    }

    #[test]
    fn test_wasm_config_default() {
        let config = WasmConfig::new();
        assert!(!config.js_bindings);
        assert!(!config.export_all);
        assert_eq!(config.memory_pages, 16);
        assert!(config.exports.contains(&"__main__".to_string()));
    }

    #[test]
    fn test_wasm_config_builder() {
        let config = WasmConfig::new()
            .with_js_bindings()
            .with_export_all()
            .with_memory_pages(32);

        assert!(config.js_bindings);
        assert!(config.export_all);
        assert_eq!(config.memory_pages, 32);
    }

    #[test]
    fn test_js_bindings_generation() {
        let config = WasmConfig::new();
        let bindings = generate_tarqeem_js_bindings("test.wasm", &config);

        // Check that the bindings contain expected content
        assert!(bindings.contains("TarqeemRuntime"));
        assert!(bindings.contains("test.wasm"));
        assert!(bindings.contains("initial: 16"));
        assert!(bindings.contains("__trq_js_print"));
        assert!(bindings.contains("صحيح")); // Arabic for true
    }

    #[test]
    fn test_linker_wasm_availability() {
        let target = Target::wasm32();
        let linker = Linker::new(target);
        // This tests the method exists; actual availability depends on system
        let _ = linker.is_wasm_available();
    }

    /// A fully populated environment, so each test can knock out just the one
    /// input it is about.
    fn sample_env() -> RuntimeEnv {
        RuntimeEnv {
            explicit: Some(PathBuf::from("/explicit/libtrq.a")),
            baked: Some(PathBuf::from("/checkout/target/release/libtrq.a")),
            exe_dir: Some(PathBuf::from("/checkout/target/release")),
            manifest_dir: Some(PathBuf::from("/checkout")),
            tarqeem_home: Some(PathBuf::from("/home/u/.tarqeem")),
            home_dir: Some(PathBuf::from("/home/u")),
            local_app_data: None,
        }
    }

    fn native_candidates(env: &RuntimeEnv) -> Vec<PathBuf> {
        runtime_candidates(env, &NATIVE_LAYOUT)
    }

    /// Index of the first candidate whose path ends with `suffix`.
    fn rank(paths: &[PathBuf], suffix: &str) -> usize {
        paths
            .iter()
            .position(|p| p.to_string_lossy().replace('\\', "/").ends_with(suffix))
            .unwrap_or_else(|| panic!("{suffix} missing from {paths:?}"))
    }

    /// The regression this whole ordering exists for (#285): an installed
    /// archive must never outrank one sitting beside the compiler.
    #[test]
    fn dev_build_outranks_every_installed_location() {
        let paths = native_candidates(&sample_env());
        let sibling = rank(&paths, "/checkout/target/release/libtrq.a");

        for installed in [
            "/home/u/.tarqeem/lib/libtrq.a",
            "/usr/local/lib/tarqeem/libtrq.a",
        ] {
            assert!(
                sibling < rank(&paths, installed),
                "{installed} outranked the archive beside the executable"
            );
        }
        assert!(sibling < rank(&paths, ".tarqeem/lib/libtrq.a"));
    }

    #[test]
    fn explicit_override_outranks_everything() {
        let paths = native_candidates(&sample_env());
        assert_eq!(paths.first().unwrap(), &PathBuf::from("/explicit/libtrq.a"));
    }

    /// `TARQEEM_HOME` used to be probed first, which is precisely how a stale
    /// installed archive shadowed every rebuild.
    #[test]
    fn tarqeem_home_does_not_outrank_the_build_tree() {
        let paths = native_candidates(&sample_env());
        let home = rank(&paths, "/home/u/.tarqeem/lib/libtrq.a");
        assert!(rank(&paths, "/checkout/target/release/libtrq.a") < home);
        assert!(rank(&paths, "/explicit/libtrq.a") < home);
    }

    /// An installed binary has no sibling archive; `../lib` is what resolves,
    /// and it still has to beat `~/.tarqeem` and the system paths.
    #[test]
    fn installed_layout_resolves_beside_its_own_executable() {
        let env = RuntimeEnv {
            exe_dir: Some(PathBuf::from("/opt/tarqeem/bin")),
            home_dir: Some(PathBuf::from("/home/u")),
            ..Default::default()
        };
        let paths = native_candidates(&env);
        assert!(
            rank(&paths, "/opt/tarqeem/lib/libtrq.a")
                < rank(&paths, "/home/u/.tarqeem/lib/libtrq.a")
        );
    }

    /// Probing `release` before `debug` regardless of build would hand a debug
    /// compiler the release runtime.
    #[test]
    fn target_dir_follows_this_binarys_profile() {
        let env = RuntimeEnv {
            manifest_dir: Some(PathBuf::from("/checkout")),
            ..Default::default()
        };
        let (matching, other) = if cfg!(debug_assertions) {
            (
                "/checkout/target/debug/libtrq.a",
                "/checkout/target/release/libtrq.a",
            )
        } else {
            (
                "/checkout/target/release/libtrq.a",
                "/checkout/target/debug/libtrq.a",
            )
        };
        let paths = native_candidates(&env);
        rank(&paths, matching);
        assert!(
            !paths.iter().any(|p| p.to_string_lossy() == other),
            "the other profile's runtime must not be a candidate"
        );
    }

    /// Locations the CLI copy searched before the merge; dropping any of them
    /// would break a layout someone relies on — CI's compiled-examples job
    /// depends on the CWD entry.
    #[test]
    fn merge_kept_every_previously_searched_location() {
        let paths = native_candidates(&sample_env());
        for kept in [
            "/checkout/target/release/runtime/libtrq.a",
            "/checkout/target/runtime/libtrq.a",
            "/checkout/target/lib/libtrq.a",
            "runtime/libtrq.a",
            "/home/u/.tarqeem/lib/libtrq.a",
        ] {
            rank(&paths, kept);
        }
    }

    /// Both `TARQEEM_RUNTIME_PATH` reads — the exported one and the one
    /// `build.rs` bakes in — name the *native* archive. Handing either to a wasm
    /// link is worse than linking without a runtime at all, so no wasm candidate
    /// may be anything but `libtrq_wasm.a`.
    #[test]
    fn wasm_search_ignores_the_native_override() {
        let paths = runtime_candidates(&sample_env(), &WASM_LAYOUT);
        assert!(
            paths
                .iter()
                .all(|p| p.file_name().is_some_and(|n| n == WASM_RUNTIME_LIB)),
            "a native archive reached the wasm search: {paths:?}"
        );
        rank(&paths, "runtime/wasm/libtrq_wasm.a");
    }

    #[test]
    fn missing_runtime_error_names_the_code_and_the_fix() {
        let err = runtime_not_found(&[PathBuf::from("/a/libtrq.a")]);
        assert_eq!(err.code.as_deref(), Some("ت٠١٠٢"));
        assert!(err.message.contains("/a/libtrq.a"));
        assert!(err.message.contains("cargo build -p tarqeem-runtime"));
    }
}
