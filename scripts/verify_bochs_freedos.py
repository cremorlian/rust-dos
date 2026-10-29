"""Run hello on Bochs + FreeDOS + CWSDPMI and assert the serial transcript.

It shares examples/hello/target/i486-dos/debug/hello.exe with the other DOS e2e lanes, so run it
through scripts/check.py rather than concurrently with the other e2e scripts.

FreeDOS and CWSDPMI are test fixtures only. They are downloaded into target/,
pinned by checksum, and never enter the toolchain or consumer output.
"""

import pathlib
import shutil
import subprocess
import sys
import time

from freedos_lane import EXPECTED, RUN_TIMEOUT, prepare_lane

REPO = pathlib.Path(__file__).resolve().parent.parent
WORK = REPO / "target/bochs-freedos"
LANE = WORK / "lane.img"
SERIAL = WORK / "serial.out"
CONFIG = WORK / "bochsrc.ini"
BOCHS = shutil.which("bochs")
BIOS = pathlib.Path("/usr/share/bochs/BIOS-bochs-latest")
VGABIOS = pathlib.Path("/usr/share/bochs/VGABIOS-lgpl-latest.bin")

BOCHSRC = (
    "config_interface: textconfig\r\n"
    "display_library: nogui\r\n"
    "memory: host=32, guest=32\r\n"
    f"romimage: file={BIOS}, options=fastboot\r\n"
    f"vgaromimage: file={VGABIOS}\r\n"
    "floppya: 1_44=lane.img, status=inserted\r\n"
    "boot: floppy\r\n"
    "cpu: model=i486dx4, ips=10000000\r\n"
    "clock: sync=realtime, time0=1\r\n"
    "log: bochs.log\r\n"
    "com1: enabled=1, mode=file, dev=serial.out\r\n"
)


def run_lane():
    if SERIAL.exists():
        SERIAL.unlink()
    lock = LANE.with_name(LANE.name + ".lock")
    lock.unlink(missing_ok=True)
    CONFIG.write_text(BOCHSRC)
    log = WORK / "bochs.out"

    with log.open("wb") as log_file:
        proc = subprocess.Popen(
            [BOCHS, "-f", str(CONFIG), "-q"],
            cwd=WORK,
            stdout=log_file,
            stderr=subprocess.STDOUT,
        )
        try:
            deadline = time.time() + RUN_TIMEOUT
            while time.time() < deadline:
                if SERIAL.exists() and b"DONE" in SERIAL.read_bytes():
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
            print("bochs output:", file=sys.stderr)
            print(output.rstrip(), file=sys.stderr)
        return 1

    print(f"PASS: serial = {captured!r}")
    return 0


def main():
    if BOCHS is None:
        raise SystemExit("bochs not found on PATH")
    for binary in (BIOS, VGABIOS):
        if not binary.exists():
            raise SystemExit(f"{binary} is missing")
    for tool in ("mcopy", "mdel"):
        if shutil.which(tool) is None:
            raise SystemExit(f"{tool} not found on PATH")

    prepare_lane(WORK)
    return run_lane()


if __name__ == "__main__":
    sys.exit(main())
