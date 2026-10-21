# rust-dos

**Work in progress.** Nothing here is stable; the toolchain does not yet
produce a bootable DOS program.

An integrated toolchain for building DOS applications with Rust:
write `#[no_std]` Rust, run `cargo run`, it boots in DOS and prints.

## Testing

For testing only, we use [CWSDPMI](http://www.ibiblio.org/pub/micro/pc-stuff/freedos/files/repositories/1.4/tools/cwsdpmi.zip),
a GPL-2.0 DPMI host, which DOSBox does not provide. It is never part of the
toolchain or its output. Its terms are in `DOC/CWSDPMI/COPYING.CWS` in the zip.

Scenarios that need a host to fail a call use `scripts/fakehost.asm`, a tiny
host built with a compile-time switch per behaviour. CWSDPMI itself is never
modified; it only runs the success path.

## License

MIT OR Apache-2.0