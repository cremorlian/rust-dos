"""Run hello twice on dosemu2's native DPMI and assert the console transcript.

The two runs share one DOS session, proving the first exits cleanly enough to
run the second. The lane uses dosemu2's built-in FDPP/comcom64 DOS, not a
FreeDOS floppy. It shares examples/hello/hello.exe with the other DOS e2e
lanes, so run it through scripts/check.py rather than concurrently with the
other e2e scripts.
"""

import pathlib
import shutil
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
WORK = REPO / "target/dosemu2-native"
STAGE = WORK / "stage"
EXE = REPO / "examples/hello/hello.exe"
DOSEMU = shutil.which("dosemu")
RUN_TIMEOUT = 60.0

RUN_BAT = (
    "@echo off\r\n"
    "hello.exe\r\n"
    "if not errorlevel 1 echo ZERO\r\n"
    "hello.exe\r\n"
    "if not errorlevel 1 echo ZERO\r\n"
    "exitemu\r\n"
)

CONFIG = (
    '$_cpu_vm = "emulated"\n'
    '$_cpu_vm_dpmi = "emulated"\n'
)

EXPECTED = [b"hello, dos", b"ZERO", b"hello, dos", b"ZERO"]


def prepare_lane():
    if not EXE.exists():
        raise SystemExit(f"{EXE} is missing; run the build first")
    if DOSEMU is None:
        raise SystemExit("dosemu not found on PATH")

    STAGE.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(EXE, STAGE / "HELLO.EXE")
    (STAGE / "RUN.BAT").write_bytes(RUN_BAT.encode("ascii"))
    config = WORK / "dosemu.conf"
    config.write_text(CONFIG, encoding="ascii")
    return config


def run_lane(config):
    command = [
        DOSEMU,
        "-n",
        "-f",
        str(config),
        "-dumb",
        str(STAGE / "RUN.BAT"),
    ]
    try:
        result = subprocess.run(
            command,
            capture_output=True,
            timeout=RUN_TIMEOUT,
        )
    except subprocess.TimeoutExpired as error:
        print(f"FAIL: dosemu timed out after {RUN_TIMEOUT}s", file=sys.stderr)
        if error.stdout:
            print(error.stdout.decode(errors="backslashreplace"), file=sys.stderr)
        if error.stderr:
            print(error.stderr.decode(errors="backslashreplace"), file=sys.stderr)
        return 1

    lines = result.stdout.replace(b"\r\n", b"\n").splitlines()
    if lines[-len(EXPECTED) :] != EXPECTED:
        print(f"FAIL: console expected trailing lines {EXPECTED!r}", file=sys.stderr)
        print(f"stdout: {result.stdout!r}", file=sys.stderr)
        print(f"stderr: {result.stderr!r}", file=sys.stderr)
        return 1

    print(f"PASS: console ends with {EXPECTED!r}")
    return 0


def main():
    return run_lane(prepare_lane())


if __name__ == "__main__":
    sys.exit(main())
