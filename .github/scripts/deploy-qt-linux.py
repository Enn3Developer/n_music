#!/usr/bin/env python3
"""Copies the Qt runtime of the desktop app into its AppDir.

Usage: deploy-qt-linux.py <AppDir>/usr <Qt prefix> <QML folder> <executable name>

The app must already be in <AppDir>/usr/bin. This adds:
- the Qt plugins it loads at runtime, in usr/plugins;
- the QML modules its QML imports, in usr/qml;
- every library those and the app link to, in usr/lib, except the ones in EXCLUDED;
- usr/bin/qt.conf, which points Qt at usr/plugins and usr/qml.
AppRun puts usr/lib on LD_LIBRARY_PATH.
"""

import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

# Plugins by folder, all of a folder's unless listed. X11 and Wayland, their OpenGL and window
# decorations, input methods, the desktop portal (dark mode, file dialogs), the image formats
# QImageReader reads covers with, and TLS.
PLUGINS = {
    "platforms": ["libqxcb.so", "libqwayland-egl.so", "libqwayland-generic.so"],
    "xcbglintegrations": None,
    "wayland-decoration-client": None,
    "wayland-graphics-integration-client": None,
    "wayland-shell-integration": None,
    "platforminputcontexts": None,
    "platformthemes": ["libqxdgdesktopportal.so"],
    "imageformats": None,
    "iconengines": None,
    "tls": None,
}

# Libraries left to the host, after AppImage's exclude list. A bundled copy breaks on newer
# hosts: the GPU driver comes from the host and needs the host's libstdc++, X11 and Wayland
# libraries; ALSA, fontconfig and D-Bus must match the host's configuration and services.
EXCLUDED = {
    # glibc
    "ld-linux-x86-64.so.2", "libanl.so.1", "libBrokenLocale.so.1", "libc.so.6", "libdl.so.2",
    "libm.so.6", "libmvec.so.1", "libpthread.so.0", "libresolv.so.2", "librt.so.1",
    "libthread_db.so.1", "libutil.so.1",
    # The compiler's runtime
    "libstdc++.so.6", "libgcc_s.so.1",
    # The GPU driver
    "libGL.so.1", "libEGL.so.1", "libGLX.so.0", "libGLdispatch.so.0", "libOpenGL.so.0",
    "libdrm.so.2", "libgbm.so.1", "libglapi.so.0", "libxcb-dri2.so.0", "libxcb-dri3.so.0",
    # X11 and Wayland
    "libX11.so.6", "libX11-xcb.so.1", "libxcb.so.1", "libwayland-client.so.0",
    "libwayland-cursor.so.0", "libwayland-egl.so.1",
    # Audio, fonts and the session bus
    "libasound.so.2", "libjack.so.0", "libpipewire-0.3.so.0", "libfontconfig.so.1",
    "libfreetype.so.6", "libharfbuzz.so.0", "libfribidi.so.0", "libthai.so.0", "libdbus-1.so.3",
    # The rest of a desktop's base
    "libglib-2.0.so.0", "libgobject-2.0.so.0", "libgio-2.0.so.0", "libgthread-2.0.so.0",
    "libz.so.1", "libexpat.so.1", "libuuid.so.1", "libICE.so.6", "libSM.so.6",
    "libgpg-error.so.0", "libcom_err.so.2", "libp11-kit.so.0", "libusb-1.0.so.0", "libgmp.so.10",
}


def excluded(name):
    return name in EXCLUDED or name.startswith("libnss_")


def output(*command, env=None):
    return subprocess.run(command, check=True, capture_output=True, text=True, env=env).stdout


class Loader:
    """Finds the libraries an ELF file links to, where the dynamic loader would."""

    def __init__(self, qt_lib):
        # The app has no RUNPATH: AppRun sets LD_LIBRARY_PATH, and here Qt's own folder stands in.
        self.env = dict(os.environ, LD_LIBRARY_PATH=str(qt_lib))
        self.found = {}

    def needs(self, path):
        """Each library `path` names, with the file it resolves to, or None."""
        if path not in self.found:
            dynamic = output("readelf", "--dynamic", "--wide", path)
            names = re.findall(r"\(NEEDED\)\s+Shared library: \[(.+?)\]", dynamic)
            resolved = {}
            for line in output("ldd", path, env=self.env).splitlines():
                match = re.match(r"\s*(\S+) => (.+?)(?: \(0x[0-9a-f]+\))?$", line)
                if match:
                    resolved[match[1]] = None if match[2] == "not found" else match[2]
            self.found[path] = {name: resolved.get(name) for name in names}
        return self.found[path]

    def closure(self, path):
        """The libraries to bundle for `path`, by name, and the ones no folder has."""
        libraries, missing, pending = {}, set(), [str(path)]
        while pending:
            for name, found in self.needs(pending.pop()).items():
                if excluded(name):
                    continue
                if found is None:
                    missing.add(name)
                elif name not in libraries:
                    libraries[name] = found
                    pending.append(found)
        return libraries, missing


def qml_modules(qt, sources):
    """The folders, under Qt's qml folder, of the modules the app's QML imports."""
    scanner = qt / "libexec" / "qmlimportscanner"
    imports = json.loads(output(str(scanner), "-rootPath", str(sources), "-importPath", str(qt / "qml")))
    root = (qt / "qml").resolve()
    for entry in imports:
        # The app's own module has no path: it is compiled into the app.
        if entry.get("type") == "module" and entry.get("path"):
            yield Path(entry["path"]).resolve().relative_to(root)


def copy_module(source, target):
    """Copies a QML module; its subfolders with a qmldir are modules of their own."""
    target.mkdir(parents=True, exist_ok=True)
    for entry in source.iterdir():
        if entry.is_dir():
            if not (entry / "qmldir").exists():
                shutil.copytree(entry, target / entry.name, symlinks=True, dirs_exist_ok=True)
        else:
            shutil.copy2(entry, target / entry.name)


def main():
    if len(sys.argv) != 5:
        sys.exit(__doc__)
    usr, qt, sources = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
    executable = usr / "bin" / sys.argv[4]
    loader = Loader(qt / "lib")
    libraries = {}

    def need(path, what):
        found, missing = loader.closure(path)
        if missing:
            sys.exit(f"{what} needs libraries this system lacks: {', '.join(sorted(missing))}")
        libraries.update(found)

    need(executable, executable.name)

    modules = sorted(set(qml_modules(qt, sources)))
    for module in modules:
        copy_module(qt / "qml" / module, usr / "qml" / module)
        for plugin in (qt / "qml" / module).glob("*.so"):
            need(plugin, f"QML module {module}")

    plugins = []
    for folder, names in PLUGINS.items():
        for plugin in sorted((qt / "plugins" / folder).glob("*.so")):
            if names is not None and plugin.name not in names:
                continue
            found, missing = loader.closure(plugin)
            if missing:
                # Optional, like a Wayland integration this system has no libraries for.
                print(f"::warning::Leaving out {folder}/{plugin.name}, which needs {', '.join(sorted(missing))}")
                continue
            libraries.update(found)
            (usr / "plugins" / folder).mkdir(parents=True, exist_ok=True)
            shutil.copy2(plugin, usr / "plugins" / folder / plugin.name)
            plugins.append(f"{folder}/{plugin.name}")

    (usr / "lib").mkdir(parents=True, exist_ok=True)
    for name, path in sorted(libraries.items()):
        shutil.copy2(os.path.realpath(path), usr / "lib" / name)

    (usr / "bin" / "qt.conf").write_text(
        "[Paths]\nPrefix = ..\nPlugins = plugins\nQmlImports = qml\nQml2Imports = qml\n"
    )
    print(f"QML modules: {', '.join(str(module) for module in modules)}")
    print(f"Plugins: {', '.join(plugins)}")
    print(f"Libraries: {', '.join(sorted(libraries))}")


if __name__ == "__main__":
    main()
