<div align="center">

<h1>xcompiler</h1>

**Unified portable compiler manager (`xllvm` + `xgcc`) for zero-Docker cross-compilation.**

Pairs directly with [`xsysroot`](https://github.com/cross-toolchains/xsysroot) and [`xforge`](https://github.com/cross-toolchains/xforge) across Linux, macOS, and Windows.

</div>

---

## Crates

| Crate | Engine | Targets | Upstream |
| :--- | :--- | :--- | :--- |
| [`xllvm`](crates/xllvm/README.md) | **LLVM** (`clang` + `lld` + binutils) | Linux, Windows MSVC & gnullvm, macOS, iOS, Android, OpenHarmony, WebAssembly | [`cross-toolchains/llvm-slim`](https://github.com/cross-toolchains/llvm-slim)<br>[`kernel.org/pub/tools/llvm`](https://www.kernel.org/pub/tools/llvm/) |
| [`xgcc`](crates/xgcc/README.md) | **GCC** (cross) | Windows MinGW (`*-pc-windows-gnu`), Embedded bare-metal (`arm-none-eabi`, `aarch64-none-elf`, `riscv-none-elf`) | [`xpack-dev-tools/mingw-w64-gcc-xpack`](https://github.com/xpack-dev-tools/mingw-w64-gcc-xpack)<br>[`xpack-dev-tools/arm-none-eabi-gcc-xpack`](https://github.com/xpack-dev-tools/arm-none-eabi-gcc-xpack)<br>[`xpack-dev-tools/aarch64-none-elf-gcc-xpack`](https://github.com/xpack-dev-tools/aarch64-none-elf-gcc-xpack)<br>[`xpack-dev-tools/riscv-none-elf-gcc-xpack`](https://github.com/xpack-dev-tools/riscv-none-elf-gcc-xpack)<br>[`arm/gnu-toolchain-for-windows-on-arm64`](https://github.com/arm/gnu-toolchain-for-windows-on-arm64) |
| `xcompiler` | Unified CLI + Lib | Re-exports `xllvm` & `xgcc` with a unified `ResolvedCompiler` API | — |
| `xcompiler-core` | Shared Lib | Streaming HTTP, pure-Rust `.tar.xz`/`.tar.gz`/`.zip`, progress bars, `cli-table` | — |

## 📦 Install

### From binary

```sh
cargo binstall xcompiler
```

### From tarball

Download a prebuilt archive from [Releases](https://github.com/cross-toolchains/xcompiler/releases), or extract directly:

```sh
curl -fsSL https://github.com/cross-toolchains/xcompiler/releases/latest/download/xcompiler-x86_64-unknown-linux-musl.tar.gz | tar -xz
```

### From source

```sh
cargo install xcompiler --locked
# or without --locked
cargo install xcompiler
```

## 🚀 Usage

```sh
# 1) Check host & managed compiler health (verifies clang + lld)
xcompiler doctor

# 2) Install portable slim LLVM (~26 MiB) and expose clang/lld in your shell
xcompiler install llvm 23.1.6
eval "$(xllvm env --managed)"
clang hello.c -o hello

# 3) Pair with xsysroot (xlinux / xdroid / xmac / xios / xohos) for cross-compilation
xlinux splat --target x86_64-unknown-linux-gnu --clang "$(xllvm path --managed)"
~/.cache/xlinux/splat/x86_64-unknown-linux-gnu.2.31/bin/x86_64-unknown-linux-gnu-clang hello.c -o hello_linux

# 4) Or use xgcc to cross-compile Windows MinGW / ARM bare-metal
xcompiler install gcc --target x86_64-pc-windows-gnu
"$(xgcc path --target x86_64-pc-windows-gnu)" hello.c -o hello.exe
```

### As a Rust library

```rust
use xcompiler::xllvm::{InstallOptions, resolve_or_install};

let llvm = resolve_or_install(&InstallOptions::default(), false)?;
println!("clang: {}", llvm.clang.display());
println!("lld:   {}", llvm.lld.display());
```

## 🦀 Library

```sh
cargo add xcompiler --no-default-features
```

```toml
[dependencies]
xcompiler = { version = "0.1", default-features = false }
```

```rust
use xcompiler::xllvm::{self, InstallOptions};

let tc = xllvm::install_llvm(&InstallOptions::default())?;
println!("clang: {}", tc.clang.display());
```

## License

Dual-licensed under `MIT OR Apache-2.0`.
