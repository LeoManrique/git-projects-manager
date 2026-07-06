import SwiftUI

/// Single-line repo path: muted monospaced directory + emphasized repo name,
/// followed by an optional accent branch chip (FRONTEND.md §5.3). Truncates
/// directory-first (`PathTruncation`): the rendered width is measured and the
/// longest fitting display found by binary search, so the repo name stays
/// visible until the directory has shrunk to its "…/" hint.
struct PathText: View {
    let path: String
    var branch: String?
    var muted = false

    @State private var displayLength: Int?
    @State private var totalWidth: CGFloat = 0
    @State private var chipWidth: CGFloat = 0

    private static let trailingPad: CGFloat = 2
    private static let spacing: CGFloat = 6

    var body: some View {
        let parts = PathTruncation.parts(of: path, length: displayLength ?? path.count)
        let dirText = Text(parts.dir).foregroundStyle(.secondary)
        let nameText = Text(parts.name)
            .fontWeight(.medium)
            .foregroundStyle(muted ? AnyShapeStyle(.secondary) : AnyShapeStyle(.primary))
        let label = Text("\(dirText)\(nameText)")
            .font(.callout)
            .monospaced()
            .lineLimit(1)

        HStack(spacing: Self.spacing) {
            if parts.display.count < path.count {
                label.help(path)
            } else {
                label
            }
            if let branch {
                // Cap via the string, not .frame(maxWidth:) — a flexible
                // frame would grab the HStack surplus and detach the chip
                // from the path.
                Text(branch.count > 24 ? "\(branch.prefix(23))…" : branch)
                    .font(.caption)
                    .foregroundStyle(Color.accentColor)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 1)
                    .background(Color.accentColor.opacity(0.12), in: Capsule())
                    .lineLimit(1)
                    .onGeometryChange(for: CGFloat.self) { $0.size.width } action: { width in
                        chipWidth = width
                        refit()
                    }
            }
            Spacer(minLength: 0)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .onGeometryChange(for: CGFloat.self) { $0.size.width } action: { width in
            totalWidth = width
            refit()
        }
    }

    private func refit() {
        // text|chip|spacer (or text|spacer) with Self.spacing between items.
        let reserved = branch == nil ? Self.spacing : chipWidth + Self.spacing * 2
        let available = totalWidth - reserved - Self.trailingPad
        guard available > 0 else { return }
        if measure(path) <= available {
            if displayLength != nil { displayLength = nil }
            return
        }
        var lo = 1
        var hi = path.count
        var best = 1
        while lo <= hi {
            let mid = (lo + hi) / 2
            if measure(PathTruncation.parts(of: path, length: mid).display) <= available {
                best = mid
                lo = mid + 1
            } else {
                hi = mid - 1
            }
        }
        if displayLength != best { displayLength = best }
    }

    /// NSFont equivalent of `.font(.callout).monospaced()`, for measuring.
    private var measureFont: NSFont {
        .monospacedSystemFont(
            ofSize: NSFont.preferredFont(forTextStyle: .callout).pointSize,
            weight: .regular
        )
    }

    private func measure(_ text: String) -> CGFloat {
        (text as NSString).size(withAttributes: [.font: measureFont]).width
    }
}
