//! Target Configuration
//!
//! This module defines target machine configurations for code generation.

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
            // Fallback to x86_64-linux
            Self::new("x86_64", "unknown", "linux", Some("gnu"))
        }
    }

    pub fn is_64bit(&self) -> bool {
        matches!(self.arch.as_str(), "x86_64" | "aarch64")
    }

    pub fn pointer_size(&self) -> u32 {
        if self.is_64bit() {
            8
        } else {
            4
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

    pub fn to_llvm_string(&self) -> String {
        // Format: e-m:e-p:64:64-i1:8:8-i8:8:8-i16:16:16-i32:32:32-i64:64:64-f64:64:64-n8:16:32:64-S128
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

    pub fn llvm_triple(&self) -> String {
        self.triple.to_string()
    }

    pub fn llvm_data_layout(&self) -> String {
        self.data_layout.to_llvm_string()
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
}
