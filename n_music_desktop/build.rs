use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};
use std::fs;
use std::path::Path;

/// The interface, one QML type per file.
const QML: &[&str] = &[
    "qml/Main.qml",
    "qml/Badge.qml",
    "qml/BarSlider.qml",
    "qml/Chip.qml",
    "qml/CollectionPage.qml",
    "qml/Cover.qml",
    "qml/DashedButton.qml",
    "qml/DashedFrame.qml",
    "qml/EmptyState.qml",
    "qml/FilterChip.qml",
    "qml/FilterDrawer.qml",
    "qml/FlatButton.qml",
    "qml/GroupsPage.qml",
    "qml/Icon.qml",
    "qml/IconButton.qml",
    "qml/MenuButton.qml",
    "qml/MenuEntry.qml",
    "qml/NavItem.qml",
    "qml/PillButton.qml",
    "qml/PlaceholderPage.qml",
    "qml/PlayerBar.qml",
    "qml/PlaylistPage.qml",
    "qml/PopupMenu.qml",
    "qml/PromptDialog.qml",
    "qml/QueuePage.qml",
    "qml/QueueRow.qml",
    "qml/RejectedToast.qml",
    "qml/RuleGroup.qml",
    "qml/RuleRow.qml",
    "qml/SearchField.qml",
    "qml/SegmentedControl.qml",
    "qml/SelectBox.qml",
    "qml/SettingRow.qml",
    "qml/SettingsGroup.qml",
    "qml/SettingsPage.qml",
    "qml/Sidebar.qml",
    "qml/SortButton.qml",
    "qml/SourceKindEntry.qml",
    "qml/SourceRow.qml",
    "qml/SourcesSettings.qml",
    "qml/TextBox.qml",
    "qml/ThinScrollBar.qml",
    "qml/ToggleSwitch.qml",
    "qml/TrackColumns.qml",
    "qml/TrackHeader.qml",
    "qml/TrackMenu.qml",
    "qml/TrackRow.qml",
    "qml/TrackTable.qml",
    "qml/TracksPage.qml",
];

/// Shared values: colours, icon shapes, strings and their formatting.
const QML_SINGLETONS: &[&str] = &[
    "qml/Theme.qml",
    "qml/Icons.qml",
    "qml/Tr.qml",
    "qml/Format.qml",
    "qml/Filters.qml",
    "qml/SourceKinds.qml",
];

/// The Rust side of the interface.
const BRIDGES: &[&str] = &[
    "src/bridge/app.rs",
    "src/bridge/base.rs",
    "src/bridge/catalog.rs",
    "src/bridge/covers.rs",
    "src/bridge/groups.rs",
    "src/bridge/player.rs",
    "src/bridge/playlists.rs",
    "src/bridge/queue.rs",
    "src/bridge/scan.rs",
    "src/bridge/sources.rs",
    "src/bridge/tracks.rs",
    "src/bridge/translations.rs",
];

/// C++ the bridges call.
const CPP: &[&str] = &["cpp/covers.cpp"];

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
    CxxQtBuilder::new_qml_module(
        QmlModule::new("NMusic")
            .depend("QtQml.Models")
            .qml_files(qml),
    )
    .qt_module("Quick")
    .qrc_resources(RESOURCES)
    .files(BRIDGES)
    .cpp_files(CPP)
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
