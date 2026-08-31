import SwiftUI

/// The single scan control in the toolbar (FRONTEND.md §5.1).
///
/// What it scans follows the view: the All Folders overview scans every
/// monitored folder, a folder's detail view scans that folder. One button
/// rather than two, because "scan" always means "scan what I am looking at" —
/// a separate per-view control asked the user to tell apart two buttons with
/// the same intent, and left ⌘R meaning something else again.
///
/// Everything it shows comes from `AppModel`, so the ⌘R menu command runs the
/// same action against the same state.
struct ScanButton: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        Button {
            Task { await model.scanSelection() }
        } label: {
            if model.isScanningSelection {
                Label {
                    Text("Scanning…")
                } icon: {
                    ProgressView().controlSize(.small)
                }
            } else {
                Label(model.scanActionTitle, systemImage: "arrow.clockwise")
            }
        }
        .buttonStyle(.glassProminent)
        .disabled(model.folders.isEmpty || model.isScanningSelection)
        .help(model.scanActionHelp)
    }
}
