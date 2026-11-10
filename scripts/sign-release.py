"""Sign FolderLens metadata and assets with its dedicated Oak-style Actions secret."""

import base64
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def main():
    version, directory, tool = sys.argv[1:]
    assets = Path(directory)
    secret = os.environ.get("SPARKLE_PRIVATE_KEY", "").strip()
    if not secret:
        raise SystemExit("Missing FolderLens SPARKLE_PRIVATE_KEY; refusing unsigned publication")
    archive = assets / f"FolderLens-{version}-macos-arm64.zip"
    with archive.open("rb") as source:
        digest = hashlib.sha256()
        for chunk in iter(lambda: source.read(1048576), b""):
            digest.update(chunk)
    manifest = {
        "schema": 1,
        "version": version,
        "tag": f"v{version}",
        "channel": "prerelease" if "-" in version else "stable",
        "expires": int(time.time()) + 365 * 86400,
        "asset": archive.name,
        "size": archive.stat().st_size,
        "sha256": digest.hexdigest(),
    }
    metadata = assets / "update.json"
    metadata.write_text(json.dumps(manifest, sort_keys=True, separators=(",", ":")) + "\n")
    for path in [metadata, archive, assets / f"FolderLens-{version}-macos-arm64.dmg"]:
        result = subprocess.run(
            [tool, "--ed-key-file", "-", "-p", str(path)],
            input=secret + "\n", text=True, capture_output=True,
        )
        if result.returncode:
            raise SystemExit("FolderLens publisher signing failed; no private diagnostics were logged")
        signature = base64.b64decode(result.stdout.strip(), validate=True)
        if len(signature) != 64:
            raise SystemExit("Invalid publisher signature output")
        path.with_name(path.name + ".sig").write_bytes(signature)


if __name__ == "__main__":
    main()
