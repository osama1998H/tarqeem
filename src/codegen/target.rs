//! Target Configuration
//!
//! This module defines target machine configurations for code generation.
//! Supports native targets (x86_64, aarch64) and WebAssembly targets.

use std::fmt;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetTriple {
    pub arch: String,
    pub vendor: String,
    pub os: String,
    pub env: Option<String>,
}

impl TargetTriple {
    pub fn new(arch: &str, vendor: &str, os: &str, env: Option<&str>) -> Self {
        Self {
            arch: arch.to_string(),
            vendor: vendor.to_string(),
            os: os.to_string(),
            env: env.map(|s| s.to_string()),
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() < 3 {
            // Handle special case for wasm32-unknown-unknown which is valid
            if parts.len() == 2 && parts[0] == "wasm32" {
                return Some(Self {
                    arch: "wasm32".to_string(),
                    vendor: parts[1].to_string(),
                    os: "unknown".to_string(),
                    env: None,
                });
            }
            return None;
        }

        Some(Self {
            arch: parts[0].to_string(),
            vendor: parts[1].to_string(),
            os: parts[2].to_string(),
            env: parts.get(3).map(|s| s.to_string()),
        })
    }

    pub fn native() -> Self {
        #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
        {
            Self::new("x86_64", "unknown", "linux", Some("gnu"))
        }

        #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
        {
            Self::new("x86_64", "apple", "darwin", None)
        }

        #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
        {
            Self::new("aarch64", "unknown", "linux", Some("gnu"))
        }

        #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
        {
            Self::new("aarch64", "apple", "darwin", None)
        }

        #[cfg(not(any(
            all(target_arch = "x86_64", target_os = "linux"),
            all(target_arch = "x86_64", target_os = "macos"),
            all(target_arch = "aarch64", target_os = "linux"),
            all(target_arch = "aarch64", target_os = "macos"),
        )))]
        {
            Self::new("x86_64", "unknown", "linux", Some("gnu"))
        }
    }

    /// WebAssembly target for browser deployment (via JavaScript bindings)
    pub fn wasm32_unknown() -> Self {
        Self::new("wasm32", "unknown", "unknown", None)
    }

    /// WebAssembly target with WASI support for standalone execution
    pub fn wasm32_wasi() -> Self {
        Self::new("wasm32", "wasi", "wasi", None)
    }

    /// WebAssembly target with WASI Preview 1 (explicit)
    pub fn wasm32_wasip1() -> Self {
        Self::new("wasm32", "wasip1", "wasi", None)
    }

    /// Check if this is a WebAssembly target
    pub fn is_wasm(&self) -> bool {
        self.arch.starts_with("wasm")
    }

    /// Check if this is a WASI target (standalone WASM with system interface)
    pub fn is_wasi(&self) -> bool {
        self.is_wasm() && (self.vendor.contains("wasi") || self.os == "wasi")
    }

    pub fn is_64bit(&self) -> bool {
        matches!(self.arch.as_str(), "x86_64" | "aarch64" | "wasm64")
    }

    pub fn is_32bit(&self) -> bool {
        matches!(self.arch.as_str(), "wasm32" | "i686" | "i386")
    }

    pub fn pointer_size(&self) -> u32 {
        if self.is_64bit() {
            8
        } else if self.is_32bit() {
            4
        } else {
            8 // Default to 64-bit
        }
    }

    pub fn pointer_bits(&self) -> u32 {
        self.pointer_size() * 8
    }
}

impl fmt::Display for TargetTriple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref env) = self.env {
            write!(f, "{}-{}-{}-{}", self.arch, self.vendor, self.os, env)
        } else {
            write!(f, "{}-{}-{}", self.arch, self.vendor, self.os)
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataLayout {
    pub endianness: char,
    pub pointer_bits: u32,
    pub stack_alignment: u32,
    pub native_integers: Vec<u32>,
}

impl DataLayout {
    pub fn x86_64() -> Self {
        Self {
            endianness: 'e',
            pointer_bits: 64,
            stack_alignment: 128,
            native_integers: vec![8, 16, 32, 64],
        }
    }

    pub fn aarch64() -> Self {
        Self {
            endianness: 'e',
            pointer_bits: 64,
            stack_alignment: 128,
            native_integers: vec![8, 16, 32, 64],
        }
    }

    /// WebAssembly 32-bit data layout
    pub fn wasm32() -> Self {
        Self {
            endianness: 'e',
            pointer_bits: 32,
            stack_alignment: 128,
            native_integers: vec![32, 64],
        }
    }

    pub fn to_llvm_string(&self) -> String {
        let native: Vec<String> = self.native_integers.iter().map(|i| i.to_string()).collect();
        format!(
            "{}-m:e-p:{}:{}-i1:8:8-i8:8:8-i16:16:16-i32:32:32-i64:64:64-f64:64:64-n{}-S{}",
            self.endianness,
            self.pointer_bits,
            self.pointer_bits,
            native.join(":"),
            self.stack_alignment
        )
    }

    /// WebAssembly-specific data layout string
    pub fn to_llvm_string_wasm(&self) -> String {
        // Standard WASM32 data layout used by LLVM
        "e-m:e-p:32:32-i64:64-n32:64-S128-ni:1:10:20".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct Target {
    pub triple: TargetTriple,
    pub data_layout: DataLayout,
    pub cpu: Option<String>,
    pub features: Option<String>,
}

impl Target {
    pub fn from_triple(triple: TargetTriple) -> Self {
        let data_layout = match triple.arch.as_str() {
            "x86_64" => DataLayout::x86_64(),
            "aarch64" => DataLayout::aarch64(),
            "wasm32" => DataLayout::wasm32(),
            _ => DataLayout::x86_64(), // Default
        };

        Self {
            triple,
            data_layout,
            cpu: None,
            features: None,
        }
    }

    pub fn native() -> Self {
        Self::from_triple(TargetTriple::native())
    }

    /// WebAssembly target for browser deployment
    pub fn wasm32() -> Self {
        Self::from_triple(TargetTriple::wasm32_unknown())
    }

    /// WebAssembly target with WASI for standalone execution
    pub fn wasm32_wasi() -> Self {
        Self::from_triple(TargetTriple::wasm32_wasi())
    }

    /// Check if this is a WebAssembly target
    pub fn is_wasm(&self) -> bool {
        self.triple.is_wasm()
    }

    /// Check if this is a WASI target
    pub fn is_wasi(&self) -> bool {
        self.triple.is_wasi()
    }

    pub fn llvm_triple(&self) -> String {
        self.triple.to_string()
    }

    pub fn llvm_data_layout(&self) -> String {
        if self.triple.is_wasm() {
            self.data_layout.to_llvm_string_wasm()
        } else {
            self.data_layout.to_llvm_string()
        }
    }

    pub fn has_clang() -> bool {
        Command::new("clang")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn has_llc() -> bool {
        Command::new("llc")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Check if wasm-ld (WebAssembly linker) is available
    pub fn has_wasm_ld() -> bool {
        Command::new("wasm-ld")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Check if LLVM has WebAssembly target support
    pub fn has_wasm_support() -> bool {
        Command::new("llc")
            .args(["--version"])
            .output()
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout);
                // Check if wasm32 is in the registered targets
                stdout.contains("wasm") || Self::has_wasm_ld()
            })
            .unwrap_or(false)
    }
}

impl Default for Target {
    fn default() -> Self {
        Self::native()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_triple_parse() {
        let triple = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(triple.arch, "x86_64");
        assert_eq!(triple.vendor, "unknown");
        assert_eq!(triple.os, "linux");
        assert_eq!(triple.env, Some("gnu".to_string()));

        let triple = TargetTriple::parse("aarch64-apple-darwin").unwrap();
        assert_eq!(triple.arch, "aarch64");
        assert_eq!(triple.vendor, "apple");
        assert_eq!(triple.os, "darwin");
        assert_eq!(triple.env, None);
    }

    #[test]
    fn test_target_triple_display() {
        let triple = TargetTriple::new("x86_64", "unknown", "linux", Some("gnu"));
        assert_eq!(triple.to_string(), "x86_64-unknown-linux-gnu");

        let triple = TargetTriple::new("aarch64", "apple", "darwin", None);
        assert_eq!(triple.to_string(), "aarch64-apple-darwin");
    }

    #[test]
    fn test_data_layout() {
        let layout = DataLayout::x86_64();
        let s = layout.to_llvm_string();
        assert!(s.starts_with("e-"));
        assert!(s.contains("p:64:64"));
    }

    #[test]
    fn test_wasm32_target_triple() {
        let triple = TargetTriple::wasm32_unknown();
        assert_eq!(triple.arch, "wasm32");
        assert_eq!(triple.vendor, "unknown");
        assert_eq!(triple.os, "unknown");
        assert!(triple.is_wasm());
        assert!(!triple.is_wasi());
        assert!(triple.is_32bit());
        assert!(!triple.is_64bit());
        assert_eq!(triple.pointer_bits(), 32);
    }

    #[test]
    fn test_wasm32_wasi_target_triple() {
        let triple = TargetTriple::wasm32_wasi();
        assert_eq!(triple.arch, "wasm32");
        assert!(triple.is_wasm());
        assert!(triple.is_wasi());
        assert_eq!(triple.to_string(), "wasm32-wasi-wasi");
    }

    #[test]
    fn test_wasm32_target_parse() {
        let triple = TargetTriple::parse("wasm32-unknown-unknown").unwrap();
        assert!(triple.is_wasm());
        assert!(!triple.is_wasi());

        let triple = TargetTriple::parse("wasm32-wasi-wasi").unwrap();
        assert!(triple.is_wasm());
        assert!(triple.is_wasi());
    }

    #[test]
    fn test_wasm32_data_layout() {
        let layout = DataLayout::wasm32();
        assert_eq!(layout.pointer_bits, 32);

        let wasm_layout = layout.to_llvm_string_wasm();
        assert!(wasm_layout.contains("p:32:32"));
    }

    #[test]
    fn test_wasm_target() {
        let target = Target::wasm32();
        assert!(target.is_wasm());
        assert!(!target.is_wasi());
        assert_eq!(target.triple.pointer_bits(), 32);

        let target_wasi = Target::wasm32_wasi();
        assert!(target_wasi.is_wasm());
        assert!(target_wasi.is_wasi());
    }

    #[test]
    fn test_wasm_data_layout_string() {
        let target = Target::wasm32();
        let layout = target.llvm_data_layout();
        // WASM should use the specialized layout string
        assert!(layout.contains("p:32:32"));
    }
}
