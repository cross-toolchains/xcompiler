//! `xcompiler` unified CLI entry point.

use std::process::ExitCode;

use clap::{Parser, Subcommand};

use xcompiler_core::print_table;

/// Unified portable compiler manager (`xllvm` + `xgcc`) for zero-Docker cross-compilation.
#[derive(Debug, Parser)]
#[command(name = "xcompiler", version, about, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// List available compiler backends (`xllvm` & `xgcc`).
    List,
    /// Inspect host and managed compiler availability.
    Doctor,
    /// Install a portable LLVM or GCC toolchain.
    Install {
        #[command(subcommand)]
        backend: InstallBackend,
    },
}

#[derive(Debug, Subcommand)]
enum InstallBackend {
    /// Install portable slim LLVM (`clang` + `lld` + binutils).
    Llvm {
        /// LLVM version to install.
        #[arg(default_value = xllvm::DEFAULT_LLVM_VERSION)]
        version: String,
        /// Upstream source (`slim` or `kernel-org`).
        #[arg(long, default_value = "slim")]
        source: String,
    },
    /// Install portable cross-GCC (`x86_64-pc-windows-gnu`, `arm-none-eabi`, etc.).
    Gcc {
        /// Target triple.
        #[arg(long)]
        target: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::List => {
            let rows = vec![
                vec![
                    "xllvm".to_string(),
                    "LLVM (clang + lld)".to_string(),
                    "All OS targets (Linux / Win MSVC & gnullvm / macOS / iOS / Android / OHOS / Wasm)".to_string(),
                    "cross-toolchains/llvm-slim".to_string(),
                ],
                vec![
                    "xgcc".to_string(),
                    "GCC (cross)".to_string(),
                    "Windows MinGW (`*-pc-windows-gnu`) & Embedded bare-metal (`arm-none-eabi`, `aarch64-none-elf`, `riscv-none-elf`)".to_string(),
                    "xPack / ARM Official".to_string(),
                ],
            ];
            print_table(&["TOOL", "ENGINE", "TARGETS", "UPSTREAM"], rows);
            ExitCode::SUCCESS
        }
        Commands::Doctor => {
            let llvm_cache = xllvm::default_cache_dir();
            let gcc_cache = xgcc::default_cache_dir();
            let mut rows = Vec::new();
            match xllvm::detect_host_llvm() {
                Some(tc) => rows.push(vec![
                    "Host LLVM".to_string(),
                    "ready".to_string(),
                    tc.clang.display().to_string(),
                    tc.version,
                ]),
                None => rows.push(vec![
                    "Host LLVM".to_string(),
                    "missing (or no lld)".to_string(),
                    "-".to_string(),
                    "-".to_string(),
                ]),
            }
            match xllvm::find_managed_llvm(&llvm_cache, xllvm::DEFAULT_LLVM_VERSION) {
                Some(tc) => rows.push(vec![
                    format!("xllvm ({})", xllvm::DEFAULT_LLVM_VERSION),
                    "installed".to_string(),
                    tc.clang.display().to_string(),
                    tc.version,
                ]),
                None => rows.push(vec![
                    format!("xllvm ({})", xllvm::DEFAULT_LLVM_VERSION),
                    "not installed".to_string(),
                    llvm_cache.display().to_string(),
                    "run `xcompiler install llvm`".to_string(),
                ]),
            }
            for spec in xgcc::SUPPORTED_TARGETS {
                if let Some(tc) = xgcc::find_installed_gcc(&gcc_cache, *spec) {
                    rows.push(vec![
                        format!("xgcc ({})", spec.target),
                        "installed".to_string(),
                        tc.gcc.display().to_string(),
                        tc.version,
                    ]);
                }
            }
            print_table(&["COMPILER", "STATUS", "PATH", "DETAILS"], rows);
            ExitCode::SUCCESS
        }
        Commands::Install { backend } => match backend {
            InstallBackend::Llvm { version, source } => {
                let src = match xllvm::LlvmSource::parse(&source) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("[xcompiler] error: {e}");
                        return ExitCode::FAILURE;
                    }
                };
                let opts = xllvm::InstallOptions {
                    version,
                    source: src,
                    ..xllvm::InstallOptions::default()
                };
                match xllvm::install_llvm(&opts) {
                    Ok(_) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("[xcompiler] error: {e}");
                        ExitCode::FAILURE
                    }
                }
            }
            InstallBackend::Gcc { target } => {
                match xgcc::install_gcc(&target, &xgcc::default_cache_dir()) {
                    Ok(_) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("[xcompiler] error: {e}");
                        ExitCode::FAILURE
                    }
                }
            }
        },
    }
}
