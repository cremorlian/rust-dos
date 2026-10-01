import os
import pathlib
import re
import struct
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
HELLO = REPO / "examples/hello"
CARGO_DOS = REPO / "target/debug/cargo-dos"
CONSUMER_CONFIG = HELLO / ".cargo/config.toml"
ELF = HELLO / "target/i486-dos/debug/hello"
EXE = HELLO / "hello.exe"

PT_LOAD = 1
P_TYPE, P_OFFSET, P_VADDR, P_FILESZ = 0, 4, 8, 16
SHT_STRTAB = 3
SH_NAME, SH_TYPE, SH_OFFSET = 0, 4, 16
E_PHOFF, E_SHOFF = 0x1C, 0x20
E_PHENTSIZE, E_PHNUM = 0x2A, 0x2C
E_SHENTSIZE, E_SHNUM, E_SHSTRNDX = 0x2E, 0x30, 0x32
E_CPARHDR = 8


def u16(data, at):
    return struct.unpack_from("<H", data, at)[0]


def u32(data, at):
    return struct.unpack_from("<I", data, at)[0]


def consumer_cargo_home():
    text = CONSUMER_CONFIG.read_text()
    match = re.search(r'^RUST_TARGET_PATH\s*=\s*"([^"]+)"', text, re.MULTILINE)
    if match is None:
        raise SystemExit(f"RUST_TARGET_PATH not found in {CONSUMER_CONFIG}")
    return pathlib.Path(match.group(1)).parent


def run(command, cwd, env):
    result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True)
    if result.returncode != 0:
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        raise SystemExit(f"failed: {' '.join(command)} (in {cwd})")
    return result


def build():
    run(["cargo", "build", "-p", "cargo-dos"], REPO, os.environ.copy())
    home = consumer_cargo_home()
    run([str(CARGO_DOS), "init"], HELLO, {**os.environ, "CARGO_HOME": str(home)})
    env = {**os.environ, "PATH": f"{CARGO_DOS.parent}:{os.environ['PATH']}"}
    run(["cargo", "clean", "--target", "i486-dos"], HELLO, env)
    run(["cargo", "run"], HELLO, env)


def load_segment(data):
    phoff = u32(data, E_PHOFF)
    phentsize = u16(data, E_PHENTSIZE)
    loads = [
        i
        for i in range(u16(data, E_PHNUM))
        if u32(data, phoff + i * phentsize + P_TYPE) == PT_LOAD
    ]
    if len(loads) != 1:
        raise SystemExit(f"expected one PT_LOAD, found {len(loads)}")
    at = phoff + loads[0] * phentsize
    return (
        u32(data, at + P_OFFSET),
        u32(data, at + P_VADDR),
        u32(data, at + P_FILESZ),
    )


def section_offset(data, wanted):
    shoff = u32(data, E_SHOFF)
    shentsize = u16(data, E_SHENTSIZE)
    names_at = u32(data, shoff + u16(data, E_SHSTRNDX) * shentsize + SH_OFFSET)
    for i in range(u16(data, E_SHNUM)):
        at = shoff + i * shentsize
        start = names_at + u32(data, at + SH_NAME)
        if u32(data, at + SH_TYPE) == SHT_STRTAB:
            continue
        if data[start : data.index(b"\0", start)] == wanted:
            return u32(data, at + SH_OFFSET)
    raise SystemExit(f"section {wanted!r} not found")


def check():
    data = ELF.read_bytes()
    p_offset, p_vaddr, p_filesz = load_segment(data)
    text_at = section_offset(data, b".text")
    exe = EXE.read_bytes()

    if exe[:2] != b"MZ":
        raise SystemExit(f"{EXE} does not start with MZ")
    image_at = u16(exe, E_CPARHDR) * 16
    image = exe[image_at : image_at + p_filesz]
    if len(image) != p_filesz:
        raise SystemExit(
            f"{EXE} is {len(exe)} bytes: the image starts at {image_at} and "
            f"p_filesz is {p_filesz}, so the load image runs past the end of the file"
        )

    failures = []
    if p_offset != text_at:
        failures.append(
            f"load image starts at file offset 0x{p_offset:x}, "
            f"but .text starts at 0x{text_at:x} (the headers are inside the image)"
        )
    if image[:4] == b"\x7fELF":
        failures.append("load image starts with the ELF header")
    if image[:4] == struct.pack("<I", PT_LOAD):
        failures.append("load image starts with a program-header row")
    if image[:4] != data[text_at : text_at + 4]:
        failures.append(
            f"load image starts with {image[:4].hex(' ')}, "
            f"but .text starts with {data[text_at:text_at + 4].hex(' ')}"
        )

    print(f"e_cparhdr        : {u16(exe, E_CPARHDR)} paragraphs -> image at {image_at}")
    print(f"PT_LOAD          : p_offset 0x{p_offset:x}  p_vaddr 0x{p_vaddr:x}  p_filesz 0x{p_filesz:x}")
    print(f".text file offset: 0x{text_at:x}")
    print(f"image[0:4]       : {image[:4].hex(' ')}")
    print(f".text[0:4]       : {data[text_at:text_at + 4].hex(' ')}")

    for failure in failures:
        print(f"FAIL: {failure}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    build()
    sys.exit(check())