#!/usr/bin/env python3
"""Measure macOS UI CPU usage with isolated QA settings and generated images.

Use a binary with debug assertions (the QA isolation hooks are absent in release).
CPU is process CPU time / wall time; 100% represents one logical core.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import platform
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent


def process_time(pid):
    value = subprocess.check_output(
        ["ps", "-p", str(pid), "-o", "time="], text=True
    ).strip()
    minutes, seconds = value.rsplit(":", 1)
    return int(minutes) * 60 + float(seconds)


def measure(binary, scene, directory, images, seconds):
    config = directory / scene
    config.mkdir()
    (config / "settings.json").write_text(json.dumps({
        "version": 1, "theme": "light", "language": "en"
    }))
    env = {k: v for k, v in os.environ.items()
           if not k.startswith("TINYEASY_QA_")
           and k not in ("TINIFY_API_KEY", "TINIFY_API_KEYS")}
    env.update(TINYEASY_QA_DIR=str(config), TINYEASY_QA_SCENE=scene)
    if scene == "gallery":
        env["TINYEASY_QA_IMPORT"] = str(images)
    process = subprocess.Popen([str(binary)], cwd=ROOT, env=env,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        time.sleep(3)
        if process.poll() is not None:
            raise RuntimeError(f"{scene}: application exited before measurement")
        cpu_start = process_time(process.pid)
        start = time.monotonic()
        time.sleep(seconds)
        cpu_end = process_time(process.pid)
        elapsed = time.monotonic() - start
        rss = int(subprocess.check_output(
            ["ps", "-p", str(process.pid), "-o", "rss="], text=True
        ).strip())
        return {"scene": scene, "seconds": round(elapsed, 3),
                "cpu_percent": round(100 * (cpu_end - cpu_start) / elapsed, 2),
                "rss_kib": rss}
    finally:
        if process.poll() is None:
            process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--seconds", type=float, default=5)
    parser.add_argument("--scene", action="append", choices=["empty", "settings", "gallery"])
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if platform.system() != "Darwin":
        parser.error("This measurement uses macOS ps CPU-time formatting")
    if args.seconds <= 0:
        parser.error("--seconds must be positive")
    binary = args.binary.resolve()
    if not binary.is_file() or b"TINYEASY_QA_DIR" not in binary.read_bytes():
        parser.error("Use a QA-enabled binary; ordinary release builds cannot isolate credentials")
    spec = importlib.util.spec_from_file_location("render_ui", ROOT / "scripts/render-ui.py")
    render = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(render)
    results = []
    with tempfile.TemporaryDirectory(prefix="tinyeasy-measure-ui-") as temporary:
        directory = Path(temporary)
        images = directory / "images"
        images.mkdir()
        for index in range(18):
            render.png(images / f"Sample {index}.png", index)
        for scene in args.scene or ["empty", "settings", "gallery"]:
            result = measure(args.binary.resolve(), scene, directory, images, args.seconds)
            results.append(result)
            print(json.dumps(result), flush=True)
    report = {"platform": platform.platform(), "binary": str(args.binary.resolve()),
              "warmup_seconds": 3, "results": results}
    if args.output:
        args.output.write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
