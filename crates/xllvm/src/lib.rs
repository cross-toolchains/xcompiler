//! `xllvm` — Portable slim LLVM (`clang` + `lld` + binutils) manager for
//! zero-Docker cross-compilation.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use xcompiler_core::{
    default_tool_cache_dir, download_to_file, extract_archive, format_status, host_triple,
    make_bytes_bar,
};

pub const DEFAULT_LLVM_VERSION: &str = "23.1.6";
pub const KNOWN_LLVM_VERSIONS: &[&str] = &["23.1.6", "23.1.5", "22.1.0", "21.1.0"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlvmSource {
    /// Minimal portable builds from `cross-toolchains/llvm-slim`.
    Slim,
    /// Linux kernel official slim LLVM builds (`kernel.org/pub/tools/llvm/`).
    KernelOrg,
}

impl LlvmSource {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "slim" | "llvm-slim" => Ok(Self::Slim),
            "kernel" | "kernel-org" | "kernel.org" => Ok(Self::KernelOrg),
            other => Err(format!(
                "unknown LLVM source `{other}` (expected `slim` or `kernel-org`)"
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LlvmToolchain {
    pub root: PathBuf,
    pub clang: PathBuf,
    pub clangxx: PathBuf,
    pub lld: PathBuf,
    pub ar: Option<PathBuf>,
    pub ranlib: Option<PathBuf>,
    pub objcopy: Option<PathBuf>,
    pub version: String,
    pub managed: bool,
}

#[derive(Debug, Clone)]
pub struct InstallOptions {
    pub version: String,
    pub source: LlvmSource,
    pub cache_dir: PathBuf,
    pub verbose: bool,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            version: DEFAULT_LLVM_VERSION.to_string(),
            source: LlvmSource::Slim,
            cache_dir: default_cache_dir(),
            verbose: false,
        }
    }
}

pub fn default_cache_dir() -> PathBuf {
    default_tool_cache_dir("xllvm")
}

/// Builds the download URL and archive filename for a given LLVM version, source, and host triple.
pub fn llvm_download_url(
    version: &str,
    source: LlvmSource,
    host: &str,
) -> Result<(String, String), String> {
    match source {
        LlvmSource::Slim => {
            let ext = if host.contains("windows") {
                "zip"
            } else {
                "tar.xz"
            };
            let filename = format!("llvm-slim-{version}-{host}.{ext}");
            let url = format!(
                "https://github.com/cross-toolchains/llvm-slim/releases/download/llvm-{version}/{filename}"
            );
            Ok((url, filename))
        }
        LlvmSource::KernelOrg => {
            let arch = match host {
                "x86_64-unknown-linux-gnu" => "x86_64",
                "aarch64-unknown-linux-gnu" => "arm64",
                other => {
                    return Err(format!(
                        "kernel.org/pub/tools/llvm only provides Linux x86_64/arm64 hosts (current host: `{other}`)"
                    ));
                }
            };
            let filename = format!("llvm-{version}-{arch}.tar.xz");
            let url = format!("https://www.kernel.org/pub/tools/llvm/files/{filename}");
            Ok((url, filename))
        }
    }
}

/// Detects a healthy host LLVM installation that has BOTH `clang` and `lld`.
///
/// Automatically skips Apple Clang (`/usr/bin/clang`) on macOS since it does
/// not ship `lld` for cross-linking ELF/PE targets.
pub fn detect_host_llvm() -> Option<LlvmToolchain> {
    let mut candidate_dirs: Vec<PathBuf> = vec![
        PathBuf::from("/opt/homebrew/opt/llvm/bin"),
        PathBuf::from("/usr/local/opt/llvm/bin"),
        PathBuf::from(r"C:\Program Files\LLVM\bin"),
    ];
    if let Some(path_var) = env::var_os("PATH") {
        candidate_dirs.extend(env::split_paths(&path_var));
    }

    for dir in candidate_dirs {
        if let Some(tc) = inspect_bin_dir(&dir, false) {
            return Some(tc);
        }
    }
    None
}

/// Returns an already-installed managed LLVM toolchain in `cache_dir` if present.
pub fn find_managed_llvm(cache_dir: &Path, version: &str) -> Option<LlvmToolchain> {
    let bin_dir = cache_dir.join("toolchains").join(version).join("bin");
    inspect_bin_dir(&bin_dir, true)
}

/// Resolves a usable LLVM toolchain: prefers a healthy host LLVM unless `force_managed`
/// is true, otherwise falls back to (or installs) the managed `xllvm` toolchain.
pub fn resolve_or_install(
    opts: &InstallOptions,
    force_managed: bool,
) -> Result<LlvmToolchain, String> {
    if !force_managed && let Some(host_tc) = detect_host_llvm() {
        return Ok(host_tc);
    }
    if let Some(managed) = find_managed_llvm(&opts.cache_dir, &opts.version) {
        return Ok(managed);
    }
    install_llvm(opts)
}

/// Downloads and installs a portable slim LLVM toolchain into `<cache-dir>/toolchains/<version>`.
pub fn install_llvm(opts: &InstallOptions) -> Result<LlvmToolchain, String> {
    let host = host_triple()?;
    let (url, filename) = llvm_download_url(&opts.version, opts.source, host)?;

    let dl_dir = opts.cache_dir.join("dl");
    let archive_path = dl_dir.join(&filename);
    let install_dir = opts.cache_dir.join("toolchains").join(&opts.version);

    if let Some(existing) = inspect_bin_dir(&install_dir.join("bin"), true) {
        return Ok(existing);
    }

    let pb = if archive_path.is_file() {
        make_bytes_bar("LLVM", 0, "📦", "unpacking")
    } else {
        let bar = make_bytes_bar("LLVM", 0, "📥", "downloading");
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

    inspect_bin_dir(&install_dir.join("bin"), true).ok_or_else(|| {
        format!(
            "extracted archive at {} did not contain clang + lld under bin/",
            install_dir.display()
        )
    })
}

fn inspect_bin_dir(bin_dir: &Path, managed: bool) -> Option<LlvmToolchain> {
    let exe = if cfg!(windows) { ".exe" } else { "" };
    let clang = bin_dir.join(format!("clang{exe}"));
    if !clang.is_file() {
        return None;
    }

    // Find lld either alongside clang or in Homebrew's separate `lld` formula on macOS.
    let lld = find_lld_for(bin_dir)?;
    let version = probe_clang_version(&clang)?;
    // Skip Apple Clang which cannot cross-link ELF/PE targets.
    if !managed && version.contains("Apple clang") {
        return None;
    }

    let clangxx = {
        let p = bin_dir.join(format!("clang++{exe}"));
        if p.is_file() { p } else { clang.clone() }
    };
    let opt_tool = |name: &str| {
        let p = bin_dir.join(format!("{name}{exe}"));
        p.is_file().then_some(p)
    };
    let root = bin_dir.parent().unwrap_or(bin_dir).to_path_buf();

    Some(LlvmToolchain {
        root,
        clang,
        clangxx,
        lld,
        ar: opt_tool("llvm-ar"),
        ranlib: opt_tool("llvm-ranlib"),
        objcopy: opt_tool("llvm-objcopy"),
        version,
        managed,
    })
}

fn find_lld_for(bin_dir: &Path) -> Option<PathBuf> {
    let exe = if cfg!(windows) { ".exe" } else { "" };
    for name in ["ld.lld", "lld"] {
        let candidate = bin_dir.join(format!("{name}{exe}"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    // Homebrew splits `lld` into `/opt/homebrew/opt/lld/bin`.
    for extra in ["/opt/homebrew/opt/lld/bin", "/usr/local/opt/lld/bin"] {
        let candidate = Path::new(extra).join(format!("ld.lld{exe}"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn probe_clang_version(clang: &Path) -> Option<String> {
    let output = Command::new(clang).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().next().map(|l| l.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_llvm_slim_urls_for_all_hosts() {
        let (url, file) =
            llvm_download_url("23.1.6", LlvmSource::Slim, "x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(file, "llvm-slim-23.1.6-x86_64-unknown-linux-gnu.tar.xz");
        assert!(url.contains("/releases/download/llvm-23.1.6/"));

        let (win_url, win_file) =
            llvm_download_url("23.1.6", LlvmSource::Slim, "x86_64-pc-windows-msvc").unwrap();
        assert_eq!(win_file, "llvm-slim-23.1.6-x86_64-pc-windows-msvc.zip");
        assert!(win_url.ends_with(".zip"));
    }

    #[test]
    fn builds_kernel_org_urls_for_linux_only() {
        let (url, _) =
            llvm_download_url("23.1.6", LlvmSource::KernelOrg, "x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(
            url,
            "https://www.kernel.org/pub/tools/llvm/files/llvm-23.1.6-x86_64.tar.xz"
        );
        assert!(
            llvm_download_url("23.1.6", LlvmSource::KernelOrg, "aarch64-apple-darwin").is_err()
        );
    }
}
