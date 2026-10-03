import AppKit
import SwiftUI

/// "Last scan: 3 minutes ago" beside the scan control, with the absolute time
/// in its tooltip (FRONTEND.md §5.1).
///
/// It re-renders only when its text changes: it sleeps until the next change,
/// and catches up at once when the app comes back to the front, in case the
/// system held the sleep back meanwhile.
struct LastScanLabel: View {
    let date: Date

    @State private var now = Date.now

    var body: some View {
        let age = RelativeAge(of: date, now: now)
        Text("Last scan: \(age.text)")
            .foregroundStyle(.secondary)
            .help(date.formatted(date: .abbreviated, time: .shortened))
            // Keyed by `now`, so every wake arms the next sleep, even one that
            // ended a moment early and left the text as it was.
            .task(id: now) {
                try? await Task.sleep(for: .seconds(age.nextChange.timeIntervalSinceNow))
                guard !Task.isCancelled else { return }
                now = .now
            }
            .onChange(of: date) { now = .now }
            .onReceive(NotificationCenter.default.publisher(for: NSApplication.didBecomeActiveNotification)) { _ in
                now = .now
            }
    }
}
