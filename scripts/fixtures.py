"""Shared download helpers for the DOS e2e fixtures."""

import hashlib
import urllib.request


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
