//! `xgcc` CLI entry point.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

use xcompiler_core::print_table;
use xgcc::{SUPPORTED_TARGETS, default_cache_dir, find_installed_gcc, find_spec, install_gcc};

/// Portable prebuilt cross-GCC toolchain manager (MinGW-w64 & embedded bare-metal).
#[derive(Debug, Parser)]
#[command(name = "xgcc", version, about, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// List supported cross-GCC targets and upstream providers.
    List,
    /// Download and install a cross-GCC toolchain for `--target`.
    Install(TargetArgs),
    /// Print the installed `*-gcc` binary path for `--target`.
    Path(TargetArgs),
}

#[derive(Debug, Args)]
struct TargetArgs {
    /// Target triple (e.g. `x86_64-pc-windows-gnu`, `arm-none-eabi`, `riscv-none-elf`).
    #[arg(long)]
    target: String,

    /// Cache directory (default: `~/.cache/xgcc`).
    #[arg(long)]
    cache_dir: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::List => {
            let rows = SUPPORTED_TARGETS
                .iter()
                .map(|s| {
                    vec![
                        s.target.to_string(),
                        s.version.to_string(),
                        s.provider.to_string(),
                        s.description.to_string(),
                    ]
                })
                .collect();
            print_table(&["TARGET", "VERSION", "PROVIDER", "DESCRIPTION"], rows);
            ExitCode::SUCCESS
        }
        Commands::Install(args) => {
            let cache_dir = args.cache_dir.unwrap_or_else(default_cache_dir);
            match install_gcc(&args.target, &cache_dir) {
                Ok(tc) => {
                    eprintln!("[xgcc] ready: {}", tc.gcc.display());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("[xgcc] error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Commands::Path(args) => {
            let cache_dir = args.cache_dir.unwrap_or_else(default_cache_dir);
            let spec = match find_spec(&args.target) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[xgcc] error: {e}");
                    return ExitCode::FAILURE;
                }
            };
            match find_installed_gcc(&cache_dir, spec) {
                Some(tc) => {
                    println!("{}", tc.gcc.display());
                    ExitCode::SUCCESS
                }
                None => {
                    eprintln!(
                        "[xgcc] error: `{}` is not installed (run `xgcc install --target {}`)",
                        spec.target, spec.target
                    );
                    ExitCode::FAILURE
                }
            }
        }
    }
}
