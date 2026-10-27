"""Run the local gate in the same order as CI.

The e2e scripts share the example's build target and hello.exe, so run them
through this wrapper rather than concurrently.
"""

import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent

STEPS = [
    ("format", ["cargo", "fmt", "--all", "--", "--check"]),
    ("clippy", ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]),
    ("test", ["cargo", "test", "--workspace"]),
    ("load-image e2e", [sys.executable, "scripts/verify_load_image.py"]),
    ("COM1 marker e2e", [sys.executable, "scripts/verify_com1_marker.py"]),
    ("qemu lane e2e", [sys.executable, "scripts/verify_qemu_freedos.py"]),
    ("bochs lane e2e", [sys.executable, "scripts/verify_bochs_freedos.py"]),
    ("dosemu2 native e2e", [sys.executable, "scripts/verify_dosemu2.py"]),
]


def main():
    for name, command in STEPS:
        print(f"==> {name}", flush=True)
        try:
            subprocess.run(command, cwd=REPO, check=True)
        except subprocess.CalledProcessError as error:
            return error.returncode
    return 0


if __name__ == "__main__":
    sys.exit(main())
