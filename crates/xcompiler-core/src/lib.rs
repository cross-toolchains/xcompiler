//! Shared terminal progress bars, `cli-table` rendering, host detection,
//! streaming HTTP downloads, and pure-Rust archive extraction for `xcompiler`.

use std::env;
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use cli_table::{Cell, Style, Table, print_stdout};
use flate2::read::MultiGzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use zip::ZipArchive;

/// Formats an aligned icon + status label for progress bars.
pub fn format_status(icon: &str, label: &str) -> String {
    format!("{icon}  {label:>11}")
}

/// Creates a green `xwin`-style aligned bytes progress bar.
pub fn make_bytes_bar(prefix: &'static str, len: u64, icon: &str, label: &str) -> ProgressBar {
    let pb = ProgressBar::new(len);
    if let Ok(style) = ProgressStyle::with_template(
        "{spinner:.green} {prefix:<9.bold} [{elapsed:>3}] {wide_bar:.green} {binary_bytes:>10}/{binary_total_bytes:>10}  {msg}",
    ) {
        pb.set_style(style.progress_chars("█▄ "));
    }
    pb.set_prefix(prefix);
    pb.set_message(format_status(icon, label));
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Renders a Unicode-bordered table with bold headers to stdout using `cli-table`.
pub fn print_table(headers: &[&str], rows: Vec<Vec<String>>) {
    let title: Vec<_> = headers.iter().map(|h| h.cell().bold(true)).collect();
    let body: Vec<Vec<_>> = rows
        .into_iter()
        .map(|row| row.into_iter().map(|c| c.cell()).collect())
        .collect();
    let table = body.table().title(title);
    let _ = print_stdout(table);
}

/// Resolves `~/.cache/<tool>` (or `%USERPROFILE%\.cache\<tool>` on Windows).
pub fn default_tool_cache_dir(tool: &str) -> PathBuf {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".cache").join(tool)
}

/// Returns the canonical host target triple for the current machine.
pub fn host_triple() -> Result<&'static str, String> {
    match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-gnu"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("windows", "x86_64") => Ok("x86_64-pc-windows-msvc"),
        ("windows", "aarch64") => Ok("aarch64-pc-windows-msvc"),
        (os, arch) => Err(format!("unsupported host platform: {os}/{arch}")),
    }
}

/// Downloads `url` to `dest` with a streaming progress bar.
pub fn download_to_file(url: &str, dest: &Path, pb: &ProgressBar) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed creating {}: {e}", parent.display()))?;
    }
    let agent = ureq::Agent::new_with_defaults();
    let response = agent
        .get(url)
        .call()
        .map_err(|e| format!("HTTP GET {url} failed: {e}"))?;

    if let Some(len) = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
    {
        pb.set_length(len);
    }

    let tmp_path = dest.with_extension("part");
    let mut out = File::create(&tmp_path)
        .map_err(|e| format!("failed creating {}: {e}", tmp_path.display()))?;
    let mut reader = response.into_body().into_reader();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("failed reading {url}: {e}"))?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])
            .map_err(|e| format!("failed writing {}: {e}", tmp_path.display()))?;
        pb.inc(n as u64);
    }
    out.flush()
        .map_err(|e| format!("failed flushing {}: {e}", tmp_path.display()))?;
    fs::rename(&tmp_path, dest).map_err(|e| {
        format!(
            "failed moving {} -> {}: {e}",
            tmp_path.display(),
            dest.display()
        )
    })?;
    Ok(())
}

/// Rejects archive entry paths containing `..`, root prefixes, or drive prefixes.
pub fn sanitize_relative_path(rel: &str) -> Result<PathBuf, String> {
    let mut clean = PathBuf::new();
    for comp in Path::new(rel).components() {
        match comp {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            _ => return Err(format!("unsafe archive entry path: `{rel}`")),
        }
    }
    Ok(clean)
}

/// Extracts a `.tar.xz`, `.tar.gz`/`.tgz`, or `.zip` archive into `dest_dir`,
/// stripping `strip_components` leading path segments.
pub fn extract_archive(
    archive_path: &Path,
    dest_dir: &Path,
    strip_components: usize,
    pb: &ProgressBar,
) -> Result<(), String> {
    fs::create_dir_all(dest_dir)
        .map_err(|e| format!("failed creating {}: {e}", dest_dir.display()))?;

    let name = archive_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if name.ends_with(".zip") {
        extract_zip(archive_path, dest_dir, strip_components, pb)
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let file = File::open(archive_path)
            .map_err(|e| format!("failed opening {}: {e}", archive_path.display()))?;
        extract_tar(
            MultiGzDecoder::new(BufReader::new(file)),
            dest_dir,
            strip_components,
            pb,
        )
    } else if name.ends_with(".tar.xz") {
        let file = File::open(archive_path)
            .map_err(|e| format!("failed opening {}: {e}", archive_path.display()))?;
        let mut tar_bytes = Vec::new();
        let mut reader = BufReader::new(file);
        lzma_rs::xz_decompress(&mut reader, &mut tar_bytes)
            .map_err(|e| format!("failed decompressing {}: {e}", archive_path.display()))?;
        extract_tar(io::Cursor::new(tar_bytes), dest_dir, strip_components, pb)
    } else {
        Err(format!(
            "unsupported archive format: {}",
            archive_path.display()
        ))
    }
}

fn strip_leading(path: &Path, strip: usize) -> Option<PathBuf> {
    let mut comps = path.components();
    for _ in 0..strip {
        comps.next()?;
    }
    let rest: PathBuf = comps.collect();
    if rest.as_os_str().is_empty() {
        None
    } else {
        Some(rest)
    }
}

fn extract_zip(
    archive_path: &Path,
    dest_dir: &Path,
    strip_components: usize,
    pb: &ProgressBar,
) -> Result<(), String> {
    let file = File::open(archive_path)
        .map_err(|e| format!("failed opening {}: {e}", archive_path.display()))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("failed reading zip {}: {e}", archive_path.display()))?;

    let mut total: u64 = 0;
    for i in 0..archive.len() {
        if let Ok(e) = archive.by_index_raw(i) {
            total = total.saturating_add(e.size());
        }
    }
    pb.reset();
    pb.set_length(total);

    let mut symlinks = SymlinkQueue::new();
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("failed reading zip entry #{i}: {e}"))?;
        let rel = sanitize_relative_path(entry.name())?;
        let Some(rel) = strip_leading(&rel, strip_components) else {
            continue;
        };
        let out_path = dest_dir.join(&rel);
        if entry.is_dir() {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("failed creating {}: {e}", out_path.display()))?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed creating {}: {e}", parent.display()))?;
        }
        if entry.is_symlink() {
            let target = io::read_to_string(&mut entry)
                .map_err(|e| format!("failed reading symlink target: {e}"))?;
            symlinks.add(&target, &out_path)?;
            continue;
        }
        let mut out = File::create(&out_path)
            .map_err(|e| format!("failed creating {}: {e}", out_path.display()))?;
        let n = io::copy(&mut entry, &mut out)
            .map_err(|e| format!("failed extracting {}: {e}", out_path.display()))?;
        pb.inc(n);
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&out_path, fs::Permissions::from_mode(mode & 0o777));
        }
    }
    symlinks.finish()
}

fn extract_tar<R: Read>(
    mut reader: R,
    dest_dir: &Path,
    strip_components: usize,
    pb: &ProgressBar,
) -> Result<(), String> {
    let mut header = [0u8; 512];
    let mut next_long_name: Option<String> = None;
    let mut next_long_link: Option<String> = None;
    let mut symlinks = SymlinkQueue::new();

    loop {
        if !read_exact_or_eof(&mut reader, &mut header)? {
            break;
        }
        if header.iter().all(|&b| b == 0) {
            continue;
        }
        let typeflag = header[156];
        let size = parse_tar_octal(&header[124..136])?;
        let pad = (512 - (size % 512)) % 512;

        if typeflag == b'L' {
            let data = read_vec(&mut reader, size)?;
            skip_exact(&mut reader, pad)?;
            next_long_name = Some(
                String::from_utf8_lossy(&data)
                    .trim_end_matches('\0')
                    .to_string(),
            );
            continue;
        }
        if typeflag == b'K' {
            let data = read_vec(&mut reader, size)?;
            skip_exact(&mut reader, pad)?;
            next_long_link = Some(
                String::from_utf8_lossy(&data)
                    .trim_end_matches('\0')
                    .to_string(),
            );
            continue;
        }
        if typeflag == b'x' || typeflag == b'g' {
            let data = read_vec(&mut reader, size)?;
            skip_exact(&mut reader, pad)?;
            if typeflag == b'x' {
                parse_pax_records(&data, &mut next_long_name, &mut next_long_link);
            }
            continue;
        }

        let raw_name = next_long_name.take().unwrap_or_else(|| {
            let name = cstr_utf8(&header[0..100]);
            let prefix = cstr_utf8(&header[345..500]);
            if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            }
        });
        let raw_link = next_long_link
            .take()
            .unwrap_or_else(|| cstr_utf8(&header[157..257]));

        let clean = raw_name.trim_start_matches("./").trim_start_matches('/');
        if clean.is_empty() {
            skip_exact(&mut reader, size + pad)?;
            continue;
        }
        let rel = sanitize_relative_path(clean)?;
        let Some(rel) = strip_leading(&rel, strip_components) else {
            skip_exact(&mut reader, size + pad)?;
            continue;
        };
        let out_path = dest_dir.join(&rel);
        #[cfg(unix)]
        let mode = parse_tar_octal(&header[100..108]).unwrap_or(0o644) as u32;

        match typeflag {
            b'5' => {
                fs::create_dir_all(&out_path)
                    .map_err(|e| format!("failed creating {}: {e}", out_path.display()))?;
                skip_exact(&mut reader, size + pad)?;
            }
            b'2' => {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("failed creating {}: {e}", parent.display()))?;
                }
                symlinks.add(&raw_link, &out_path)?;
                skip_exact(&mut reader, size + pad)?;
            }
            b'1' => {
                let link_clean = raw_link.trim_start_matches("./").trim_start_matches('/');
                let link_rel = sanitize_relative_path(link_clean)?;
                if let Some(target_rel) = strip_leading(&link_rel, strip_components) {
                    let target_path = dest_dir.join(target_rel);
                    if let Some(parent) = out_path.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|e| format!("failed creating {}: {e}", parent.display()))?;
                    }
                    if target_path.is_file() && fs::hard_link(&target_path, &out_path).is_err() {
                        let _ = fs::copy(&target_path, &out_path);
                    }
                }
                skip_exact(&mut reader, size + pad)?;
            }
            b'0' | 0 => {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("failed creating {}: {e}", parent.display()))?;
                }
                if out_path.symlink_metadata().is_ok() {
                    let _ = fs::remove_file(&out_path);
                }
                let mut out = File::create(&out_path)
                    .map_err(|e| format!("failed creating {}: {e}", out_path.display()))?;
                let mut limited = (&mut reader).take(size);
                let n = io::copy(&mut limited, &mut out)
                    .map_err(|e| format!("failed extracting {}: {e}", out_path.display()))?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ =
                        fs::set_permissions(&out_path, fs::Permissions::from_mode(mode & 0o777));
                }
                skip_exact(&mut reader, pad)?;
                let new_pos = pb.position() + n;
                if new_pos > pb.length().unwrap_or(0) {
                    pb.set_length(new_pos);
                }
                pb.set_position(new_pos);
            }
            _ => skip_exact(&mut reader, size + pad)?,
        }
    }
    symlinks.finish()
}

fn cstr_utf8(slice: &[u8]) -> String {
    let end = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
    String::from_utf8_lossy(&slice[..end]).into_owned()
}

fn parse_tar_octal(slice: &[u8]) -> Result<u64, String> {
    let s = cstr_utf8(slice);
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(0);
    }
    u64::from_str_radix(trimmed, 8).map_err(|e| format!("invalid tar octal `{trimmed}`: {e}"))
}

fn parse_pax_records(data: &[u8], path: &mut Option<String>, linkpath: &mut Option<String>) {
    let text = String::from_utf8_lossy(data);
    for line in text.lines() {
        if let Some((_, kv)) = line.split_once(' ') {
            if let Some(val) = kv.strip_prefix("path=") {
                *path = Some(val.to_string());
            } else if let Some(val) = kv.strip_prefix("linkpath=") {
                *linkpath = Some(val.to_string());
            }
        }
    }
}

fn read_exact_or_eof<R: Read>(r: &mut R, buf: &mut [u8]) -> Result<bool, String> {
    let mut read = 0;
    while read < buf.len() {
        let n = r
            .read(&mut buf[read..])
            .map_err(|e| format!("tar read error: {e}"))?;
        if n == 0 {
            return Ok(false);
        }
        read += n;
    }
    Ok(true)
}

fn read_vec<R: Read>(r: &mut R, size: u64) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; size as usize];
    r.read_exact(&mut buf)
        .map_err(|e| format!("tar read error: {e}"))?;
    Ok(buf)
}

fn skip_exact<R: Read>(r: &mut R, mut n: u64) -> Result<(), String> {
    let mut sink = [0u8; 8192];
    while n > 0 {
        let to_read = (n as usize).min(sink.len());
        let got = r
            .read(&mut sink[..to_read])
            .map_err(|e| format!("tar skip error: {e}"))?;
        if got == 0 {
            break;
        }
        n -= got as u64;
    }
    Ok(())
}

/// Deferred symlink queue for cross-platform archive extraction.
#[derive(Default)]
pub struct SymlinkQueue {
    #[cfg(not(unix))]
    pending: Vec<(String, PathBuf)>,
}

impl SymlinkQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, target: &str, link: &Path) -> Result<(), String> {
        #[cfg(unix)]
        {
            if link.exists() || link.symlink_metadata().is_ok() {
                let _ = fs::remove_file(link);
            }
            std::os::unix::fs::symlink(target, link).map_err(|e| {
                format!(
                    "failed to create symlink {} -> {target}: {e}",
                    link.display()
                )
            })
        }
        #[cfg(not(unix))]
        {
            self.pending.push((target.to_string(), link.to_path_buf()));
            Ok(())
        }
    }

    pub fn finish(self) -> Result<(), String> {
        #[cfg(unix)]
        {
            Ok(())
        }
        #[cfg(not(unix))]
        {
            for (target, link) in self.pending {
                let win_target = target.replace('/', "\\");
                let resolved = link.parent().unwrap_or(Path::new(".")).join(&win_target);
                if resolved.is_dir() {
                    #[cfg(windows)]
                    if std::os::windows::fs::symlink_dir(&win_target, &link).is_ok() {
                        continue;
                    }
                } else if resolved.is_file() {
                    #[cfg(windows)]
                    if std::os::windows::fs::symlink_file(&win_target, &link).is_ok() {
                        continue;
                    }
                    if fs::hard_link(&resolved, &link).is_err() {
                        let _ = fs::copy(&resolved, &link);
                    }
                }
            }
            Ok(())
        }
    }
}
