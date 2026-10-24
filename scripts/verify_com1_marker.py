"""Run the example under DOSBox and assert the stub's real-mode COM1 marker.

DOSBox has no DPMI host, so CWSDPMI is downloaded and loaded first. COM1 is
captured over nullmodem: Python binds and listens before DOSBox starts, and
DOSBox is the TCP client, because a program that writes its marker once loses
it when the reader attaches late.

Run through scripts/check.py. Do not run this concurrently with
verify_load_image.py: both share the example's build target and hello.exe.
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

# A tiny DPMI host built from scripts/fakehost.asm and staged as a .COM. It is
# a host by the spec's contract, just one whose behaviour we choose. Builds
# share that source and differ only by the behaviour a scenario requests.
FAKEHOST = "fakehost"
REFUSE = "refuse"
NO32 = "no32bit"
FAKE_HUNGRY = "hungry"
FAKE_NODESC = "nodesc"
FAKE_NORIGHTS = "norights"
FAKE_CODERIGHTS = "badcode"
FAKE_NOBASE = "nobase"
FAKE_NOLIMIT = "nolimit"
FAKE_NOPRIV = "nopriv"

SCENARIOS = [
    ("host loaded", ["cwsdpmi"], b"hello, dos\r\n"),
    ("no host", [], b"stub rm\r\nrust-dos: no DPMI host found\r\n"),
    (
        "host without 32-bit",
        [NO32],
        b"stub rm\r\nrust-dos: warning: host reports no 32-bit support\r\n"
        b"stub pm\r\nhello, dos\r\n",
    ),
    (
        "host wants more memory than DOS has",
        [FAKE_HUNGRY],
        b"stub rm\r\nrust-dos: insufficient memory for the DPMI host\r\n",
    ),
    (
        "host needs no private memory",
        [FAKE_NOPRIV],
        b"stub rm\r\nstub pm\r\nhello, dos\r\n",
    ),
    (
        "host refuses mode switch",
        [REFUSE],
        b"stub rm\r\nrust-dos: DPMI host refused mode switch\r\n",
    ),
    (
        "fake host boots the client",
        [FAKEHOST],
        b"stub rm\r\nstub pm\r\nhello, dos\r\n",
    ),
    (
        "no LDT descriptors",
        [FAKE_NODESC],
        b"stub rm\r\nstub pm\r\nrust-dos: no LDT descriptors available\r\n",
    ),
    (
        "host rejects descriptor rights",
        [FAKE_NORIGHTS],
        b"stub rm\r\nstub pm\r\nrust-dos: DPMI host rejected descriptor rights\r\n",
    ),
    (
        "host rejects code descriptor rights only",
        [FAKE_CODERIGHTS],
        b"stub rm\r\nstub pm\r\nrust-dos: DPMI host rejected descriptor rights\r\n",
    ),
    (
        "host rejects descriptor base",
        [FAKE_NOBASE],
        b"stub rm\r\nstub pm\r\nrust-dos: DPMI host rejected descriptor setup\r\n",
    ),
    (
        "host rejects descriptor limit",
        [FAKE_NOLIMIT],
        b"stub rm\r\nstub pm\r\nrust-dos: DPMI host rejected descriptor setup\r\n",
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


def build_fakehost(work):
    source = REPO / "scripts/fakehost.asm"
    builds = {
        FAKEHOST: [],
        REFUSE: ["-DREFUSE_SWITCH"],
        NO32: ["-DNO_32BIT"],
        FAKE_HUNGRY: ["-DHUNGRY"],
        FAKE_NODESC: ["-DNO_DESCRIPTORS"],
        FAKE_NORIGHTS: ["-DREJECT_RIGHTS"],
        FAKE_CODERIGHTS: ["-DREJECT_CODE_RIGHTS"],
        FAKE_NOBASE: ["-DREJECT_BASE"],
        FAKE_NOLIMIT: ["-DREJECT_LIMIT"],
        FAKE_NOPRIV: ["-DNO_PRIVATE"],
    }
    for name, flags in builds.items():
        dest = work / f"{name}.COM"
        run(
            ["nasm", "-f", "bin", *flags, str(source), "-o", str(dest)],
            REPO,
            os.environ.copy(),
        )
    return work / f"{FAKEHOST}.COM"


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


def capture(mount, port, commands, wanted):
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
    for line in [*commands, "hello"]:
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
    if shutil.which("nasm") is None:
        raise SystemExit("nasm not found on PATH")

    build()

    work = REPO / "target/com1-marker"
    shutil.rmtree(work, ignore_errors=True)
    work.mkdir(parents=True)
    shutil.copy(cwsdpmi(work), work / "CWSDPMI.EXE")
    build_fakehost(work)
    shutil.copy(EXE, work / "HELLO.EXE")

    failures = 0
    for name, commands, wanted in SCENARIOS:
        captured = capture(work, free_port(), commands, wanted)
        failures |= check(name, captured, wanted)
    sys.exit(failures)