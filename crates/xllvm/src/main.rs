//! `xllvm` CLI entry point.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

use xcompiler_core::{host_triple, print_table};
use xllvm::{
    DEFAULT_LLVM_VERSION, InstallOptions, KNOWN_LLVM_VERSIONS, LlvmSource, default_cache_dir,
    detect_host_llvm, find_managed_llvm, install_llvm, llvm_download_url, resolve_or_install,
};

/// Portable slim LLVM (`clang` + `lld` + binutils) manager.
#[derive(Debug, Parser)]
#[command(name = "xllvm", version, about, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// List known LLVM versions and download URLs for the current host.
    List {
        /// Upstream source (`slim` or `kernel-org`).
        #[arg(long, default_value = "slim")]
        source: String,
    },
    /// Download and install a portable slim LLVM toolchain.
    Install(InstallArgs),
    /// Print the resolved `clang` binary path (prefers healthy host LLVM unless `--managed`).
    Path(ResolveArgs),
    /// Print shell environment exports (`CC`, `CXX`, `AR`, `PATH`).
    Env(ResolveArgs),
    /// Inspect host and managed LLVM availability.
    Doctor,
}

#[derive(Debug, Args)]
struct InstallArgs {
    /// LLVM version to install.
    #[arg(default_value = DEFAULT_LLVM_VERSION)]
    version: String,

    /// Upstream source (`slim` or `kernel-org`).
    #[arg(long, default_value = "slim")]
    source: String,

    /// Cache directory (default: `~/.cache/xllvm`).
    #[arg(long)]
    cache_dir: Option<PathBuf>,

    /// Print verbose output.
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Args)]
struct ResolveArgs {
    /// LLVM version when using managed toolchain.
    #[arg(long, default_value = DEFAULT_LLVM_VERSION)]
    version: String,

    /// Force using managed `xllvm` instead of host LLVM.
    #[arg(long)]
    managed: bool,

    /// Cache directory (default: `~/.cache/xllvm`).
    #[arg(long)]
    cache_dir: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::List { source } => {
            let src = match LlvmSource::parse(&source) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[xllvm] error: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let host = match host_triple() {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("[xllvm] error: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let rows: Vec<Vec<String>> = KNOWN_LLVM_VERSIONS
                .iter()
                .filter_map(|v| {
                    llvm_download_url(v, src, host)
                        .ok()
                        .map(|(url, _)| vec![v.to_string(), host.to_string(), url])
                })
                .collect();
            print_table(&["VERSION", "HOST", "URL"], rows);
            ExitCode::SUCCESS
        }
        Commands::Install(args) => {
            let source = match LlvmSource::parse(&args.source) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[xllvm] error: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let opts = InstallOptions {
                version: args.version,
                source,
                cache_dir: args.cache_dir.unwrap_or_else(default_cache_dir),
                verbose: args.verbose,
            };
            match install_llvm(&opts) {
                Ok(tc) => {
                    if args.verbose {
                        eprintln!("[xllvm] ready: {} ({})", tc.clang.display(), tc.version);
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("[xllvm] error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Commands::Path(args) => {
            let opts = InstallOptions {
                version: args.version,
                cache_dir: args.cache_dir.unwrap_or_else(default_cache_dir),
                ..InstallOptions::default()
            };
            match resolve_or_install(&opts, args.managed) {
                Ok(tc) => {
                    println!("{}", tc.clang.display());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("[xllvm] error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Commands::Env(args) => {
            let opts = InstallOptions {
                version: args.version,
                cache_dir: args.cache_dir.unwrap_or_else(default_cache_dir),
                ..InstallOptions::default()
            };
            match resolve_or_install(&opts, args.managed) {
                Ok(tc) => {
                    let bin_dir = tc.clang.parent().unwrap_or(&tc.root);
                    println!("export CC=\"{}\"", tc.clang.display());
                    println!("export CXX=\"{}\"", tc.clangxx.display());
                    if let Some(ar) = &tc.ar {
                        println!("export AR=\"{}\"", ar.display());
                    }
                    println!("export PATH=\"{}:$PATH\"", bin_dir.display());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("[xllvm] error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Commands::Doctor => {
            let cache_dir = default_cache_dir();
            let mut rows = Vec::new();
            match detect_host_llvm() {
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
            match find_managed_llvm(&cache_dir, DEFAULT_LLVM_VERSION) {
                Some(tc) => rows.push(vec![
                    format!("Managed ({DEFAULT_LLVM_VERSION})"),
                    "installed".to_string(),
                    tc.clang.display().to_string(),
                    tc.version,
                ]),
                None => rows.push(vec![
                    format!("Managed ({DEFAULT_LLVM_VERSION})"),
                    "not installed".to_string(),
                    cache_dir.display().to_string(),
                    "run `xllvm install`".to_string(),
                ]),
            }
            print_table(&["TOOLCHAIN", "STATUS", "PATH", "DETAILS"], rows);
            ExitCode::SUCCESS
        }
    }
}
