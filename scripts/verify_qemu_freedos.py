"""Run hello on qemu + FreeDOS + CWSDPMI and assert the serial transcript.

It shares examples/hello/hello.exe with the other DOS e2e lanes, so run it
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
WORK = REPO / "target/qemu-freedos"
LANE = WORK / "lane.img"
SERIAL = WORK / "serial.out"
QEMU = shutil.which("qemu-system-i386")


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

    prepare_lane(WORK)
    return run_lane()


if __name__ == "__main__":
    sys.exit(main())
