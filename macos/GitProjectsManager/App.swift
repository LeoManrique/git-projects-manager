import AppKit
import SwiftUI

@main
@MainActor
struct GitProjectsManagerApp: App {
    private let model: AppModel?

    init() {
        // AppKit's stock tooltip delay is long enough that a toolbar button's
        // help text reads as missing, and inconsistent with it: once a tooltip
        // has been shown the next one appears almost instantly, so two adjacent
        // controls behave differently depending on which was hovered first.
        // The delay is in milliseconds, and registering it here applies it to
        // this app only — a value the user set globally still wins, since the
        // registration domain is the lowest-priority one.
        UserDefaults.standard.register(defaults: ["NSInitialToolTipDelay": 350])
        AppLog.start()
        do {
            model = try AppModel()
        } catch {
            AppLog.error("failed to start: \(AppModel.message(error))")
            model = nil
        }
    }

    var body: some Scene {
        WindowGroup {
            if let model {
                MainWindow()
                    .environment(model)
                    .task { await model.start() }
                    .onReceive(
                        NotificationCenter.default.publisher(
                            for: NSApplication.didBecomeActiveNotification
                        )
                    ) { _ in
                        model.appDidBecomeActive()
                    }
            } else {
                ContentUnavailableView(
                    "Failed to Start",
                    systemImage: "exclamationmark.triangle",
                    description: Text("The configuration directory could not be accessed.")
                )
            }
        }
        .defaultSize(width: 1080, height: 700)
        .commands {
            CommandGroup(after: .toolbar) {
                if let model {
                    // Same action and same title as the toolbar's ScanButton,
                    // so ⌘R never scans something other than what is on screen.
                    Button(model.scanActionTitle) {
                        Task { await model.scanSelection() }
                    }
                    .keyboardShortcut("r", modifiers: .command)
                    .disabled(model.folders.isEmpty || model.isScanningSelection)
                }
            }
        }

        Settings {
            if let model {
                SettingsView()
                    .environment(model)
            }
        }
    }
}
