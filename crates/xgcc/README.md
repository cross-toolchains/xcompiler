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
xgcc list
xgcc install --target x86_64-pc-windows-gnu
xgcc install --target arm-none-eabi
xgcc path --target arm-none-eabi
```

## Upstream Providers

| Target | Upstream |
| :--- | :--- |
| `x86_64-pc-windows-gnu` | [`xpack-dev-tools/mingw-w64-gcc-xpack`](https://github.com/xpack-dev-tools/mingw-w64-gcc-xpack) |
| `aarch64-pc-windows-gnu` | [`arm/gnu-toolchain-for-windows-on-arm64`](https://github.com/arm/gnu-toolchain-for-windows-on-arm64) |
| `arm-none-eabi` | [`xpack-dev-tools/arm-none-eabi-gcc-xpack`](https://github.com/xpack-dev-tools/arm-none-eabi-gcc-xpack) |
| `aarch64-none-elf` | [`xpack-dev-tools/aarch64-none-elf-gcc-xpack`](https://github.com/xpack-dev-tools/aarch64-none-elf-gcc-xpack) |
| `riscv-none-elf` | [`xpack-dev-tools/riscv-none-elf-gcc-xpack`](https://github.com/xpack-dev-tools/riscv-none-elf-gcc-xpack) |

## License

Dual-licensed under `MIT OR Apache-2.0`.
