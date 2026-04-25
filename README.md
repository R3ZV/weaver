# Run locally

Dependencies:
- Rust
- clang
- llvm
- bpftool
- linux-headers

Build the project `cargo build --release` and in order to execute the binary
you have to use `sudo ./target/release/weaver`.

# Benchmarking

See [this](./bench/README.md).
