# rust-dos

**Work in progress.** Nothing here is stable; the toolchain does not yet
produce a bootable DOS program.

An integrated toolchain for building DOS applications with Rust:
write `#[no_std]` Rust, run `cargo run`, it boots in DOS and prints.

## Requirements

- Rust nightly via rustup, with `rustfmt`, `clippy`, and `rust-src`
  (`rust-src` is needed for `build-std`).
- `nasm` on `PATH` for the stub.
- Python 3 for the verification scripts; they use only the standard library.
- For the COM1 e2e: `dosbox` or `dosbox-staging` on `PATH`, and network access
  to fetch CWSDPMI.

## Testing

Run every local gate with `python3 scripts/check.py`. It runs format, clippy,
the Rust tests, and both DOS e2e checks in sequence. The two e2e scripts share
the example's build target and `hello.exe`; do not run them concurrently.

For testing only, we use [CWSDPMI](http://www.ibiblio.org/pub/micro/pc-stuff/freedos/files/repositories/1.4/tools/cwsdpmi.zip),
a GPL-2.0 DPMI host, which DOSBox does not provide. It is never part of the
toolchain or its output. Its terms are in `DOC/CWSDPMI/COPYING.CWS` in the zip.

Scenarios that need a host to fail a call use `scripts/fakehost.asm`, a tiny
host built with a compile-time switch per behaviour. CWSDPMI itself is never
modified; it only runs the success path.

## License

MIT OR Apache-2.0