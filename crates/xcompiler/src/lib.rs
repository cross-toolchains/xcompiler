//! `xcompiler` — Unified portable compiler manager (`xllvm` + `xgcc`) for
//! zero-Docker cross-compilation.

pub use xcompiler_core;
pub use xgcc;
pub use xllvm;

use std::path::Path;

/// A resolved host compiler toolchain ready to be paired with `xsysroot` in `xforge`.
#[derive(Debug, Clone)]
pub enum ResolvedCompiler {
    Llvm(xllvm::LlvmToolchain),
    Gcc(xgcc::GccToolchain),
}

impl ResolvedCompiler {
    pub fn c_compiler(&self) -> &Path {
        match self {
            Self::Llvm(tc) => &tc.clang,
            Self::Gcc(tc) => &tc.gcc,
        }
    }

    pub fn cxx_compiler(&self) -> &Path {
        match self {
            Self::Llvm(tc) => &tc.clangxx,
            Self::Gcc(tc) => &tc.gxx,
        }
    }
}
