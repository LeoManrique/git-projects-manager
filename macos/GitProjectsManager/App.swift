import AppKit
import SwiftUI

@main
@MainActor
struct GitProjectsManagerApp: App {
    private let model: AppModel?

    init() {
        model = try? AppModel()
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
                    Button("Scan All Folders") {
                        Task { await model.scanAll() }
                    }
                    .keyboardShortcut("r", modifiers: .command)
                    .disabled(model.folders.isEmpty)
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
