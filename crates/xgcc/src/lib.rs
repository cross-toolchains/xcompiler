//! `xgcc` — Portable prebuilt cross-GCC toolchain manager for Windows MinGW-w64
//! and embedded bare-metal targets (`arm-none-eabi`, `aarch64-none-elf`, `riscv-none-elf`).

use std::fs;
use std::path::{Path, PathBuf};

use xcompiler_core::{
    default_tool_cache_dir, download_to_file, extract_archive, format_status, host_triple,
    make_bytes_bar,
};

#[derive(Debug, Clone, Copy)]
pub struct GccTargetSpec {
    pub target: &'static str,
    pub tool_prefix: &'static str,
    pub provider: &'static str,
    pub version: &'static str,
    pub description: &'static str,
}

pub const SUPPORTED_TARGETS: &[GccTargetSpec] = &[
    GccTargetSpec {
        target: "x86_64-pc-windows-gnu",
        tool_prefix: "x86_64-w64-mingw32",
        provider: "xpack-dev-tools/mingw-w64-gcc-xpack",
        version: "15.2.0-2",
        description: "Windows x86_64 MinGW-w64 GCC (Linux/macOS/Windows hosts)",
    },
    GccTargetSpec {
        target: "aarch64-pc-windows-gnu",
        tool_prefix: "aarch64-w64-mingw32",
        provider: "arm/gnu-toolchain-for-windows-on-arm64",
        version: "2026.11.21",
        description: "Windows ARM64 MinGW-w64 GCC (Linux/macOS/Windows hosts)",
    },
    GccTargetSpec {
        target: "arm-none-eabi",
        tool_prefix: "arm-none-eabi",
        provider: "xpack-dev-tools/arm-none-eabi-gcc-xpack",
        version: "14.2.1-1.1",
        description: "ARM32 Cortex-M/R bare-metal GCC (Linux/macOS/Windows hosts)",
    },
    GccTargetSpec {
        target: "aarch64-none-elf",
        tool_prefix: "aarch64-none-elf",
        provider: "xpack-dev-tools/aarch64-none-elf-gcc-xpack",
        version: "14.2.1-1.1",
        description: "ARM64 bare-metal GCC (Linux/macOS/Windows hosts)",
    },
    GccTargetSpec {
        target: "riscv-none-elf",
        tool_prefix: "riscv-none-elf",
        provider: "xpack-dev-tools/riscv-none-elf-gcc-xpack",
        version: "14.2.0-3",
        description: "RISC-V bare-metal GCC (Linux/macOS/Windows hosts)",
    },
];

#[derive(Debug, Clone)]
pub struct GccToolchain {
    pub root: PathBuf,
    pub gcc: PathBuf,
    pub gxx: PathBuf,
    pub ar: PathBuf,
    pub ranlib: Option<PathBuf>,
    pub objcopy: Option<PathBuf>,
    pub objdump: Option<PathBuf>,
    pub strip: Option<PathBuf>,
    pub size: Option<PathBuf>,
    pub target: String,
    pub version: String,
}

pub fn default_cache_dir() -> PathBuf {
    default_tool_cache_dir("xgcc")
}

/// Resolves a target string (Rust, LLVM, GNU, or Zig triple) into a supported [`GccTargetSpec`].
///
/// Uses [`xtarget::Target`] normalization so Rust bare-metal and MinGW triples such as
/// `thumbv7em-none-eabi`, `riscv32imac-unknown-none-elf`, `aarch64-unknown-none`, or
/// `x86_64-w64-mingw32` automatically route to their corresponding GCC multilib toolchain.
pub fn find_spec(target: &str) -> Result<GccTargetSpec, String> {
    if let Some(spec) = SUPPORTED_TARGETS
        .iter()
        .copied()
        .find(|s| s.target == target || s.tool_prefix == target)
    {
        return Ok(spec);
    }

    if let Ok(parsed) = xtarget::Target::parse_loose(target)
        && let Some(gnu) = parsed.gnu_triple()
        && let Some(spec) = SUPPORTED_TARGETS
            .iter()
            .copied()
            .find(|s| s.tool_prefix == gnu || s.target == parsed.as_str())
    {
        return Ok(spec);
    }

    let names: Vec<_> = SUPPORTED_TARGETS.iter().map(|s| s.target).collect();
    Err(format!(
        "unsupported xgcc target `{target}` (supported: {})",
        names.join(", ")
    ))
}

fn xpack_host_tag(host: &str) -> Result<(&'static str, &'static str), String> {
    match host {
        "x86_64-unknown-linux-gnu" => Ok(("linux-x64", "tar.gz")),
        "aarch64-unknown-linux-gnu" => Ok(("linux-arm64", "tar.gz")),
        "aarch64-apple-darwin" => Ok(("darwin-arm64", "tar.gz")),
        "x86_64-apple-darwin" => Ok(("darwin-x64", "tar.gz")),
        // Windows ARM64 runs x64 xPack toolchains transparently via Prism emulation.
        "x86_64-pc-windows-msvc" | "aarch64-pc-windows-msvc" => Ok(("win32-x64", "zip")),
        other => Err(format!("no xPack prebuilt archive for host `{other}`")),
    }
}

/// Builds the upstream download URL and filename for an `xgcc` target on `host`.
pub fn gcc_download_url(spec: GccTargetSpec, host: &str) -> Result<(String, String), String> {
    if spec.provider.starts_with("xpack-dev-tools/") {
        let (xpack_host, ext) = xpack_host_tag(host)?;
        let repo = spec.provider;
        let slug = repo
            .strip_prefix("xpack-dev-tools/")
            .unwrap_or(repo)
            .strip_suffix("-xpack")
            .unwrap_or(repo);
        let filename = format!("xpack-{slug}-{}-{xpack_host}.{ext}", spec.version);
        let url = format!(
            "https://github.com/{repo}/releases/download/v{}/{filename}",
            spec.version
        );
        Ok((url, filename))
    } else {
        Err(format!(
            "automated URL template for `{}` is not yet configured; check https://github.com/{}/releases",
            spec.target, spec.provider
        ))
    }
}

pub fn find_installed_gcc(cache_dir: &Path, spec: GccTargetSpec) -> Option<GccToolchain> {
    let root = cache_dir.join("toolchains").join(spec.target);
    let bin = root.join("bin");
    let exe = if cfg!(windows) { ".exe" } else { "" };
    let gcc = bin.join(format!("{}-gcc{exe}", spec.tool_prefix));
    if !gcc.is_file() {
        return None;
    }
    let gxx = bin.join(format!("{}-g++{exe}", spec.tool_prefix));
    let ar = bin.join(format!("{}-ar{exe}", spec.tool_prefix));
    let opt_tool = |name: &str| {
        let p = bin.join(format!("{}-{name}{exe}", spec.tool_prefix));
        p.is_file().then_some(p)
    };
    Some(GccToolchain {
        root,
        gcc,
        gxx,
        ar,
        ranlib: opt_tool("ranlib"),
        objcopy: opt_tool("objcopy"),
        objdump: opt_tool("objdump"),
        strip: opt_tool("strip"),
        size: opt_tool("size"),
        target: spec.target.to_string(),
        version: spec.version.to_string(),
    })
}

pub fn install_gcc(target: &str, cache_dir: &Path) -> Result<GccToolchain, String> {
    let spec = find_spec(target)?;
    if let Some(existing) = find_installed_gcc(cache_dir, spec) {
        return Ok(existing);
    }

    let host = host_triple()?;
    let (url, filename) = gcc_download_url(spec, host)?;
    let dl_dir = cache_dir.join("dl");
    let archive_path = dl_dir.join(&filename);
    let install_dir = cache_dir.join("toolchains").join(spec.target);

    let pb = if archive_path.is_file() {
        make_bytes_bar("GCC", 0, "📦", "unpacking")
    } else {
        let bar = make_bytes_bar("GCC", 0, "📥", "downloading");
        download_to_file(&url, &archive_path, &bar)?;
        bar.set_message(format_status("📦", "unpacking"));
        bar
    };

    if install_dir.exists() {
        fs::remove_dir_all(&install_dir)
            .map_err(|e| format!("failed cleaning {}: {e}", install_dir.display()))?;
    }
    extract_archive(&archive_path, &install_dir, 1, &pb)?;
    pb.finish_with_message(format_status("📦", "installed"));

    find_installed_gcc(cache_dir, spec).ok_or_else(|| {
        format!(
            "extracted archive at {} did not contain {}-gcc under bin/",
            install_dir.display(),
            spec.tool_prefix
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_xpack_mingw_and_embedded_urls() {
        let mingw = find_spec("x86_64-pc-windows-gnu").unwrap();
        let (url, file) = gcc_download_url(mingw, "aarch64-apple-darwin").unwrap();
        assert_eq!(file, "xpack-mingw-w64-gcc-15.2.0-2-darwin-arm64.tar.gz");
        assert!(url.contains("xpack-dev-tools/mingw-w64-gcc-xpack/releases/download/v15.2.0-2/"));

        let arm = find_spec("arm-none-eabi").unwrap();
        let (arm_url, arm_file) = gcc_download_url(arm, "x86_64-pc-windows-msvc").unwrap();
        assert_eq!(arm_file, "xpack-arm-none-eabi-gcc-14.2.1-1.1-win32-x64.zip");
        assert!(arm_url.ends_with(".zip"));

        // Windows ARM64 host falls back to win32-x64 xPack archive
        let (_, win_arm64_file) = gcc_download_url(arm, "aarch64-pc-windows-msvc").unwrap();
        assert_eq!(
            win_arm64_file,
            "xpack-arm-none-eabi-gcc-14.2.1-1.1-win32-x64.zip"
        );
    }

    #[test]
    fn routes_rust_and_gnu_triples_via_xtarget() {
        assert_eq!(
            find_spec("thumbv7em-none-eabi").unwrap().tool_prefix,
            "arm-none-eabi"
        );
        assert_eq!(
            find_spec("thumbv6m-none-eabi").unwrap().tool_prefix,
            "arm-none-eabi"
        );
        assert_eq!(
            find_spec("riscv32imac-unknown-none-elf")
                .unwrap()
                .tool_prefix,
            "riscv-none-elf"
        );
        assert_eq!(
            find_spec("riscv64gc-unknown-none-elf").unwrap().tool_prefix,
            "riscv-none-elf"
        );
        assert_eq!(
            find_spec("aarch64-unknown-none").unwrap().tool_prefix,
            "aarch64-none-elf"
        );
        assert_eq!(
            find_spec("x86_64-w64-mingw32").unwrap().target,
            "x86_64-pc-windows-gnu"
        );
        assert_eq!(
            find_spec("aarch64-w64-mingw32").unwrap().target,
            "aarch64-pc-windows-gnu"
        );
    }
}
