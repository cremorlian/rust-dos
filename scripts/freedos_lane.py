"""FreeDOS lane image setup for the DOS e2e lanes."""

import pathlib
import subprocess
import sys
import zipfile

from cwsdpmi import cwsdpmi
from fixtures import download

REPO = pathlib.Path(__file__).resolve().parent.parent
EXE = REPO / "examples/hello/hello.exe"

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


def prepare_lane(work):
    if not EXE.exists():
        raise SystemExit(f"{EXE} is missing; run the build first")

    work.mkdir(parents=True, exist_ok=True)
    downloads = work / "downloads"
    downloads.mkdir(exist_ok=True)

    freedos = download(
        FREEDOS_URL, FREEDOS_SHA256, downloads / "FD14-FloppyEdition.zip"
    )
    lane = work / "lane.img"
    with zipfile.ZipFile(freedos) as archive:
        lane.write_bytes(archive.read(FREEDOS_MEMBER))

    cwsd = cwsdpmi(downloads)
    autoexec = work / "AUTOEXEC.BAT"
    fdconfig = work / "FDCONFIG.SYS"
    autoexec.write_bytes(AUTOEXEC.encode("ascii"))
    fdconfig.write_bytes(FDCONFIG.encode("ascii"))

    run(["mcopy", "-o", "-i", str(lane), str(cwsd), "::CWSDPMI.EXE"])
    run(["mcopy", "-o", "-i", str(lane), str(EXE), "::HELLO.EXE"])
    run(["mcopy", "-o", "-i", str(lane), str(autoexec), "::AUTOEXEC.BAT"])
    run(["mcopy", "-o", "-i", str(lane), str(fdconfig), "::FDCONFIG.SYS"])
    subprocess.run(
        ["mdel", "-i", str(lane), "::FDAUTO.BAT"],
        capture_output=True,
        text=True,
    )

    return lane
