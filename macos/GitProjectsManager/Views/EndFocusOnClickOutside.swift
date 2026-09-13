import AppKit
import SwiftUI

extension View {
    /// Drops `focus` on any mouse-down that lands outside the focused control.
    /// AppKit leaves a text field first responder when a click hits nothing
    /// focusable (empty board space, another card's text, a toolbar button),
    /// so an editor that closes on losing focus would stay open. Attach it to
    /// the field itself, so the monitor lives exactly as long as the editor.
    func endsFocusOnClickOutside(_ focus: FocusState<Bool>.Binding) -> some View {
        modifier(EndFocusOnClickOutside(focus: focus))
    }
}

private struct EndFocusOnClickOutside: ViewModifier {
    let focus: FocusState<Bool>.Binding
    /// Token for `removeMonitor`; AppKit holds the monitor itself.
    @State private var monitor: Any?

    func body(content: Content) -> some View {
        content
            .onAppear {
                // A local monitor sees every click in every window of the app
                // before it is dispatched, which no SwiftUI gesture can.
                monitor = NSEvent.addLocalMonitorForEvents(
                    matching: [.leftMouseDown, .rightMouseDown, .otherMouseDown]
                ) { event in
                    if !Self.landsInFocusedControl(event) { focus.wrappedValue = false }
                    // Returned unchanged, so the click still reaches what is
                    // under it: another card, a toolbar button, a menu.
                    return event
                }
            }
            .onDisappear {
                if let monitor { NSEvent.removeMonitor(monitor) }
                monitor = nil
            }
    }

    /// Whether the click is inside the control the key window is editing.
    /// A click in another window never is, nor one while nothing focusable
    /// is first responder.
    @MainActor
    private static func landsInFocusedControl(_ event: NSEvent) -> Bool {
        guard let window = event.window else { return false }
        if let key = NSApp.keyWindow, key !== window { return false }
        guard let responder = window.firstResponder as? NSView else { return false }
        let control = editedControl(for: responder)
        return control.bounds.contains(control.convert(event.locationInWindow, from: nil))
    }

    /// A text field is edited through the window's field editor, installed
    /// inside the field, so walk out of it to the field's own bounds.
    @MainActor
    private static func editedControl(for responder: NSView) -> NSView {
        var view = responder
        while let parent = view.superview,
              parent is NSClipView || parent is NSScrollView || parent is NSTextField {
            view = parent
        }
        return view
    }
}
