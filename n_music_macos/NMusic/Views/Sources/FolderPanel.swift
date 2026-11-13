import AppKit

/// Picks a folder to add as a source (brief §5; spec §5.5 replaced by the system's panel): an
/// `NSOpenPanel` for directories, as a sheet on the main window, with a "Display name" field
/// whose placeholder is the folder's default name (else "Optional") and, under it, the path of
/// the folder that would be added, or "This folder is already added" in red (Qt's footer). It
/// opens at the parent of the last folder chosen from the same place (Sources page or first
/// run), else Music, else home. A folder already added is refused with the same words, and the
/// panel stays open. Showing hidden folders (⌘⇧.) is remembered, as Qt's toggle.
@MainActor
enum FolderPanel {
    /// Where it was opened from: each keeps its own last folder (Qt's picker instances).
    enum Picker: Hashable {
        case sources, firstRun
    }

    /// The parent of the last folder chosen in this launch, by picker.
    private static var lastParent: [Picker: URL] = [:]
    /// The panel showing now, so a Debug hook can close it.
    static weak var shown: NSOpenPanel?

    /// `chosen(path, trimmedName)` once a folder not in `taken` was chosen.
    static func choose(_ picker: Picker, taken: Set<Locator>, chosen: @escaping (Locator, String) -> Void) {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.canCreateDirectories = false
        panel.resolvesAliases = true
        panel.showsHiddenFiles = UISettings.shared.showHiddenFolders
        panel.message = tr("choose_folder")
        panel.prompt = tr("add_this_folder")
        panel.directoryURL = startingFolder(picker)

        let field = NSTextField()
        field.placeholderString = tr("optional")
        field.setAccessibilityLabel(tr("display_name"))
        let label = NSTextField(labelWithString: tr("display_name"))
        label.font = .systemFont(ofSize: NSFont.systemFontSize, weight: .medium)
        label.textColor = .secondaryLabelColor
        let row = NSStackView(views: [label, field])
        row.orientation = .horizontal
        row.spacing = 12
        field.widthAnchor.constraint(greaterThanOrEqualToConstant: 280).isActive = true
        let path = NSTextField(labelWithString: "")
        path.font = .systemFont(ofSize: 12)
        path.lineBreakMode = .byTruncatingMiddle
        path.maximumNumberOfLines = 1
        path.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        let footer = NSStackView(views: [row, path])
        footer.orientation = .vertical
        footer.alignment = .leading
        footer.spacing = 8
        footer.edgeInsets = NSEdgeInsets(top: 12, left: 20, bottom: 12, right: 20)
        path.widthAnchor.constraint(lessThanOrEqualToConstant: 560).isActive = true
        panel.accessoryView = footer
        panel.isAccessoryViewDisclosed = true

        let delegate = FolderPanelDelegate(taken: taken, field: field, path: path)
        panel.delegate = delegate
        delegate.update(panel)
        shown = panel

        let finish = { (response: NSApplication.ModalResponse) in
            // The panel holds its delegate weakly: this keeps it until the panel closes.
            withExtendedLifetime(delegate) {}
            #if DEBUG
            NSLog("N Music folder panel closed: %d, hidden folders %d", response.rawValue, panel.showsHiddenFiles ? 1 : 0)
            #endif
            if panel.showsHiddenFiles != UISettings.shared.showHiddenFolders {
                UISettings.shared.showHiddenFolders = panel.showsHiddenFiles
            }
            guard response == .OK, let url = panel.url else { return }
            lastParent[picker] = url.deletingLastPathComponent()
            let name = field.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
            chosen(.local(Self.path(url)), name)
        }
        if let window = Windows.shared.main ?? NSApp.keyWindow {
            panel.beginSheetModal(for: window, completionHandler: finish)
        } else {
            // A modal loop from the run loop, never inside a task's main-actor job (which would
            // hold every other main-actor job until the panel closes).
            MainRunLoop.perform {
                finish(panel.runModal())
            }
        }
    }

    /// A folder's path as the library writes it: absolute, without a trailing slash.
    static func path(_ url: URL) -> String {
        var path = url.standardizedFileURL.path(percentEncoded: false)
        while path.count > 1, path.hasSuffix("/") {
            path.removeLast()
        }
        return path
    }

    private static func startingFolder(_ picker: Picker) -> URL {
        let files = FileManager.default
        if let lastParent = lastParent[picker], files.fileExists(atPath: lastParent.path(percentEncoded: false)) {
            return lastParent
        }
        let home = URL(fileURLWithPath: NSHomeDirectory(), isDirectory: true)
        let music = home.appendingPathComponent("Music", isDirectory: true)
        var isFolder: ObjCBool = false
        if files.fileExists(atPath: music.path(percentEncoded: false), isDirectory: &isFolder), isFolder.boolValue {
            return music
        }
        return home
    }
}

@MainActor
private final class FolderPanelDelegate: NSObject, NSOpenSavePanelDelegate {
    /// The folders added, their links resolved (the panel says /tmp for /private/tmp).
    let taken: Set<String>
    let field: NSTextField
    let path: NSTextField

    init(taken: Set<Locator>, field: NSTextField, path: NSTextField) {
        self.taken = Set(taken.compactMap { root in
            if case .local(let location) = root { return Self.resolved(location) }
            return nil
        })
        self.field = field
        self.path = path
    }

    /// The folder that would be added (the one selected, else the one shown): its default name
    /// as the name's placeholder, its path, or that it is added already.
    func update(_ panel: NSOpenPanel) {
        guard let url = panel.url ?? panel.directoryURL else {
            field.placeholderString = tr("optional")
            path.stringValue = ""
            return
        }
        let location = FolderPanel.path(url)
        let name = defaultSourceName(root: .local(location))
        field.placeholderString = name.isEmpty ? tr("optional") : name
        if taken.contains(Self.resolved(location)) {
            path.stringValue = tr("folder_taken")
            path.textColor = .systemRed
        } else {
            path.stringValue = location
            path.textColor = .secondaryLabelColor
        }
        #if DEBUG
        NSLog("N Music folder panel: %@ (placeholder %@)", path.stringValue, field.placeholderString ?? "")
        #endif
    }

    private static func resolved(_ location: String) -> String {
        FolderPanel.path(URL(fileURLWithPath: location, isDirectory: true).resolvingSymlinksInPath())
    }

    func panelSelectionDidChange(_ sender: Any?) {
        if let panel = sender as? NSOpenPanel { update(panel) }
    }

    func panel(_ sender: Any, didChangeToDirectoryURL url: URL?) {
        if let panel = sender as? NSOpenPanel { update(panel) }
    }

    func panel(_ sender: Any, validate url: URL) throws {
        guard taken.contains(Self.resolved(FolderPanel.path(url))) else { return }
        throw NSError(domain: "com.enn3developer.n-music", code: 1, userInfo: [
            NSLocalizedDescriptionKey: tr("folder_taken"),
        ])
    }
}
