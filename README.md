# rust-dos

**Work in progress.** Nothing here is stable; the toolchain does not yet
produce a bootable DOS program.

An integrated toolchain for building DOS applications with Rust:
write `#[no_std]` Rust, run `cargo run`, it boots in DOS and prints.

## Testing

For testing only, we use [CWSDPMI](http://www.ibiblio.org/pub/micro/pc-stuff/freedos/files/repositories/1.4/tools/cwsdpmi.zip),
a GPL-2.0 DPMI host, which DOSBox does not provide. It is never part of the
toolchain or its output. Its terms are in `DOC/CWSDPMI/COPYING.CWS` in the zip.

The COM1 test harness mutates one downloaded copy at run time to simulate a
host that will not switch a 32-bit client, and keeps it local.

## License

MIT OR Apache-2.0