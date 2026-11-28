<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://api.iconify.design/devicon-plain/gcc.svg?color=%23ffffff" />
  <source media="(prefers-color-scheme: light)" srcset="https://api.iconify.design/devicon-plain/gcc.svg?color=%23000000" />
  <img src="https://api.iconify.design/devicon-plain/gcc.svg?color=%23000000" height="56" alt="GNU GCC" />
</picture>

<h1>xgcc</h1>

**Portable prebuilt cross-GCC toolchain manager for Windows MinGW-w64 and embedded bare-metal targets.**

Downloads official/xPack prebuilt toolchains across Linux, macOS, and Windows hosts without building GCC from source.

</div>

---

## 📦 Install

### From binary

```sh
cargo binstall xgcc
```

### From tarball

Download a prebuilt archive from [Releases](https://github.com/cross-toolchains/xcompiler/releases), or extract directly:

```sh
curl -fsSL https://github.com/cross-toolchains/xcompiler/releases/latest/download/xgcc-x86_64-unknown-linux-musl.tar.gz | tar -xz
```

### From source

```sh
cargo install xgcc --locked
# or without --locked
cargo install xgcc
```

## 🚀 Usage

```sh
# List supported cross-GCC targets
xgcc list

# Cross-compile a Windows x86_64 binary from Linux / macOS / Windows
xgcc install --target x86_64-pc-windows-gnu
"$(xgcc path --target x86_64-pc-windows-gnu)" hello.c -o hello.exe

# Compile an ARM Cortex-M bare-metal object
xgcc install --target arm-none-eabi
"$(xgcc path --target arm-none-eabi)" -mcpu=cortex-m4 -mthumb -c main.c -o main.o
```

## Upstream Providers

| Target | Upstream |
| :--- | :--- |
| `x86_64-pc-windows-gnu` | [`xpack-dev-tools/mingw-w64-gcc-xpack`](https://github.com/xpack-dev-tools/mingw-w64-gcc-xpack) |
| `aarch64-pc-windows-gnu` | [`arm/gnu-toolchain-for-windows-on-arm64`](https://github.com/arm/gnu-toolchain-for-windows-on-arm64) |
| `arm-none-eabi` | [`xpack-dev-tools/arm-none-eabi-gcc-xpack`](https://github.com/xpack-dev-tools/arm-none-eabi-gcc-xpack) |
| `aarch64-none-elf` | [`xpack-dev-tools/aarch64-none-elf-gcc-xpack`](https://github.com/xpack-dev-tools/aarch64-none-elf-gcc-xpack) |
| `riscv-none-elf` | [`xpack-dev-tools/riscv-none-elf-gcc-xpack`](https://github.com/xpack-dev-tools/riscv-none-elf-gcc-xpack) |

> [!NOTE]
> **Windows ARM64 (`aarch64-pc-windows-msvc`) hosts:** Upstream xPack releases do not publish native `win32-arm64` host binaries because mainline GCC lacks complete Windows-on-Arm host support. `xgcc` automatically falls back to the `win32-x64` archive, which runs transparently on Windows 11 ARM64 via Prism x64 emulation. For native Windows ARM64 host binaries, use [`xllvm`](../xllvm/README.md).
>
> **Target triple normalization:** Powered by [`xtarget`](https://crates.io/crates/xtarget), Rust bare-metal and MinGW triples (e.g. `thumbv7em-none-eabi`, `riscv32imac-unknown-none-elf`, `aarch64-unknown-none`, `x86_64-w64-mingw32`) automatically route to their corresponding GCC multilib toolchain.

## 🦀 Library

```sh
cargo add xgcc --no-default-features
```

```toml
[dependencies]
xgcc = { version = "0.1", default-features = false }
```

```rust
let cache_dir = xgcc::default_cache_dir();
let tc = xgcc::install_gcc("arm-none-eabi", &cache_dir)?;
println!("gcc: {}", tc.gcc.display());
```

## License

Dual-licensed under `MIT OR Apache-2.0`.
