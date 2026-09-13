import SwiftUI

extension View {
    /// `.draggable` has no off switch, so a view that must not be dragged
    /// for a while (a card whose notes are being edited) leaves the modifier
    /// off instead. Flipping `enabled` rebuilds the subtree below this
    /// modifier; state that has to survive the flip belongs on the view
    /// applying it, not inside.
    @ViewBuilder
    func draggableIf<Payload: Transferable>(_ enabled: Bool, _ payload: Payload) -> some View {
        if enabled {
            draggable(payload)
        } else {
            self
        }
    }
}
