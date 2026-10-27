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
- For the qemu lane e2e: `qemu-system-i386` and `mtools` (`mcopy`, `mdel`) on
  `PATH`, plus network access to fetch FreeDOS and CWSDPMI.
- For the Bochs lane e2e: `bochs` with its BIOS ROMs under `/usr/share/bochs`,
  plus `mtools` and network access to fetch FreeDOS and CWSDPMI.
- For the dosemu2 native e2e: `dosemu` 2.0pre9 with built-in FDPP/comcom64 on
  `PATH` (`dosemu2` package); no FreeDOS image or CWSDPMI.

## Testing

Run every local gate with `python3 scripts/check.py`. It runs format, clippy,
the Rust tests, and the e2e checks in sequence. The e2e scripts share the
example's build target and `hello.exe`; do not run them concurrently.

For testing only, the DOS lanes use DOSBox Staging, QEMU, Bochs, FreeDOS,
CWSDPMI, dosemu2, FDPP, comcom64, and dj64. DOSBox Staging, QEMU, FreeDOS,
CWSDPMI, and dosemu2 are GPLv2-family; FDPP, comcom64, and dj64 are GPLv3+;
Bochs is LGPL-2.0-or-later. They are never linked into or distributed with
`rust-dos` or its output; they are fetched or invoked as separate test
dependencies. The toolchain remains MIT OR Apache-2.0. Redistributing a bundle
or image containing them will carry their notice/source obligations.

Scenarios that need a host to fail a call use `scripts/fakehost.asm`, a tiny
host built with a compile-time switch per behaviour. CWSDPMI itself is never
modified; it only runs the success path.

## License

MIT OR Apache-2.0