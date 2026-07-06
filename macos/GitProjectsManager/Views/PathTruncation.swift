import Foundation

/// Filename-first path truncation (FRONTEND.md §5.3): the directory shrinks
/// first, down to a trailing "…/" bridge — but never below a first-letter
/// "/…/" hint, so a nested path can't be mistaken for a root one. Only once
/// the hint plus the full filename can't fit does the filename itself
/// middle-truncate. Building the parts directly (instead of splitting an
/// already-truncated string) keeps directory characters from ever being
/// classified as filename or vice versa.
enum PathTruncation {
    struct Parts: Equatable {
        let dir: String
        let name: String
        var display: String { dir + name }
    }

    /// Middle-truncate: keep both ends, collapse the middle to a single "…".
    static func truncateMid(_ value: String, length: Int) -> String {
        if value.count <= length { return value }
        if length <= 0 { return "" }
        if length == 1 { return "…" }
        let preCount = (length - 1) / 2
        let pre = value.prefix(preCount)
        let post = value.suffix(length - 1 - preCount)
        return "\(pre)…\(post)"
    }

    /// Split `path` into the directory and filename shown at `length`.
    static func parts(of path: String, length: Int) -> Parts {
        let dir: String
        let name: String
        if let sep = path.lastIndex(of: "/") {
            dir = String(path[...sep])
            name = String(path[path.index(after: sep)...])
        } else {
            dir = ""
            name = path
        }
        if path.count <= length { return Parts(dir: dir, name: name) }
        if length <= 0 { return Parts(dir: "", name: "") }
        if !dir.isEmpty {
            if name.count + 3 <= length {
                let keep = length - name.count - 2
                return Parts(dir: "\(dir.prefix(keep))…/", name: name)
            }
            // A dir short enough to fit within the hint's footprint shows whole.
            let hint = dir.count <= 3 ? dir : "\(dir.prefix(1))…/"
            if length > hint.count {
                return Parts(dir: hint, name: truncateMid(name, length: length - hint.count))
            }
            // Trailing-slash paths have no filename to keep.
            if name.isEmpty { return Parts(dir: "…", name: "") }
        }
        return Parts(dir: "", name: truncateMid(name, length: length))
    }
}
