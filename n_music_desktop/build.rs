use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};
use std::fs;
use std::path::Path;

/// The interface, one QML type per file.
const QML: &[&str] = &[
    "qml/Main.qml",
    "qml/Icon.qml",
    "qml/NavItem.qml",
    "qml/Sidebar.qml",
];

/// Shared values: colours, icon shapes and strings.
const QML_SINGLETONS: &[&str] = &["qml/Theme.qml", "qml/Icons.qml", "qml/Tr.qml"];

/// The Rust side of the interface.
const BRIDGES: &[&str] = &["src/bridge/app.rs", "src/bridge/translations.rs"];

const RESOURCES: &[&str] = &[
    "assets/fonts/Figtree-Regular.ttf",
    "assets/fonts/Figtree-Medium.ttf",
    "assets/fonts/Figtree-SemiBold.ttf",
    "assets/fonts/Figtree-Bold.ttf",
];

fn main() {
    write_locales();
    let qml = QML.iter().map(|&file| QmlFile::from(file)).chain(
        QML_SINGLETONS
            .iter()
            .map(|&file| QmlFile::from(file).singleton(true)),
    );
    CxxQtBuilder::new_qml_module(QmlModule::new("NMusic").qml_files(qml))
        .qt_module("Quick")
        .qrc_resources(RESOURCES)
        .files(BRIDGES)
        .build();
}

/// Bundles every `assets/lang/<code>_<name>.json` as `LOCALES`.
fn write_locales() {
    println!("cargo::rerun-if-changed=assets/lang");
    let mut files: Vec<_> = fs::read_dir("assets/lang")
        .expect("assets/lang is missing")
        .map(|entry| entry.expect("unreadable assets/lang entry").path())
        .collect();
    files.sort();
    let mut code = String::from(
        "/// Bundled languages: code, name and strings as JSON.\nconst LOCALES: &[(&str, &str, &str)] = &[\n",
    );
    for file in files {
        let stem = file.file_stem().and_then(|stem| stem.to_str()).unwrap();
        let (language, name) = stem
            .split_once('_')
            .unwrap_or_else(|| panic!("{} is not named <code>_<name>.json", file.display()));
        let path = fs::canonicalize(&file).unwrap();
        code.push_str(&format!(
            "    ({language:?}, {name:?}, include_str!({path:?})),\n"
        ));
    }
    code.push_str("];\n");
    let out = std::env::var_os("OUT_DIR").unwrap();
    fs::write(Path::new(&out).join("locales.rs"), code).unwrap();
}
