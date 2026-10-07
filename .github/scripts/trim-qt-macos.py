#!/usr/bin/env python3
"""Takes out of the macOS app bundle what macdeployqt adds but the app never loads.

Usage: trim-qt-macos.py <app bundle> <Qt prefix> <QML folder>

macdeployqt copies Qt's QML folders whole, so every Qt Quick Controls style, Dialogs, Effects,
Particles and LocalStorage come along, with the frameworks they link to, QtSql and QtWidgets
among them. This keeps the QML modules the app's QML imports, their plugins and the Qt plugins
in PLUGINS, then removes the frameworks nothing left links to. Qt's binaries are universal and
the app is arm64-only, so it also keeps only the arm64 code of each binary. The bundle must be
signed again afterwards.
"""

import json
import os
import re
import shutil
import struct
import subprocess
import sys
from pathlib import Path

# Qt plugin folders to keep; quick holds the QML modules' plugins, sorted out separately.
PLUGINS = {"platforms", "imageformats", "iconengines", "tls", "quick"}

ARM64 = 0x0100000C
# Load commands naming a library: LC_LOAD_DYLIB, LC_LOAD_WEAK_DYLIB, LC_REEXPORT_DYLIB,
# LC_LAZY_LOAD_DYLIB and LC_LOAD_UPWARD_DYLIB.
LINKS = {0xC, 0x80000018, 0x8000001F, 0x20, 0x80000023}


def macho(data):
    """The arm64 Mach-O image in `data` and whether `data` is universal, or (None, False)."""
    if len(data) >= 8 and struct.unpack_from(">I", data)[0] in (0xCAFEBABE, 0xCAFEBABF):
        wide = struct.unpack_from(">I", data)[0] == 0xCAFEBABF
        count = struct.unpack_from(">I", data, 4)[0]
        for index in range(count if count < 16 else 0):
            if wide:
                cpu, _, offset, size = struct.unpack_from(">IIQQ", data, 8 + index * 32)
            else:
                cpu, _, offset, size = struct.unpack_from(">IIII", data, 8 + index * 20)
            if cpu == ARM64:
                return data[offset:offset + size], True
        return None, False
    if len(data) >= 32 and struct.unpack_from("<I", data)[0] == 0xFEEDFACF:
        return data, False
    return None, False


def links(path):
    """The libraries the arm64 code in `path` loads."""
    image, _ = macho(path.read_bytes())
    if image is None:
        return []
    count = struct.unpack_from("<I", image, 16)[0]
    offset, names = 32, []
    for _ in range(count):
        command, size = struct.unpack_from("<II", image, offset)
        if command in LINKS:
            start = offset + struct.unpack_from("<I", image, offset + 8)[0]
            names.append(image[start:image.index(b"\0", start)].decode())
        offset += size
    return names


def qml_modules(qt, sources):
    """The modules, as paths under Qt's qml folder, that the app's QML imports."""
    scanner = qt / "libexec" / "qmlimportscanner"
    found = subprocess.run(
        [str(scanner), "-rootPath", str(sources), "-importPath", str(qt / "qml")],
        check=True, capture_output=True, text=True,
    ).stdout
    root = (qt / "qml").resolve()
    # The app's own module has no path: it is compiled into the app.
    return {
        Path(entry["path"]).resolve().relative_to(root)
        for entry in json.loads(found)
        if entry.get("type") == "module" and entry.get("path")
    }


def module_files(folder):
    """A QML module's files; its subfolders with a qmldir are modules of their own."""
    for entry in folder.iterdir():
        if entry.is_dir() and not entry.is_symlink():
            if not (entry / "qmldir").exists():
                yield from module_files(entry)
        else:
            yield entry


def remove(path):
    if path.is_dir() and not path.is_symlink():
        shutil.rmtree(path)
    else:
        path.unlink()


def size(folder):
    return sum(path.stat().st_size for path in folder.rglob("*") if path.is_file() and not path.is_symlink())


def main():
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    app, qt, sources = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
    contents = app / "Contents"
    before = size(app)

    # QML modules: keep the imported ones, and with them the plugins their qmldir names.
    qml = contents / "Resources" / "qml"
    keep = set()
    plugins = set()
    for module in qml_modules(qt, sources):
        folder = qml / module
        if not folder.is_dir():
            sys.exit(f"macdeployqt did not deploy the QML module {module}")
        keep.update(module_files(folder))
        for line in (folder / "qmldir").read_text().splitlines():
            words = line.split()
            if "plugin" in words[:2]:
                plugins.add(f"lib{words[words.index('plugin') + 1]}.dylib")
    for path in sorted(qml.rglob("*"), key=lambda path: -len(path.parts)):
        if path.is_symlink() or path.is_file():
            if path not in keep:
                path.unlink()
        elif not any(path.iterdir()):
            path.rmdir()

    # Qt plugins: keep the folders in PLUGINS, and in quick only the kept modules' plugins.
    for folder in (contents / "PlugIns").iterdir():
        if folder.name not in PLUGINS:
            remove(folder)
    quick = contents / "PlugIns" / "quick"
    if quick.is_dir():
        for plugin in quick.iterdir():
            if plugin.name not in plugins:
                plugin.unlink()

    # Frameworks: keep what the app and the kept plugins load, directly or not.
    frameworks = contents / "Frameworks"
    pending = [path for path in (contents / "MacOS").iterdir() if path.is_file()]
    pending += [path for path in (contents / "PlugIns").rglob("*") if path.is_file()]
    needed = set()
    while pending:
        for name in links(pending.pop()):
            match = re.search(r"([^/]+)\.framework/", name)
            if match and match[1] not in needed and (frameworks / f"{match[1]}.framework").exists():
                needed.add(match[1])
                pending.append((frameworks / f"{match[1]}.framework" / match[1]).resolve())
    for framework in frameworks.glob("*.framework"):
        if framework.stem not in needed:
            remove(framework)

    dangling = [str(path) for path in app.rglob("*") if path.is_symlink() and not path.exists()]
    if dangling:
        sys.exit("Trimming left links to removed files: " + ", ".join(dangling))

    # Keep only the arm64 code of universal binaries.
    for path in app.rglob("*"):
        if path.is_file() and not path.is_symlink():
            image, universal = macho(path.read_bytes())
            if universal:
                path.write_bytes(image)

    print(f"Frameworks: {', '.join(sorted(needed))}")
    print(f"QML plugins: {', '.join(sorted(plugins))}")
    print(f"Trimmed {app.name} from {before / 1e6:.1f} MB to {size(app) / 1e6:.1f} MB")


if __name__ == "__main__":
    main()
