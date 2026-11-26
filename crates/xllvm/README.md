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
xllvm doctor
xllvm list
xllvm install 23.1.6
eval "$(xllvm env)"
```

## License

Dual-licensed under `MIT OR Apache-2.0`.
