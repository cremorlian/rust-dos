"""CWSDPMI DPMI host test fixture."""

import hashlib
import pathlib
import zipfile

from fixtures import download

CWSDPMI_SHA256 = "2de899fecaa90632b8b9bdfc0305cb0375e59ae252c37e32d06c1ed3f98a8f44"
CWSDPMI_SOURCES = (
    (
        "https://www.delorie.com/pub/djgpp/current/v2misc/csdpmi7b.zip",
        "deacda0488e1cdd7c4a9f32fab45662b34c0ed6b2d7d4d13bc07041b62004a8c",
        "bin/CWSDPMI.EXE",
    ),
    (
        "http://www.ibiblio.org/pub/micro/pc-stuff/freedos/files/repositories"
        "/1.4/tools/cwsdpmi.zip",
        "6ae65336f780e54dee889b5ac865992cf72738717aff5565ef84ccba3459e141",
        "BIN/CWSDPMI.EXE",
    ),
)


def cwsdpmi(cache):
    zip_path = cache / "cwsdpmi.zip"
    errors = []
    for url, sha256, member in CWSDPMI_SOURCES:
        try:
            download(url, sha256, zip_path)
        except SystemExit as error:
            errors.append(str(error))
            continue

        with zipfile.ZipFile(zip_path) as archive:
            extracted = pathlib.Path(archive.extract(member, cache))

        digest = hashlib.sha256(extracted.read_bytes()).hexdigest()
        if digest != CWSDPMI_SHA256:
            errors.append(
                f"{url}: CWSDPMI.EXE sha256 {digest}, expected {CWSDPMI_SHA256}"
            )
            continue
        return extracted

    raise SystemExit("could not fetch CWSDPMI:\n" + "\n".join(errors))
