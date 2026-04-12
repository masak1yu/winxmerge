import Cocoa
import FinderSync

class FinderSyncExtension: FIFinderSync {

    let sharedDefaults = UserDefaults(suiteName: "group.io.github.masak1yu.winxmerge")

    override init() {
        super.init()
        NSLog("WinXMergeFinderSync: init() called")
        // Monitor all volumes so the context menu appears everywhere
        FIFinderSyncController.default().directoryURLs = [URL(fileURLWithPath: "/")]
        NSLog("WinXMergeFinderSync: directoryURLs set to /")
    }

    // MARK: - Context Menu

    override func menu(for menuKind: FIMenuKind) -> NSMenu? {
        NSLog("WinXMergeFinderSync: menu(for:) called, menuKind=%d", menuKind.rawValue)
        guard menuKind == .contextualMenuForItems else { return nil }

        let menu = NSMenu(title: "WinXMerge")
        let selectedItems = FIFinderSyncController.default().selectedItemURLs() ?? []

        if selectedItems.count >= 2 {
            // 2+ items selected: offer direct comparison
            let compareItem = NSMenuItem(
                title: "Compare with WinXMerge",
                action: #selector(compareSelected(_:)),
                keyEquivalent: ""
            )
            compareItem.image = NSImage(named: "winxmerge-icon")
            menu.addItem(compareItem)
        }

        if selectedItems.count == 1 {
            // 1 item selected: offer mark or compare-with-marked
            let markItem = NSMenuItem(
                title: "Mark for Compare (WinXMerge)",
                action: #selector(markForCompare(_:)),
                keyEquivalent: ""
            )
            menu.addItem(markItem)

            if let markedPath = sharedDefaults?.string(forKey: "markedFilePath"),
               FileManager.default.fileExists(atPath: markedPath) {
                let markedName = (markedPath as NSString).lastPathComponent
                let compareItem = NSMenuItem(
                    title: "Compare with \"\(markedName)\" (WinXMerge)",
                    action: #selector(compareWithMarked(_:)),
                    keyEquivalent: ""
                )
                menu.addItem(compareItem)
            }
        }

        return menu.items.isEmpty ? nil : menu
    }

    // MARK: - Actions

    @objc func compareSelected(_ sender: AnyObject?) {
        guard let selectedItems = FIFinderSyncController.default().selectedItemURLs(),
              selectedItems.count >= 2 else { return }

        // Take first 2 (or 3 for 3-way) items
        let paths = Array(selectedItems.prefix(3)).map { $0.path }
        launchWinXMerge(with: paths)
    }

    @objc func markForCompare(_ sender: AnyObject?) {
        guard let selectedItems = FIFinderSyncController.default().selectedItemURLs(),
              let firstItem = selectedItems.first else { return }

        sharedDefaults?.set(firstItem.path, forKey: "markedFilePath")
        sharedDefaults?.synchronize()
    }

    @objc func compareWithMarked(_ sender: AnyObject?) {
        guard let selectedItems = FIFinderSyncController.default().selectedItemURLs(),
              let selectedItem = selectedItems.first,
              let markedPath = sharedDefaults?.string(forKey: "markedFilePath") else { return }

        launchWinXMerge(with: [markedPath, selectedItem.path])

        // Clear the marked file after comparison
        sharedDefaults?.removeObject(forKey: "markedFilePath")
        sharedDefaults?.synchronize()
    }

    // MARK: - Launch Helper

    private func launchWinXMerge(with paths: [String]) {
        // Navigate from .appex to the main app binary:
        // .../WinXMerge.app/Contents/PlugIns/WinXMergeFinderSync.appex
        //  -> .../WinXMerge.app/Contents/MacOS/winxmerge
        let appexURL = URL(fileURLWithPath: Bundle.main.bundlePath)
        let contentsURL = appexURL
            .deletingLastPathComponent()  // PlugIns/
            .deletingLastPathComponent()  // Contents/
        let binaryPath = contentsURL
            .appendingPathComponent("MacOS/winxmerge").path

        let binaryURL = URL(fileURLWithPath: binaryPath)

        guard FileManager.default.fileExists(atPath: binaryPath) else {
            NSLog("WinXMerge binary not found at: %@", binaryPath)
            return
        }

        do {
            let process = Process()
            process.executableURL = binaryURL
            process.arguments = paths
            NSLog("WinXMergeFinderSync: launching %@ with args: %@", binaryPath, paths.joined(separator: ", "))
            try process.run()
        } catch {
            NSLog("Failed to launch WinXMerge: %@", error.localizedDescription)
        }
    }
}
