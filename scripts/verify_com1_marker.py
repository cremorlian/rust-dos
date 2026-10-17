"""Run the example under DOSBox and assert the stub's real-mode COM1 marker.

DOSBox has no DPMI host, so CWSDPMI is downloaded and loaded first. COM1 is
captured over nullmodem: Python binds and listens before DOSBox starts, and
DOSBox is the TCP client, because a program that writes its marker once loses
it when the reader attaches late.
"""

import hashlib
import os
import pathlib
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request
import zipfile

REPO = pathlib.Path(__file__).resolve().parent.parent
HELLO = REPO / "examples/hello"
CONSUMER_CONFIG = HELLO / ".cargo/config.toml"
CARGO_DOS = REPO / "target/debug/cargo-dos"
EXE = HELLO / "hello.exe"

CWSDPMI_URL = (
    "http://www.ibiblio.org/pub/micro/pc-stuff/freedos/files/repositories"
    "/1.4/tools/cwsdpmi.zip"
)
CWSDPMI_SHA1 = "e1d5569817019bbde41eceeeb0a8bed78e87be28"
CWSDPMI_MEMBER = "BIN/CWSDPMI.EXE"

# Test-only mutation of the downloaded copy; never committed or redistributed.
# See the README's Testing section.
BL32_OFFSET = 4967  # low byte of the imm16 in `mov bx,1`, which advertises 32-bit
NO_32BIT = "no32bit"

# 0xFFFF paragraphs cannot fit below the megabyte, so INT 21h/48h fails.
INIT_SIZE_OFFSET = 4972  # `mov si,[cs:_init_size]`, which sizes that allocation
HUNGRY = "hungry"

REFUSES_SWITCH_OFFSET = 4988  # dpmientry, so the switch returns carry
REFUSES = "refuses"

SCENARIOS = [
    ("host loaded", ["cwsdpmi"], b"hello, dos\r\n"),
    ("no host", [], b"stub rm\r\nrust-dos: no DPMI host found\r\n"),
    (
        "host without 32-bit",
        [NO_32BIT],
        b"stub rm\r\nrust-dos: warning: host reports no 32-bit support\r\n"
        b"stub pm\r\n",
    ),
    (
        "host wants more memory than DOS has",
        [HUNGRY],
        b"stub rm\r\nrust-dos: insufficient memory for the DPMI host\r\n",
    ),
    (
        "host refuses mode switch",
        [REFUSES],
        b"stub rm\r\nrust-dos: DPMI host refused mode switch\r\n",
    ),
    (
        "no LDT descriptors",
        ["cwsdpmi"],
        b"stub rm\r\nstub pm\r\nrust-dos: no LDT descriptors available\r\n",
        "many",
    ),
]
HANDSHAKE = b"\xff\x00"
DOSBOX = next(
    (name for name in ("dosbox", "dosbox-staging") if shutil.which(name)), None
)
CONNECT_TIMEOUT = 30.0
READ_TIMEOUT = 2.0
RUN_TIMEOUT = 30.0


def cwsdpmi(cache):
    zip_path = cache / "cwsdpmi.zip"
    try:
        with urllib.request.urlopen(CWSDPMI_URL, timeout=60) as response:
            zip_path.write_bytes(response.read())
    except OSError as e:
        raise SystemExit(f"could not fetch {CWSDPMI_URL}: {e}")

    digest = hashlib.sha1(zip_path.read_bytes()).hexdigest()
    if digest != CWSDPMI_SHA1:
        raise SystemExit(
            f"{CWSDPMI_URL} has sha1 {digest}, expected {CWSDPMI_SHA1}"
        )

    with zipfile.ZipFile(zip_path) as archive:
        extracted = pathlib.Path(archive.extract(CWSDPMI_MEMBER, cache))
    return extracted


def without_32bit(source, dest):
    data = bytearray(source.read_bytes())
    if bytes(data[BL32_OFFSET - 1 : BL32_OFFSET + 2]) != b"\xBB\x01\x00":
        raise SystemExit(f"no `mov bx,1` at {BL32_OFFSET - 1} in {source.name}")
    data[BL32_OFFSET] = 0x00
    dest.write_bytes(bytes(data))


def hungry(source, dest):
    data = bytearray(source.read_bytes())
    load = b"\x2E\x8B\x36\x58\x11"  # mov si,[cs:_init_size]
    if bytes(data[INIT_SIZE_OFFSET : INIT_SIZE_OFFSET + len(load)]) != load:
        raise SystemExit(f"no `mov si,[cs:_init_size]` at {INIT_SIZE_OFFSET}")
    data[INIT_SIZE_OFFSET : INIT_SIZE_OFFSET + len(load)] = b"\xBE\xFF\xFF\x90\x90"
    dest.write_bytes(bytes(data))


def refuses(source, dest):
    data = bytearray(source.read_bytes())
    probe = bytes(data[REFUSES_SWITCH_OFFSET : REFUSES_SWITCH_OFFSET + 4])
    if probe != b"\xA8\x01\x75\x29":  # test al,1 / jnz ok_32
        raise SystemExit(f"no mode-switch entry at {REFUSES_SWITCH_OFFSET}")
    data[REFUSES_SWITCH_OFFSET : REFUSES_SWITCH_OFFSET + 2] = b"\xF9\xCB"  # stc/retf
    dest.write_bytes(bytes(data))


def request_many_descriptors(source, dest):
    data = bytearray(source.read_bytes())
    needle = b"\xb9\x02\x00"  # mov cx, 2
    idx = data.find(needle)
    if idx < 0:
        raise SystemExit(f"no `mov cx, 2` in {source.name}")
    data[idx : idx + 3] = b"\xb9\x80\x00"  # mov cx, 128
    dest.write_bytes(bytes(data))


def consumer_cargo_home():
    home = os.environ.get("CARGO_HOME")
    if home:
        return pathlib.Path(home)
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


def free_port():
    probe = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    probe.bind(("127.0.0.1", 0))
    port = probe.getsockname()[1]
    probe.close()
    return port


def capture(mount, port, commands, wanted, exe="hello"):
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind(("127.0.0.1", port))
    server.listen(1)

    command = [
        DOSBOX,
        "--noprimaryconf",
        "--nolocalconf",
        "--set",
        f"serial1=nullmodem server:127.0.0.1 port:{port}",
        "--set",
        "core=dynamic",
        "--set",
        "mouse mouse_capture=nomouse",
        "-c",
        f"mount c {mount}",
        "-c",
        "c:",
    ]
    for line in [*commands, exe]:
        command += ["-c", line]
    log = tempfile.NamedTemporaryFile(prefix="dosbox-", suffix=".log", delete=False)
    env = dict(os.environ)
    env.setdefault("SDL_VIDEODRIVER", "offscreen")
    env.setdefault("SDL_AUDIODRIVER", "dummy")
    proc = subprocess.Popen(command, env=env, stdout=log, stderr=subprocess.STDOUT)

    server.settimeout(CONNECT_TIMEOUT)
    try:
        conn, _ = server.accept()
    except socket.timeout:
        conn = None
    server.close()

    captured = b""
    if conn is not None:
        conn.settimeout(READ_TIMEOUT)
        end = time.time() + RUN_TIMEOUT
        while time.time() < end:
            try:
                chunk = conn.recv(4096)
            except socket.timeout:
                if proc.poll() is not None:
                    break
                continue
            if not chunk:
                break
            captured += chunk
            if wanted in captured:
                break
        conn.close()

    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
    log.close()

    output = pathlib.Path(log.name).read_text(errors="replace")
    pathlib.Path(log.name).unlink()
    if output.strip():
        print("dosbox output:")
        print(output.rstrip())
    return captured


def check(name, captured, wanted):
    payload = captured[len(HANDSHAKE) :] if captured.startswith(HANDSHAKE) else captured
    print(f"[{name}] captured {len(captured)} bytes: {captured[:200]!r}")

    if not captured.startswith(HANDSHAKE):
        print(
            f"FAIL [{name}]: COM1 capture did not begin with the nullmodem handshake",
            file=sys.stderr,
        )
        return 1
    if wanted not in payload:
        print(f"FAIL [{name}]: COM1 never received {wanted!r}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    if DOSBOX is None:
        raise SystemExit("dosbox or dosbox-staging not found on PATH")
    if shutil.which("cargo") is None:
        raise SystemExit("cargo not found on PATH")

    build()

    work = REPO / "target/com1-marker"
    shutil.rmtree(work, ignore_errors=True)
    work.mkdir(parents=True)
    shutil.copy(cwsdpmi(work), work / "CWSDPMI.EXE")
    without_32bit(work / "CWSDPMI.EXE", work / f"{NO_32BIT}.EXE")
    hungry(work / "CWSDPMI.EXE", work / f"{HUNGRY}.EXE")
    refuses(work / "CWSDPMI.EXE", work / f"{REFUSES}.EXE")
    shutil.copy(EXE, work / "HELLO.EXE")
    request_many_descriptors(EXE, work / "MANY.EXE")

    failures = 0
    for entry in SCENARIOS:
        if len(entry) == 4:
            name, commands, wanted, exe = entry
        else:
            name, commands, wanted = entry
            exe = "hello"
        captured = capture(work, free_port(), commands, wanted, exe)
        failures |= check(name, captured, wanted)
    sys.exit(failures)