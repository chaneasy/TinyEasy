#!/usr/bin/env python3
"""Package only the Rust release binary and brand icon on the native build host."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import plistlib
import shutil
import subprocess
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parent.parent
VERSION = "2.0.0"

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--skip-build", action="store_true")
    args = parser.parse_args()
    if not args.skip_build:
        cargo = shutil.which("cargo") or str(Path.home() / ".cargo" / "bin" / ("cargo.exe" if platform.system() == "Windows" else "cargo"))
        subprocess.run([cargo, "build", "--release", "--locked"], cwd=ROOT, check=True)
    system = platform.system()
    arch = platform.machine().lower()
    arch = {"arm64": "aarch64", "amd64": "x86_64"}.get(arch, arch)
    output = ROOT / "release" / "rust"
    output.mkdir(parents=True, exist_ok=True)
    executable = ROOT / "target" / "release" / ("tinyeasy.exe" if system == "Windows" else "tinyeasy")
    if not executable.is_file():
        raise SystemExit("Rust release binary missing; run cargo build --release --locked")
    stem = f"TinyEasy-{VERSION}-{system.lower()}-{arch}"
    with tempfile.TemporaryDirectory(prefix="tinyeasy-package-") as directory:
        stage = Path(directory)
        if system == "Darwin":
            app = stage / "TinyEasy.app"
            (app / "Contents" / "MacOS").mkdir(parents=True)
            (app / "Contents" / "Resources").mkdir()
            shutil.copy2(executable, app / "Contents" / "MacOS" / "tinyeasy")
            shutil.copy2(ROOT / "assets" / "app-icons" / "icon.icns", app / "Contents" / "Resources" / "icon.icns")
            with (app / "Contents" / "Info.plist").open("wb") as file:
                plistlib.dump({
                    "CFBundleName": "TinyEasy", "CFBundleDisplayName": "TinyEasy",
                    "CFBundleIdentifier": "com.tinyeasy.imagecompressor.rust",
                    "CFBundleExecutable": "tinyeasy", "CFBundleIconFile": "icon.icns",
                    "CFBundlePackageType": "APPL", "CFBundleShortVersionString": VERSION,
                    "CFBundleVersion": VERSION, "LSMinimumSystemVersion": "11.0",
                    "NSHighResolutionCapable": True,
                    "NSHumanReadableCopyright": "TinyEasy · MIT",
                }, file)
            subprocess.run(["codesign", "--force", "--sign", "-", str(app)], check=True)
            # Ad-hoc signing is local validation, not notarized distribution.
            preserved_app = output / "TinyEasy.app"
            if preserved_app.exists():
                shutil.rmtree(preserved_app)
            shutil.copytree(app, preserved_app)
            (stage / "Applications").symlink_to("/Applications")
            artifact = output / (stem + ".dmg")
            subprocess.run(["hdiutil", "create", "-ov", "-format", "UDZO", "-volname", "TinyEasy", "-srcfolder", str(stage), str(artifact)], check=True)
        elif system == "Windows":
            artifact = output / (stem + ".zip")
            with zipfile.ZipFile(artifact, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
                archive.write(executable, "TinyEasy/tinyeasy.exe")
        elif system == "Linux":
            artifact = output / (stem + ".tar.gz")
            with tarfile.open(artifact, "w:gz", compresslevel=9) as archive:
                archive.add(executable, "TinyEasy/tinyeasy")
                archive.add(ROOT / "assets" / "app-icons" / "icon-256.png", "TinyEasy/tinyeasy.png")
        else:
            raise SystemExit(f"Unsupported host: {system}")
    digest = hashlib.sha256()
    with artifact.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    manifest = {
        "version": VERSION, "platform": system, "architecture": arch,
        "artifact": artifact.name, "artifact_bytes": artifact.stat().st_size,
        "binary_bytes": executable.stat().st_size,
        "sha256": digest.hexdigest(),
        "compression": {"Darwin": "UDZO", "Windows": "ZIP deflate 9", "Linux": "gzip 9"}[system],
    }
    (output / (stem + ".json")).write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest, indent=2))

if __name__ == "__main__":
    main()
