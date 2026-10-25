"""Run hello on qemu + FreeDOS + CWSDPMI and assert the serial transcript.

It shares examples/hello/hello.exe with the DOSBox lane, so run it through
scripts/check.py rather than concurrently with the other e2e scripts.

FreeDOS and CWSDPMI are test fixtures only. They are downloaded into target/,
pinned by checksum, and never enter the toolchain or consumer output.
"""

import hashlib
import pathlib
import shutil
import subprocess
import sys
import time
import urllib.request
import zipfile

from verify_com1_marker import cwsdpmi

REPO = pathlib.Path(__file__).resolve().parent.parent
EXE = REPO / "examples/hello/hello.exe"
WORK = REPO / "target/qemu-freedos"
LANE = WORK / "lane.img"
SERIAL = WORK / "serial.out"
QEMU = shutil.which("qemu-system-i386")

FREEDOS_URL = "https://download.freedos.org/1.4/FD14-FloppyEdition.zip"
FREEDOS_SHA256 = "45b1fa7c52dd996c3bfa5e352ffcd410781b952a6ad629f15a4c9ec4bbaefc5a"
FREEDOS_MEMBER = "144m/x86BOOT.img"

EXPECTED = b"hello, dos\r\nZERO\r\nDONE\r\n"

AUTOEXEC = (
    "@echo off\r\n"
    "cwsdpmi\r\n"
    "hello > com1\r\n"
    "if not errorlevel 1 echo ZERO>com1\r\n"
    "echo DONE>com1\r\n"
)

FDCONFIG = (
    "!COUNTRY=001,858:\\FREEDOS\\BIN\\COUNTRY.SYS\r\n"
    "!LASTDRIVE=Z\r\n"
    "!BUFFERS=20\r\n"
    "!FILES=40\r\n"
    "SHELL=\\FREEDOS\\BIN\\COMMAND.COM \\FREEDOS\\BIN /E:2048 /P\r\n"
)

RUN_TIMEOUT = 60.0


def run(command):
    result = subprocess.run(command, capture_output=True, text=True)
    if result.returncode != 0:
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        raise SystemExit(f"failed: {' '.join(command)}")
    return result


def download(url, sha256, dest):
    if dest.exists() and hashlib.sha256(dest.read_bytes()).hexdigest() == sha256:
        return dest

    dest.parent.mkdir(parents=True, exist_ok=True)
    try:
        with urllib.request.urlopen(url, timeout=120) as response:
            dest.write_bytes(response.read())
    except OSError as error:
        raise SystemExit(f"could not fetch {url}: {error}")

    digest = hashlib.sha256(dest.read_bytes()).hexdigest()
    if digest != sha256:
        raise SystemExit(f"{url} has sha256 {digest}, expected {sha256}")
    return dest


def prepare_lane():
    if not EXE.exists():
        raise SystemExit(f"{EXE} is missing; run the build first")

    WORK.mkdir(parents=True, exist_ok=True)
    downloads = WORK / "downloads"
    downloads.mkdir(exist_ok=True)

    freedos = download(FREEDOS_URL, FREEDOS_SHA256, downloads / "FD14-FloppyEdition.zip")
    with zipfile.ZipFile(freedos) as archive:
        LANE.write_bytes(archive.read(FREEDOS_MEMBER))

    cwsd = cwsdpmi(downloads)
    autoexec = WORK / "AUTOEXEC.BAT"
    fdconfig = WORK / "FDCONFIG.SYS"
    autoexec.write_bytes(AUTOEXEC.encode("ascii"))
    fdconfig.write_bytes(FDCONFIG.encode("ascii"))

    run(["mcopy", "-o", "-i", str(LANE), str(cwsd), "::CWSDPMI.EXE"])
    run(["mcopy", "-o", "-i", str(LANE), str(EXE), "::HELLO.EXE"])
    run(["mcopy", "-o", "-i", str(LANE), str(autoexec), "::AUTOEXEC.BAT"])
    run(["mcopy", "-o", "-i", str(LANE), str(fdconfig), "::FDCONFIG.SYS"])
    subprocess.run(
        ["mdel", "-i", str(LANE), "::FDAUTO.BAT"],
        capture_output=True,
        text=True,
    )


def run_lane():
    if SERIAL.exists():
        SERIAL.unlink()
    log = WORK / "qemu.log"

    with log.open("wb") as log_file:
        proc = subprocess.Popen(
            [
                QEMU,
                "-display",
                "none",
                "-monitor",
                "none",
                "-serial",
                f"file:{SERIAL}",
                "-fda",
                str(LANE),
                "-boot",
                "a",
                "-m",
                "16",
                "-no-reboot",
            ],
            stdout=log_file,
            stderr=subprocess.STDOUT,
        )
        try:
            deadline = time.time() + RUN_TIMEOUT
            while time.time() < deadline:
                if SERIAL.exists():
                    captured = SERIAL.read_bytes()
                    if captured == EXPECTED or b"DONE" in captured:
                        break
                if proc.poll() is not None:
                    break
                time.sleep(0.05)
        finally:
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait()

    captured = SERIAL.read_bytes() if SERIAL.exists() else b""
    if captured != EXPECTED:
        print(f"FAIL: serial expected exactly {EXPECTED!r}, got {captured!r}", file=sys.stderr)
        output = log.read_text(errors="replace")
        if output.strip():
            print("qemu output:", file=sys.stderr)
            print(output.rstrip(), file=sys.stderr)
        return 1

    print(f"PASS: serial = {captured!r}")
    return 0


def main():
    if QEMU is None:
        raise SystemExit("qemu-system-i386 not found on PATH")
    for tool in ("mcopy", "mdel"):
        if shutil.which(tool) is None:
            raise SystemExit(f"{tool} not found on PATH")

    prepare_lane()
    return run_lane()


if __name__ == "__main__":
    sys.exit(main())
