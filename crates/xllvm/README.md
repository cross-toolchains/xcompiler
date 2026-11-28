<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://api.iconify.design/simple-icons/llvm.svg?color=%23ffffff" />
  <source media="(prefers-color-scheme: light)" srcset="https://api.iconify.design/simple-icons/llvm.svg?color=%23000000" />
  <img src="https://api.iconify.design/simple-icons/llvm.svg?color=%23000000" height="56" alt="LLVM" />
</picture>

<h1>xllvm</h1>

**Portable slim LLVM (`clang` + `lld` + binutils) manager for zero-Docker cross-compilation.**

Auto-detects healthy host LLVM (skipping Apple Clang without `lld`) or downloads version-pinned minimal LLVM builds from [`cross-toolchains/llvm-slim`](https://github.com/cross-toolchains/llvm-slim).

</div>

---

## 📦 Install

### From binary

```sh
cargo binstall xllvm
```

### From tarball

Download a prebuilt archive from [Releases](https://github.com/cross-toolchains/xcompiler/releases), or extract directly:

```sh
curl -fsSL https://github.com/cross-toolchains/xcompiler/releases/latest/download/xllvm-x86_64-unknown-linux-musl.tar.gz | tar -xz
```

### From source

```sh
cargo install xllvm --locked
# or without --locked
cargo install xllvm
```

## 🚀 Usage

```sh
# Check host & managed LLVM status
xllvm doctor

# Install portable slim LLVM and add clang/lld to PATH
xllvm install 23.1.6
eval "$(xllvm env --managed)"

# Compile a C program directly
clang hello.c -o hello

# Pair with xsysroot (e.g. xlinux / xdroid / xmac) for zero-Docker cross-compilation
xlinux splat --target x86_64-unknown-linux-gnu --clang "$(xllvm path --managed)"
~/.cache/xlinux/splat/x86_64-unknown-linux-gnu.2.31/bin/x86_64-unknown-linux-gnu-clang hello.c -o hello_linux
```

## Upstream Sources

| Source (`--source`) | Hosts | Upstream |
| :--- | :--- | :--- |
| `slim` (default) | Linux / macOS / Windows (`x86_64` & `aarch64`) | [`cross-toolchains/llvm-slim`](https://github.com/cross-toolchains/llvm-slim/releases) |
| `kernel-org` | Linux (`x86_64` & `aarch64`) | [`kernel.org/pub/tools/llvm`](https://www.kernel.org/pub/tools/llvm/) |

## 🦀 Library

```sh
cargo add xllvm --no-default-features
```

```toml
[dependencies]
xllvm = { version = "0.1", default-features = false }
```

```rust
use xllvm::InstallOptions;

let tc = xllvm::install_llvm(&InstallOptions::default())?;
println!("clang: {}", tc.clang.display());
```

## License

Dual-licensed under `MIT OR Apache-2.0`.
