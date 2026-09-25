import Foundation

/// The diagnostics log (FRONTEND.md §6.4). The core owns the file and writes the
/// git side itself; this is how the UI adds what only it knows: the errors it
/// showed the user, and the ones it recovered from without showing anything.
enum AppLog {
    /// Start the log. Called first thing at launch, before the core is created,
    /// so a failure to create it is recorded too.
    static func start() {
        let version = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String
        do {
            _ = try initLogging(appVersion: version ?? "unknown")
        } catch {
            // Nowhere else to record it: the log is what failed.
            NSLog("failed to start the diagnostics log: \(AppModel.message(error))")
        }
    }

    static func error(_ message: String) {
        logMessage(level: .error, message: message)
    }

    static func warn(_ message: String) {
        logMessage(level: .warn, message: message)
    }
}
