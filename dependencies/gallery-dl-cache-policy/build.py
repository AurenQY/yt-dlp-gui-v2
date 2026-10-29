"""Locked, project-local standalone dependency build for ADR-0002.

Run with target/gallery-dl-cache-policy/venv/Scripts/python.exe -B.
Use --record-bundle only after reviewing the resulting validation artifact.
"""
import argparse
import difflib
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import urllib.request
import zipfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
WORK = ROOT / "target/gallery-dl-cache-policy"
LOCK = HERE / "runtime.lock.json"


def digest(path):
    with open(path, "rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def checked(path):
    path = Path(path).resolve()
    if not path.is_relative_to(ROOT):
        raise ValueError("Build paths must remain inside the project")
    return path


def write_json(path, data):
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8", newline="\n")


def apply_patch(text, patch):
    source, result, position = text.splitlines(True), [], 0
    lines = patch.splitlines(True)
    cursor = 2
    while cursor < len(lines):
        match = re.match(r"@@ -(\d+)(?:,\d+)? \+\d+(?:,\d+)? @@", lines[cursor])
        if not match:
            raise ValueError("Unexpected unified patch syntax")
        start = int(match[1]) - 1
        result.extend(source[position:start])
        position, cursor = start, cursor + 1
        while cursor < len(lines) and not lines[cursor].startswith("@@"):
            line = lines[cursor]
            if line[:1] in (" ", "-"):
                if position >= len(source) or source[position] != line[1:]:
                    raise ValueError("Patch context mismatch")
                position += 1
            if line[:1] in (" ", "+"):
                result.append(line[1:])
            cursor += 1
    result.extend(source[position:])
    return "".join(result)


def prepare(lock):
    archive = WORK / "upstream.zip"
    if not archive.exists():
        urllib.request.urlretrieve(f"https://codeload.github.com/mikf/gallery-dl/zip/{lock['upstream_commit']}", archive)
    if digest(archive) != lock["archive_sha256"]:
        raise ValueError("Upstream archive hash mismatch")
    source = WORK / "source"
    # Verify every source file, including files not changed by this patch set.
    with zipfile.ZipFile(archive) as entries:
        for member in entries.infolist():
            relative = Path(*Path(member.filename).parts[1:])
            if not relative.parts:
                continue
            destination = checked(source / relative)
            if not destination.is_relative_to(source.resolve()):
                raise ValueError("Unsafe upstream archive member")
            if member.is_dir():
                destination.mkdir(parents=True, exist_ok=True)
                continue
            data = entries.read(member)
            expected = hashlib.sha256(data).hexdigest()
            policy = lock["source_files"].get(relative.as_posix())
            if destination.exists() and digest(destination) not in {expected, (policy or {}).get("after_sha256")}:
                raise ValueError(f"Modified source outside locked patch: {relative}")
            destination.parent.mkdir(parents=True, exist_ok=True)
            if not destination.exists():
                destination.write_bytes(data)
    for relative, policy in lock["source_files"].items():
        path, patch = source / relative, HERE / policy["patch"]
        if digest(patch) != policy["patch_sha256"]:
            raise ValueError("Patch hash mismatch")
        if digest(path) == policy["before_sha256"]:
            path.write_text(apply_patch(path.read_text(encoding="utf-8"), patch.read_text(encoding="utf-8")), encoding="utf-8", newline="\n")
        if digest(path) != policy["after_sha256"]:
            raise ValueError(f"Patched source hash mismatch: {relative}")
    for name, expected in lock["input_hashes"].items():
        if digest(HERE / name) != expected:
            raise ValueError(f"Build input hash mismatch: {name}")
    shutil.copyfile(HERE / "runtime_policy.py", source / "gallery_dl/_gui_cache_policy.py")
    return source


def inventory(bundle):
    files = {p.relative_to(bundle).as_posix(): digest(p) for p in sorted(bundle.rglob("*"))
             if p.is_file() and p.name != "bundle.manifest.json"}
    canonical = "".join(f"{name}\0{value}\n" for name, value in sorted(files.items()))
    return files, hashlib.sha256(canonical.encode()).hexdigest()


def build(lock, source):
    checked(WORK / "artifacts")
    checked(WORK / "build")
    environment = dict(os.environ)
    environment.update({"TEMP": str(WORK / "temp"), "TMP": str(WORK / "temp"),
                        "PIP_CACHE_DIR": str(WORK / "pip-cache"),
                        "PYINSTALLER_CONFIG_DIR": str(WORK / "pyinstaller-cache"),
                        "PYTHONDONTWRITEBYTECODE": "1"})
    (WORK / "temp").mkdir(exist_ok=True)
    subprocess.run([sys.executable, "-B", "-m", "pip", "install", "--require-hashes", "--only-binary=:all:",
                    "-r", str(HERE / "requirements-build.lock")], check=True, env=environment)
    interpreter_data = []
    for relative, expected in lock.get("interpreter_data", {}).items():
        path = Path(sys.base_prefix) / relative
        if digest(path) != expected:
            raise ValueError(f"Interpreter data hash mismatch: {relative}")
        interpreter_data.extend(["--add-data", f"{path}{os.pathsep}."])
    subprocess.run([sys.executable, "-B", "-m", "PyInstaller", "--noconfirm", "--onedir", "--console",
                    "--name", "gallery-dl", "--paths", str(source),
                    "--additional-hooks-dir", str(source / "scripts"),
                    "--hidden-import", "gallery_dl._gui_cache_policy", "--collect-all", "yt_dlp",
                    "--collect-all", "yt_dlp_ejs", "--copy-metadata", "yt-dlp",
                    "--distpath", str(WORK / "artifacts/windows-x86_64"),
                    "--workpath", str(WORK / "build"), "--specpath", str(WORK / "build"),
                    *interpreter_data, str(HERE / "entrypoint.py")], check=True, env=environment, cwd=ROOT)
    bundle = WORK / "artifacts/windows-x86_64/gallery-dl"
    # Complete modified gallery source and exact recipe/locks remain beside the binary.
    shutil.copytree(source, bundle / "source/gallery-dl", dirs_exist_ok=True,
                    ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
    shutil.copytree(HERE, bundle / "source/cache-policy", dirs_exist_ok=True,
                    ignore=shutil.ignore_patterns("runtime.lock.json", "__pycache__", "*.pyc"))
    source_lock = dict(lock, bundles=[])
    write_json(bundle / "source/cache-policy/runtime.lock.json", source_lock)
    shutil.copyfile(HERE / "LICENSE.gallery-dl", bundle / "LICENSE.gallery-dl")
    notices = bundle / "notices"
    notices.mkdir(exist_ok=True)
    for name in ("LICENSE.txt", "LICENSE"):
        license_path = Path(sys.base_prefix) / name
        if license_path.is_file():
            shutil.copyfile(license_path, notices / ("Python-" + name))
    packages = {}
    for distribution in importlib.metadata.distributions():
        name = distribution.metadata["Name"]
        packages[name] = distribution.version
        for file in distribution.files or ():
            if any(token in file.name.lower() for token in ("license", "copying", "notice")):
                origin = distribution.locate_file(file)
                if origin.is_file():
                    destination = notices / name / str(file).replace("..", "_").replace(":", "_")
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(origin, destination)
    write_json(bundle / "build-environment.json", {"python": sys.version, "executable": str(Path(sys.executable).relative_to(ROOT)), "packages": packages})
    (bundle / "CHANGES.cache-policy.txt").write_text(
        "Locally modified gallery-dl; not an official upstream binary.\n"
        "Playlist work/finalization and per-artifact completion follow ADR-0002.\n"
        "Full modified source, patches, locks, recipe, and upstream license are in source/.\n", encoding="utf-8")
    files, tree_hash = inventory(bundle)
    manifest = {"schema": 1, "bundle_id": f"gallery-dl-1.32.18-cache1-{tree_hash[:16]}",
                "upstream_commit": lock["upstream_commit"], "upstream_version": lock["upstream_version"],
                "patch_revision": lock["patch_revision"], "yt_dlp_version": lock["yt_dlp_version"],
                "platform": lock["platform"], "entrypoint": "gallery-dl.exe",
                "protocol": "yt-dlp-gui-cache-policy/v1", "inventory_sha256": tree_hash, "files": files}
    write_json(bundle / "bundle.manifest.json", manifest)
    return bundle, manifest


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--prepare-only", action="store_true")
    parser.add_argument("--record-bundle", action="store_true")
    args = parser.parse_args()
    WORK.mkdir(parents=True, exist_ok=True)
    checked(WORK)
    lock = json.loads(LOCK.read_text(encoding="utf-8"))
    if sys.version.split()[0] != lock["python_version"]:
        raise ValueError("Build interpreter does not match the lock")
    source = prepare(lock)
    if args.prepare_only:
        print(source)
        return
    bundle, manifest = build(lock, source)
    if args.record_bundle:
        lock["bundles"] = [{key: manifest[key] for key in ("bundle_id", "inventory_sha256", "platform")}]
        LOCK.write_text(json.dumps(lock, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(json.dumps({"bundle": str(bundle), "bundle_id": manifest["bundle_id"], "inventory_sha256": manifest["inventory_sha256"]}))


if __name__ == "__main__":
    main()
