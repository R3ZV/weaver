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
- https://en.wikipedia.org/wiki/Exponential_decay
- https://web.archive.org/web/20260225090858/https://citeseerx.ist.psu.edu/document?doi=805acf7726282721504c8f00575d91ebfd750564&repid=rep1&type=pdf
