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

# References

- https://github.com/sched-ext/scx/blob/main/scheds/rust/scx_lavd/src/bpf/main.bpf.c
- https://lpc.events/event/18/contributions/1713/attachments/1425/3058/scx_lavd-lpc-mc-24.pdf
- https://docs.kernel.org/scheduler/sched-eevdf.html
- https://docs.kernel.org/scheduler/sched-design-CFS.html
- https://lwn.net/Articles/1051430/
- https://www.youtube.com/watch?v=gZaZIZ1W1Vo
